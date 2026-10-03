use std::{
    future::Future,
    pin::Pin,
    sync::{
        Arc, Mutex, MutexGuard, PoisonError,
        atomic::{AtomicU8, Ordering},
    },
    task::{Context, Poll, Wake, Waker},
};

use pyo3::{
    PyTraverseError, PyVisit,
    exceptions::{PyRuntimeError, PyStopIteration},
    intern,
    prelude::*,
};

use super::Port;

type BoxFuture = Pin<Box<dyn Future<Output = PyResult<Py<PyAny>>> + Send>>;

/// An awaitable driving a Rust future on the asyncio event loop thread.
///
/// Each suspension hands the task a real asyncio future, so task cancellation and
/// the C task fast path work unchanged. A throw or close drops the Rust future,
/// which aborts any spawned work. PyO3's borrow flag rejects reentrant polls.
#[pyclass(module = "wreq")]
pub struct Coroutine {
    qualname: &'static str,
    /// Polled only through `&mut self`, so `get_mut` reaches it without locking.
    future: Mutex<Option<BoxFuture>>,
    slot: Arc<Slot>,
    waker: Waker,
}

/// Wake state shared with Rust wakers for the coroutine's whole life.
///
/// Reusing one waker lets Tokio skip re-registering it on every poll. Wakes only
/// change the state and queue the slot; Python objects are touched on the loop thread.
#[derive(Default)]
pub(super) struct Slot {
    state: AtomicU8,
    /// The port of the loop the coroutine last waited on, set before each wait.
    port: Mutex<Option<Arc<Port>>>,
    /// The asyncio future the task awaits while the coroutine waits.
    waiter: Mutex<Option<Py<PyAny>>>,
}

const POLLING: u8 = 0;
const WAITING: u8 = 1;
const NOTIFIED: u8 = 2;
const DONE: u8 = 3;

// ===== impl Coroutine =====

impl Coroutine {
    pub(super) fn new<F>(qualname: &'static str, future: F) -> Self
    where
        F: Future<Output = PyResult<Py<PyAny>>> + Send + 'static,
    {
        let slot = Arc::new(Slot::default());
        Coroutine {
            qualname,
            future: Mutex::new(Some(Box::pin(future))),
            waker: Waker::from(slot.clone()),
            slot,
        }
    }

    fn step(&mut self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        let future = self
            .future
            .get_mut()
            .unwrap_or_else(PoisonError::into_inner)
            .as_mut()
            .ok_or_else(|| PyRuntimeError::new_err("cannot reuse already awaited coroutine"))?;

        // A pending waiter means another task is suspended on this coroutine. Once
        // it is taken, wakes from now on resume this poll.
        if let Some(waiter) = self.slot.take_waiter() {
            let done = waiter
                .bind(py)
                .call_method0(intern!(py, "done"))
                .and_then(|done| done.is_truthy());
            if !matches!(done, Ok(true)) {
                self.slot.set_waiter(waiter);
                done?;
                return Err(PyRuntimeError::new_err(
                    "coroutine is being awaited already",
                ));
            }
        }

        self.slot.state.store(POLLING, Ordering::Release);
        if let Poll::Ready(result) = future.as_mut().poll(&mut Context::from_waker(&self.waker)) {
            self.finish();
            return Err(PyStopIteration::new_err((result?,)));
        }
        // A coroutine that cannot wait ends here, releasing any spawned work.
        self.slot.suspend(py).inspect_err(|_| self.finish())
    }

    fn finish(&mut self) {
        *self
            .future
            .get_mut()
            .unwrap_or_else(PoisonError::into_inner) = None;
        self.slot.state.store(DONE, Ordering::Release);
        // A finished coroutine must not keep its loop's port open.
        let port = self.slot.lock_port().take();
        drop(port);
        drop(self.slot.take_waiter());
    }
}

#[pymethods]
impl Coroutine {
    #[inline]
    #[getter]
    fn __name__(&self) -> &'static str {
        self.qualname
            .rsplit_once('.')
            .map_or(self.qualname, |(_, name)| name)
    }

    #[inline]
    #[getter]
    fn __qualname__(&self) -> &'static str {
        self.qualname
    }

    #[inline]
    fn send(&mut self, py: Python<'_>, _value: &Bound<'_, PyAny>) -> PyResult<Py<PyAny>> {
        self.step(py)
    }

    #[inline]
    fn throw(&mut self, exc: Bound<'_, PyAny>) -> PyResult<Py<PyAny>> {
        self.finish();
        Err(PyErr::from_value(exc))
    }

    #[inline]
    fn close(&mut self) {
        self.finish();
    }

    #[inline]
    fn __await__(slf: Py<Self>) -> Py<Self> {
        slf
    }

    #[inline]
    fn __next__(&mut self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        self.step(py)
    }

    fn __traverse__(&self, visit: PyVisit<'_>) -> Result<(), PyTraverseError> {
        // Pending Rust work owns the waiter until its loop closes the port; only then
        // can a cycle through the waiter be collected without aborting live requests.
        let closed = self
            .slot
            .port
            .try_lock()
            .is_ok_and(|port| port.as_ref().is_some_and(|port| !port.is_open()));
        if closed && let Ok(waiter) = self.slot.waiter.try_lock() {
            visit.call(&*waiter)?;
        }
        Ok(())
    }

    fn __clear__(&mut self) {
        self.finish();
    }
}

// ===== impl Slot =====

impl Slot {
    /// Suspend after a pending poll, returning the future the task should await,
    /// or `None` for a bare yield when a wake already arrived.
    fn suspend(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        if self.state.load(Ordering::Acquire) == NOTIFIED {
            return Ok(py.None());
        }
        let (event_loop, port) = Port::current(py)?;
        let waiter = event_loop.call_method0(intern!(py, "create_future"))?;
        waiter.setattr(intern!(py, "_asyncio_future_blocking"), true)?;
        let waiter = waiter.unbind();
        // Publish the port and waiter before waiting, so a wake that sees WAITING
        // always reaches the loop that runs this task.
        *self.lock_port() = Some(port);
        self.set_waiter(waiter.clone_ref(py));
        match self
            .state
            .compare_exchange(POLLING, WAITING, Ordering::AcqRel, Ordering::Acquire)
        {
            Ok(_) => Ok(waiter),
            // Woken after the poll but before waiting: resume on the next loop turn.
            Err(_) => {
                drop(self.take_waiter());
                Ok(py.None())
            }
        }
    }

    #[inline]
    fn lock_port(&self) -> MutexGuard<'_, Option<Arc<Port>>> {
        self.port.lock().unwrap_or_else(PoisonError::into_inner)
    }

    #[inline]
    pub(super) fn set_waiter(&self, waiter: Py<PyAny>) {
        *self.waiter.lock().unwrap_or_else(PoisonError::into_inner) = Some(waiter);
    }

    #[inline]
    pub(super) fn take_waiter(&self) -> Option<Py<PyAny>> {
        self.waiter
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .take()
    }
}

impl Wake for Slot {
    #[inline]
    fn wake(self: Arc<Self>) {
        self.wake_by_ref();
    }

    fn wake_by_ref(self: &Arc<Self>) {
        let prev = self
            .state
            .try_update(Ordering::AcqRel, Ordering::Acquire, |state| {
                matches!(state, POLLING | WAITING).then_some(NOTIFIED)
            });
        // A wake during a poll is seen by `suspend`; only a waiting task needs the loop.
        if prev == Ok(WAITING) {
            // Release the lock before pushing, which may attach to Python.
            let port = self.lock_port().clone();
            if let Some(port) = port {
                port.push(self.clone());
            }
        }
    }
}
