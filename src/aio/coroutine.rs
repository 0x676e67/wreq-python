use std::{
    future::Future,
    mem,
    pin::Pin,
    sync::{
        Arc, Mutex, MutexGuard, PoisonError,
        atomic::{AtomicU8, Ordering},
    },
    task::{Context, Poll, Wake, Waker},
};

use pyo3::{
    PyTraverseError, PyVisit,
    exceptions::{PyRuntimeError, PyStopIteration, PyTypeError},
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
///
/// A coroutine built with [`managed`](Self::managed) also works as `async with`: entering
/// awaits it, then awaits its result's `__aenter__`, and keeps the result for `__aexit__`.
#[pyclass(module = "wreq")]
pub struct Coroutine {
    qualname: &'static str,
    /// Polled only through `&mut self`, so `get_mut` reaches it without locking.
    future: Mutex<Option<BoxFuture>>,
    slot: Arc<Slot>,
    waker: Waker,
    scope: Scope,
}

/// The `async with` state of a coroutine.
enum Scope {
    /// `async with` is unsupported.
    Unsupported,
    /// Not yet awaited; `async with` may enter it.
    Ready,
    /// Awaited, entered or exited already.
    Spent,
    /// Awaited by `async with` until the result, an async context manager, is ready.
    Entering,
    /// Awaiting the result's own `__aenter__`. A wreq coroutine runs as this one's future;
    /// any other awaitable is driven through `delegate`, its `__await__` iterator.
    Opening {
        manager: Py<PyAny>,
        delegate: Option<Py<PyAny>>,
    },
    /// The entered context manager, kept until `__aexit__`.
    Entered(Py<PyAny>),
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
            scope: Scope::Unsupported,
        }
    }

    /// Let `async with` enter the coroutine; see [`aio::managed`](super::managed).
    pub(super) fn managed(mut self) -> Self {
        self.scope = Scope::Ready;
        self
    }

    #[inline]
    fn future(&mut self) -> &mut Option<BoxFuture> {
        self.future
            .get_mut()
            .unwrap_or_else(PoisonError::into_inner)
    }

    fn step(&mut self, py: Python<'_>, sent: Option<&Bound<'_, PyAny>>) -> PyResult<Py<PyAny>> {
        if let Scope::Opening {
            delegate: Some(delegate),
            ..
        } = &self.scope
        {
            let delegate = delegate.bind(py).clone();
            let result = match sent {
                Some(value) if !value.is_none() => {
                    delegate.call_method1(intern!(py, "send"), (value,))
                }
                _ => delegate.call_method0(intern!(py, "__next__")),
            };
            return self.resume(py, result);
        }
        if let Scope::Ready = self.scope {
            self.scope = Scope::Spent;
        }

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
            return match result {
                Ok(value) => self.complete(py, value),
                Err(err) => {
                    self.abandon();
                    Err(err)
                }
            };
        }
        // A coroutine that cannot wait ends here, releasing any spawned work.
        self.slot.suspend(py).inspect_err(|_| self.abandon())
    }

    /// Finish with `value`, first entering it when awaited by `async with`.
    fn complete(&mut self, py: Python<'_>, value: Py<PyAny>) -> PyResult<Py<PyAny>> {
        match mem::replace(&mut self.scope, Scope::Spent) {
            Scope::Entering => return self.open(py, value),
            Scope::Opening { manager, .. } => self.scope = Scope::Entered(manager),
            scope => self.scope = scope,
        }
        self.finish();
        Err(PyStopIteration::new_err((value,)))
    }

    /// Await `manager.__aenter__()`, as `async with` would.
    fn open(&mut self, py: Python<'_>, manager: Py<PyAny>) -> PyResult<Py<PyAny>> {
        let awaitable = match manager.bind(py).call_method0(intern!(py, "__aenter__")) {
            Ok(awaitable) => awaitable,
            Err(err) => {
                self.finish();
                return Err(err);
            }
        };
        // `Response.__aenter__` returns a ready wreq coroutine; poll its future in place.
        if let Ok(coroutine) = awaitable.cast::<Coroutine>()
            && let Ok(mut coroutine) = coroutine.try_borrow_mut()
            && let Some(future) = coroutine.future().take()
        {
            coroutine.finish();
            *self.future() = Some(future);
            self.scope = Scope::Opening {
                manager,
                delegate: None,
            };
            return self.step(py, None);
        }
        self.finish();
        let delegate = awaitable.call_method0(intern!(py, "__await__"))?;
        self.scope = Scope::Opening {
            manager,
            delegate: Some(delegate.clone().unbind()),
        };
        let result = delegate.call_method0(intern!(py, "__next__"));
        self.resume(py, result)
    }

    /// Handle a step of the `__aenter__` delegate: pass on what it yields, and finish when
    /// it returns or raises.
    fn resume(
        &mut self,
        py: Python<'_>,
        result: PyResult<Bound<'_, PyAny>>,
    ) -> PyResult<Py<PyAny>> {
        match result {
            Ok(yielded) => Ok(yielded.unbind()),
            Err(err) if err.is_instance_of::<PyStopIteration>(py) => {
                let value = err.value(py).getattr(intern!(py, "value"))?.unbind();
                self.complete(py, value)
            }
            Err(err) => {
                self.abandon();
                Err(err)
            }
        }
    }

    /// The `__aenter__` delegate, if one is running.
    fn delegate<'py>(&self, py: Python<'py>) -> Option<Bound<'py, PyAny>> {
        match &self.scope {
            Scope::Opening {
                delegate: Some(delegate),
                ..
            } => Some(delegate.bind(py).clone()),
            _ => None,
        }
    }

    /// Finish without entering, dropping any context manager being entered.
    fn abandon(&mut self) {
        self.finish();
        if let Scope::Entering | Scope::Opening { .. } = self.scope {
            self.scope = Scope::Spent;
        }
    }

    fn finish(&mut self) {
        *self.future() = None;
        self.slot.state.store(DONE, Ordering::Release);
        // A finished coroutine must not keep its loop's port open.
        let port = self.slot.lock_port().take();
        drop(port);
        drop(self.slot.take_waiter());
    }
}

#[pymethods]
impl Coroutine {
    #[getter]
    fn __name__(&self) -> &'static str {
        self.qualname
            .rsplit_once('.')
            .map_or(self.qualname, |(_, name)| name)
    }

    #[getter]
    fn __qualname__(&self) -> &'static str {
        self.qualname
    }

    fn send(&mut self, py: Python<'_>, value: &Bound<'_, PyAny>) -> PyResult<Py<PyAny>> {
        self.step(py, Some(value))
    }

    fn throw(&mut self, py: Python<'_>, exc: Bound<'_, PyAny>) -> PyResult<Py<PyAny>> {
        if let Some(delegate) = self.delegate(py)
            && let Ok(throw) = delegate.getattr(intern!(py, "throw"))
        {
            let result = throw.call1((exc,));
            return self.resume(py, result);
        }
        self.abandon();
        Err(PyErr::from_value(exc))
    }

    fn close(&mut self, py: Python<'_>) -> PyResult<()> {
        let delegate = self.delegate(py);
        self.abandon();
        if let Some(delegate) = delegate
            && let Ok(close) = delegate.getattr(intern!(py, "close"))
        {
            close.call0()?;
        }
        Ok(())
    }

    fn __await__(slf: Py<Self>) -> Py<Self> {
        slf
    }

    fn __next__(&mut self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        self.step(py, None)
    }

    /// Enter by awaiting the coroutine itself, which also awaits the result's `__aenter__`.
    fn __aenter__(mut slf: PyRefMut<'_, Self>) -> PyResult<PyRefMut<'_, Self>> {
        let pending = slf.future().is_some();
        match slf.scope {
            Scope::Unsupported => Err(PyTypeError::new_err(
                "coroutine does not support the asynchronous context manager protocol",
            )),
            Scope::Ready if pending => {
                slf.scope = Scope::Entering;
                Ok(slf)
            }
            _ => Err(PyRuntimeError::new_err(
                "cannot reuse already awaited coroutine",
            )),
        }
    }

    /// Exit the entered result, returning its `__aexit__` awaitable.
    fn __aexit__<'py>(
        &mut self,
        py: Python<'py>,
        exc_type: Bound<'py, PyAny>,
        exc_val: Bound<'py, PyAny>,
        traceback: Bound<'py, PyAny>,
    ) -> PyResult<Bound<'py, PyAny>> {
        let value = match mem::replace(&mut self.scope, Scope::Spent) {
            Scope::Entered(value) => value,
            scope => {
                self.scope = scope;
                return Err(PyRuntimeError::new_err("coroutine was not entered"));
            }
        };
        value
            .into_bound(py)
            .call_method1(intern!(py, "__aexit__"), (exc_type, exc_val, traceback))
    }

    fn __traverse__(&self, visit: PyVisit<'_>) -> Result<(), PyTraverseError> {
        match &self.scope {
            Scope::Opening { manager, delegate } => {
                visit.call(manager)?;
                visit.call(delegate)?;
            }
            Scope::Entered(value) => visit.call(value)?,
            _ => {}
        }
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
        if !matches!(self.scope, Scope::Unsupported) {
            self.scope = Scope::Spent;
        }
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
