//! Python iterators read ahead of the upload on Tokio's blocking pool.

use std::{
    pin::Pin,
    sync::{Arc, Mutex},
    task::{Context, Poll},
    time::Duration,
};

use futures_util::{Stream, StreamExt};
use pyo3::prelude::*;
use tokio::{runtime::Handle, sync::mpsc::error::TrySendError, task::spawn_blocking, time};

use super::{
    Item, PyBytesLike, lock, next_item,
    queue::{self, Permit},
};

/// An iterator read ahead by pump tasks on the blocking pool, within the upload budget. When
/// the budget stays full, a pump parks the iterator with the chunk it read and frees its
/// thread; the next item taken restarts it.
pub(super) struct Pumped {
    rx: queue::Rx,
    /// The pump while none runs: before the first poll, or once parked at a full budget.
    parked: Arc<Mutex<Option<Pump>>>,
}

/// What a pump task needs to read the iterator into the body.
struct Pump {
    iter: Py<PyAny>,
    tx: queue::Tx,
    /// A chunk read before the pump parked, queued first when it restarts.
    pending: Option<PyBytesLike>,
}

// ===== impl Pumped =====

impl Pumped {
    pub(super) fn new(iter: Py<PyAny>) -> Self {
        let (tx, rx) = queue::channel();
        let pump = Pump {
            iter,
            tx,
            pending: None,
        };
        Pumped {
            rx,
            parked: Arc::new(Mutex::new(Some(pump))),
        }
    }
}

impl Stream for Pumped {
    type Item = Item;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let this = self.get_mut();
        // Taking an item returns its share of the budget before the parked check.
        let poll = this.rx.poll_next_unpin(cx);
        // A parked pump waits for budget, which the first poll or a taken item frees. The
        // check follows the take, so a pump parking concurrently is seen here.
        let parked = lock(&this.parked).take();
        if let Some(pump) = parked {
            let parked = this.parked.clone();
            spawn_blocking(move || pump.run(&parked));
        }
        poll
    }
}

// ===== impl Pump =====

impl Pump {
    /// How long a pump waits for budget before parking to free its thread.
    const PARK_AFTER: Duration = Duration::from_millis(10);

    /// Queue chunks until the iterator ends or raises, the body is dropped, or the budget
    /// stays full past [`PARK_AFTER`](Self::PARK_AFTER).
    fn run(mut self, parked: &Mutex<Option<Pump>>) {
        let handle = Handle::current();
        // Once Python is unavailable, stop reading without creating a PyErr that could
        // require another attachment to format.
        Python::try_attach(|py| {
            while !self.tx.is_closed() {
                let chunk = match self.pending.take() {
                    Some(chunk) => chunk,
                    None => match next_item(py, &self.iter) {
                        Some(Ok(chunk)) => chunk,
                        Some(Err(err)) => {
                            self.tx.fail(err);
                            return;
                        }
                        None => return,
                    },
                };
                let permit = match self.wait_for_room(py, &handle, &chunk) {
                    Some(permit) => permit,
                    None => {
                        // Park under the lock the body takes after receiving, so either it
                        // sees the pump parked or the pump sees budget.
                        let mut slot = lock(parked);
                        match self.tx.try_reserve(&chunk) {
                            Ok(permit) => permit,
                            Err(TrySendError::Closed(())) => return,
                            Err(TrySendError::Full(())) => {
                                self.pending = Some(chunk);
                                *slot = Some(self);
                                return;
                            }
                        }
                    }
                };
                if !permit.send(chunk) {
                    return;
                }
            }
        });
    }

    /// Reserve room for `chunk`, waiting detached up to [`PARK_AFTER`](Self::PARK_AFTER) while
    /// the budget is full; `None` if it stays full or the body is dropped.
    fn wait_for_room(
        &self,
        py: Python<'_>,
        handle: &Handle,
        chunk: &PyBytesLike,
    ) -> Option<Permit<'_>> {
        match self.tx.try_reserve(chunk) {
            Ok(permit) => Some(permit),
            Err(TrySendError::Closed(())) => None,
            Err(TrySendError::Full(())) => {
                let wait = time::timeout(Self::PARK_AFTER, self.tx.reserve(chunk));
                py.detach(|| handle.block_on(wait)).ok().flatten()
            }
        }
    }
}
