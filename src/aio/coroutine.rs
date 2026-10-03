use std::{
    future::Future,
    pin::Pin,
    sync::{
        Arc, Mutex, PoisonError,
        atomic::{AtomicU8, Ordering},
    },
    task::{Context, Poll, Wake, Waker},
};

use arc_swap::ArcSwapOption;
use pyo3::{
    exceptions::{PyRuntimeError, PyStopIteration},
    intern,
    prelude::*,
};
use sync_wrapper::SyncWrapper;

use super::Port;

type BoxFuture = Pin<Box<dyn Future<Output = PyResult<Py<PyAny>>> + Send>>;

/// An awaitable driving a Rust future on the asyncio event loop thread.
///
/// Each suspension hands the task a real asyncio future, so task cancellation and
/// the C task fast path work unchanged. A throw or close drops the Rust future,
/// which aborts any spawned work. PyO3's borrow flag rejects reentrant polls.
#[pyclass(module = "wreq", name = "Coroutine")]
pub struct Coroutine {
    qualname: &'static str,
    /// Polled only through `&mut self`, so it needs no lock to be shared.
    future: SyncWrapper<Option<BoxFuture>>,
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
    port: ArcSwapOption<Port>,
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
            future: SyncWrapper::new(Some(Box::pin(future))),
            waker: Waker::from(slot.clone()),
            slot,
        }
    }

    fn step(&mut self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        let future = self
            .future
            .get_mut()
            .as_mut()
            .ok_or_else(|| PyRuntimeError::new_err("cannot reuse already awaited coroutine"))?;

        self.slot.begin();
        if let Poll::Ready(result) = future.as_mut().poll(&mut Context::from_waker(&self.waker)) {
            self.finish();
            return Err(PyStopIteration::new_err((result?,)));
        }
        // A coroutine that cannot wait ends here, releasing any spawned work.
        self.slot.suspend(py).inspect_err(|_| self.finish())
    }

    fn finish(&mut self) {
        *self.future.get_mut() = None;
        self.slot.state.store(DONE, Ordering::Release);
        drop(self.slot.take_waiter());
    }
}

#[pymethods]
impl Coroutine {
    #[getter]
    fn __name__(&self) -> &'static str {
        self.qualname.rsplit('.').next().unwrap_or(self.qualname)
    }

    #[getter]
    fn __qualname__(&self) -> &'static str {
        self.qualname
    }

    fn send(&mut self, py: Python<'_>, _value: &Bound<'_, PyAny>) -> PyResult<Py<PyAny>> {
        self.step(py)
    }

    fn throw(&mut self, exc: Bound<'_, PyAny>) -> PyResult<Py<PyAny>> {
        self.finish();
        Err(PyErr::from_value(exc))
    }

    fn close(&mut self) {
        self.finish();
    }

    fn __await__(slf: Py<Self>) -> Py<Self> {
        slf
    }

    fn __next__(&mut self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        self.step(py)
    }
}

// ===== impl Slot =====

impl Slot {
    /// Start a poll; wakes from now on resume the coroutine.
    fn begin(&self) {
        drop(self.take_waiter());
        self.state.store(POLLING, Ordering::Release);
    }

    /// Suspend after a pending poll, returning the future the task should await,
    /// or `None` for a bare yield when a wake already arrived.
    fn suspend(self: &Arc<Self>, py: Python<'_>) -> PyResult<Py<PyAny>> {
        if self.state.load(Ordering::Acquire) == NOTIFIED {
            return Ok(py.None());
        }
        let (event_loop, port) = Port::current(py)?;
        let waiter = waiter(&event_loop)?.unbind();
        // Publish the port and waiter before waiting, so a wake that sees WAITING
        // always reaches the loop that runs this task.
        self.port.store(Some(port));
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

    pub(super) fn set_waiter(&self, waiter: Py<PyAny>) {
        *self.waiter.lock().unwrap_or_else(PoisonError::into_inner) = Some(waiter);
    }

    pub(super) fn take_waiter(&self) -> Option<Py<PyAny>> {
        self.waiter
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .take()
    }
}

/// Create the asyncio future a task awaits while its coroutine waits.
fn waiter<'py>(event_loop: &Bound<'py, PyAny>) -> PyResult<Bound<'py, PyAny>> {
    let py = event_loop.py();
    let future = event_loop.call_method0(intern!(py, "create_future"))?;
    future.setattr(intern!(py, "_asyncio_future_blocking"), true)?;
    Ok(future)
}

impl Wake for Slot {
    fn wake(self: Arc<Self>) {
        self.wake_by_ref();
    }

    fn wake_by_ref(self: &Arc<Self>) {
        let mut state = self.state.load(Ordering::Acquire);
        while matches!(state, POLLING | WAITING) {
            match self.state.compare_exchange_weak(
                state,
                NOTIFIED,
                Ordering::AcqRel,
                Ordering::Acquire,
            ) {
                Ok(_) => break,
                Err(actual) => state = actual,
            }
        }
        // A wake during a poll is seen by `suspend`; only a waiting task needs the loop.
        if state == WAITING
            && let Some(port) = &*self.port.load()
        {
            port.push(self.clone());
        }
    }
}
