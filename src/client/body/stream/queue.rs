//! The queue between a chunk producer and its request body: each queued chunk holds a share
//! of the upload budget until the body takes it, and dropping the body closes the budget.
//! The producer marks the end of the body, so a producer that stops early fails it.

use std::{
    pin::Pin,
    sync::Arc,
    task::{Context, Poll, ready},
};

use futures_util::Stream;
use pyo3::{PyErr, exceptions::PyRuntimeError};
use tokio::sync::{OwnedSemaphorePermit, Semaphore, TryAcquireError, mpsc};

use super::{Item, PyBytesLike};

/// Chunk bytes an upload may queue ahead of the connection; a larger chunk queues alone.
const UPLOAD_BUDGET: usize = 256 * 1024;

/// The least budget a queued chunk holds, so small chunks queue at most 64 items.
const UPLOAD_CHARGE: usize = 4 * 1024;

/// The producer end. The end and errors skip the budget, so finishing never waits.
#[derive(Clone)]
pub(super) struct Tx {
    chan: mpsc::UnboundedSender<Queued>,
    budget: Arc<Semaphore>,
}

/// A chunk's reserved share of the budget, spent by [`Permit::send`].
pub(super) struct Permit<'a> {
    chan: &'a mpsc::UnboundedSender<Queued>,
    share: OwnedSemaphorePermit,
}

/// The body end: taking an item returns its share, and dropping it stops waiting producers.
pub(super) struct Rx {
    chan: mpsc::UnboundedReceiver<Queued>,
    budget: Arc<Semaphore>,
    /// Set once the body ended or failed; later polls end it again.
    ended: bool,
}

/// A queued chunk, the error that ends the body, or `None` for its normal end, with the
/// share a chunk holds until the body takes it.
struct Queued {
    item: Option<Item>,
    _share: Option<OwnedSemaphorePermit>,
}

/// A queue with the whole budget free.
pub(super) fn channel() -> (Tx, Rx) {
    let (tx, rx) = mpsc::unbounded_channel();
    let budget = Arc::new(Semaphore::new(UPLOAD_BUDGET));
    let tx = Tx {
        chan: tx,
        budget: budget.clone(),
    };
    let rx = Rx {
        chan: rx,
        budget,
        ended: false,
    };
    (tx, rx)
}

/// The budget a queued `chunk` holds; one larger than the budget takes all of it.
fn charge(chunk: &PyBytesLike) -> u32 {
    u32::try_from(chunk.len().clamp(UPLOAD_CHARGE, UPLOAD_BUDGET)).unwrap_or(u32::MAX)
}

/// The error for a body whose producer stopped before ending it.
pub(super) fn unfinished() -> PyErr {
    PyRuntimeError::new_err("request body stopped before it finished")
}

// ===== impl Tx =====

impl Tx {
    /// Reserve `chunk`'s share if the budget has room: `NoPermits` while it is full,
    /// `Closed` once the body is dropped.
    pub(super) fn try_reserve(&self, chunk: &PyBytesLike) -> Result<Permit<'_>, TryAcquireError> {
        self.budget
            .clone()
            .try_acquire_many_owned(charge(chunk))
            .map(|share| Permit {
                chan: &self.chan,
                share,
            })
    }

    /// Wait for room for `chunk`'s share; `None` once the body is dropped.
    pub(super) async fn reserve(&self, chunk: &PyBytesLike) -> Option<Permit<'_>> {
        let share = self
            .budget
            .clone()
            .acquire_many_owned(charge(chunk))
            .await
            .ok()?;
        Some(Permit {
            chan: &self.chan,
            share,
        })
    }

    /// Queue the normal end of the body.
    pub(super) fn finish(&self) {
        let _ = self.chan.send(Queued {
            item: None,
            _share: None,
        });
    }

    /// Queue the error that ends the body.
    pub(super) fn fail(&self, err: PyErr) {
        let _ = self.chan.send(Queued {
            item: Some(Err(err)),
            _share: None,
        });
    }

    /// Whether the body is dropped.
    pub(super) fn is_closed(&self) -> bool {
        self.chan.is_closed()
    }
}

// ===== impl Permit =====

impl Permit<'_> {
    /// Queue `chunk`, the chunk this share was reserved for; `false` once the body is dropped.
    pub(super) fn send(self, chunk: PyBytesLike) -> bool {
        self.chan
            .send(Queued {
                item: Some(Ok(chunk)),
                _share: Some(self.share),
            })
            .is_ok()
    }
}

// ===== impl Rx =====

impl Rx {
    /// Refuse further items and stop producers waiting for room: a pump stops without reading
    /// another item, and a pending send resolves to `False`.
    pub(super) fn close(&mut self) {
        self.chan.close();
        self.budget.close();
    }
}

impl Stream for Rx {
    type Item = Item;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let this = self.get_mut();
        if this.ended {
            return Poll::Ready(None);
        }
        // Taking an item returns its share of the budget. Every producer gone without the
        // end means one stopped early, so the body is incomplete.
        let item = ready!(this.chan.poll_recv(cx))
            .map_or_else(|| Some(Err(unfinished())), |queued| queued.item);
        this.ended = !matches!(item, Some(Ok(_)));
        Poll::Ready(item)
    }
}

impl Drop for Rx {
    fn drop(&mut self) {
        self.close();
    }
}
