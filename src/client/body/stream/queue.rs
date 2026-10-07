//! The queue between a chunk producer and its request body: each queued chunk holds a share
//! of the upload budget until the body takes it, and dropping the body closes the budget.

use std::{
    pin::Pin,
    sync::Arc,
    task::{Context, Poll},
};

use futures_util::Stream;
use pyo3::PyErr;
use tokio::sync::{
    OwnedSemaphorePermit, Semaphore, TryAcquireError,
    mpsc::{self, error::TrySendError},
};

use super::{Item, PyBytesLike};

/// Chunk bytes an upload may queue ahead of the connection; a larger chunk queues alone.
const UPLOAD_BUDGET: usize = 256 * 1024;

/// The least budget a queued chunk holds, so small chunks queue at most 64 items.
const UPLOAD_CHARGE: usize = 4 * 1024;

/// The producer end. Errors skip the budget, so the error that ends a body never waits.
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
}

/// A queued item and the share it holds until the body takes it.
struct Queued {
    item: Item,
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
    (tx, Rx { chan: rx, budget })
}

/// The budget a queued `chunk` holds; one larger than the budget takes all of it.
fn charge(chunk: &PyBytesLike) -> u32 {
    u32::try_from(chunk.len().clamp(UPLOAD_CHARGE, UPLOAD_BUDGET)).unwrap_or(u32::MAX)
}

// ===== impl Tx =====

impl Tx {
    /// Reserve `chunk`'s share if the budget has room; `Closed` once the body is dropped.
    pub(super) fn try_reserve(&self, chunk: &PyBytesLike) -> Result<Permit<'_>, TrySendError<()>> {
        match self.budget.clone().try_acquire_many_owned(charge(chunk)) {
            Ok(share) => Ok(Permit {
                chan: &self.chan,
                share,
            }),
            Err(TryAcquireError::NoPermits) => Err(TrySendError::Full(())),
            Err(TryAcquireError::Closed) => Err(TrySendError::Closed(())),
        }
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

    /// Queue the error that ends the body.
    pub(super) fn fail(&self, err: PyErr) {
        let _ = self.chan.send(Queued {
            item: Err(err),
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
                item: Ok(chunk),
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

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        // Taking an item returns its share of the budget.
        self.chan
            .poll_recv(cx)
            .map(|queued| queued.map(|queued| queued.item))
    }
}

impl Drop for Rx {
    fn drop(&mut self) {
        self.close();
    }
}
