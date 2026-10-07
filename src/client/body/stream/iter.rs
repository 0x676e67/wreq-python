//! Request bodies from Python iterators.

use std::{
    mem,
    pin::Pin,
    task::{Context, Poll, ready},
    thread::{self, ThreadId},
};

use futures_util::{Stream, StreamExt};
use pyo3::{
    exceptions::{PyException, PyKeyboardInterrupt},
    prelude::*,
};

use super::{Item, next_item, pump::Pumped, queue};
use crate::runtime::Runtime;

/// A request body from a Python iterator.
///
/// Worker runtimes read the iterator on Tokio's blocking pool ([`Pumped`]): neither a worker
/// nor a blocked caller runs it, so a slow `__next__` cannot delay the response, a timeout or
/// cancellation. A current-thread runtime reads it on whichever thread drives it
/// ([`Inline`]), which is already blocked in a call on that runtime, though not necessarily
/// this request's.
pub(super) enum SyncStream {
    /// Not polled yet; the first poll picks where the iterator is read.
    Idle(Inline),
    Inline(Inline),
    Pumped(Pumped),
    /// The inline iterator ended or raised.
    Done,
}

/// An iterator read on the thread driving a current-thread runtime, between its IO polls.
pub(super) struct Inline {
    iter: Py<PyAny>,
    /// The thread sending the request, whose call an interrupt from the iterator may end.
    owner: ThreadId,
    /// Whether a poll has returned to the connection since the last item.
    flushed: bool,
}

// ===== impl SyncStream =====

impl SyncStream {
    /// An iterator the calling thread passes to a request.
    pub(super) fn new(iter: Py<PyAny>) -> Self {
        SyncStream::Idle(Inline {
            iter,
            owner: thread::current().id(),
            flushed: false,
        })
    }

    /// Hand an unread iterator to a request the calling thread sends.
    pub(super) fn claim(&mut self) {
        if let SyncStream::Idle(inline) = self {
            inline.owner = thread::current().id();
        }
    }
}

impl Stream for SyncStream {
    type Item = Item;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let this = self.get_mut();
        if let SyncStream::Idle(_) = this
            && let SyncStream::Idle(inline) = mem::replace(this, SyncStream::Done)
        {
            *this = if Runtime::driving() {
                SyncStream::Inline(inline)
            } else {
                SyncStream::Pumped(Pumped::new(inline.iter))
            };
        }
        match this {
            SyncStream::Inline(inline) => {
                let poll = inline.poll_next_unpin(cx);
                // Release the iterator once it ends or raises.
                if let Poll::Ready(None | Some(Err(_))) = poll {
                    *this = SyncStream::Done;
                }
                poll
            }
            SyncStream::Pumped(pumped) => pumped.poll_next_unpin(cx),
            SyncStream::Idle(_) | SyncStream::Done => Poll::Ready(None),
        }
    }
}

// ===== impl Inline =====

impl Inline {
    /// Return to the connection once before each read, so the request head and the chunks
    /// already yielded go out before `__next__` can block. The deferred wake lets the runtime
    /// poll its IO and the caller first.
    fn poll_flush(&mut self, cx: &mut Context<'_>) -> Poll<()> {
        if mem::take(&mut self.flushed) {
            return Poll::Ready(());
        }
        self.flushed = true;
        Runtime::defer_wake(cx);
        Poll::Pending
    }
}

impl Stream for Inline {
    type Item = Item;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let this = self.get_mut();
        ready!(this.poll_flush(cx));
        // Like a pump, fail the body once Python is unavailable instead of ending it short.
        Python::try_attach(|py| {
            let item = next_item(py, &this.iter);
            // A KeyboardInterrupt or SystemExit reaches the caller as itself: the owner's call,
            // or for Ctrl+C the main thread signals reach.
            if let Some(Err(err)) = &item
                && !err.is_instance_of::<PyException>(py)
                && (this.owner == thread::current().id()
                    || err.is_instance_of::<PyKeyboardInterrupt>(py))
            {
                Runtime::interrupt(err.clone_ref(py));
            }
            item
        })
        .unwrap_or_else(|| Some(Err(queue::unfinished())))
        .into()
    }
}
