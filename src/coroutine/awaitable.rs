use std::{
    future::Future,
    sync::{
        Arc, Mutex, MutexGuard, PoisonError,
        atomic::{AtomicU8, Ordering},
    },
    task::{Context, Poll, Wake, Waker},
};

use futures_util::{FutureExt, future::BoxFuture};
use pyo3::{
    PyTraverseError, PyVisit,
    exceptions::{PyBaseException, PyRuntimeError, PyStopIteration, PyTypeError},
    intern,
    prelude::*,
    types::{PyTraceback, PyTuple, PyType},
};

use super::{Port, scope::Scope};

/// An awaitable driving a Rust future on the asyncio event loop thread.
///
/// Each suspension hands the task a real asyncio future, so task cancellation and
/// the C task fast path work unchanged. A throw it accepts, or a close, drops the Rust
/// future, which aborts any spawned work. PyO3's borrow flag rejects reentrant polls.
///
/// A coroutine built with [`managed`](super::managed) also works as `async with`, as
/// `async with await` would; the `scope` module holds that state.
#[pyclass(module = "wreq")]
pub struct Coroutine {
    qualname: &'static str,
    /// Polled only through `&mut self`, so `get_mut` reaches it without locking.
    future: Mutex<Option<BoxFuture<'static, PyResult<Py<PyAny>>>>>,
    slot: Arc<Slot>,
    waker: Waker,
    pub(super) scope: Scope,
}

/// The outcome of a step: the protocol's `StopIteration` is built only for Python.
pub(super) enum Step {
    /// Suspend, handing the task an asyncio future to wait on, or `None`.
    Yield(Py<PyAny>),
    /// Finish with this value.
    Return(Py<PyAny>),
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
            future: Mutex::new(Some(future.boxed())),
            waker: Waker::from(slot.clone()),
            slot,
            scope: Scope::Unsupported,
        }
    }

    #[inline]
    pub(super) fn future(&mut self) -> &mut Option<BoxFuture<'static, PyResult<Py<PyAny>>>> {
        self.future
            .get_mut()
            .unwrap_or_else(PoisonError::into_inner)
    }

    pub(super) fn step(
        &mut self,
        py: Python<'_>,
        sent: Option<&Bound<'_, PyAny>>,
    ) -> PyResult<Step> {
        if self.scope.is_opening() {
            return self.forward(py, sent);
        }
        self.scope.start();

        let future = self
            .future
            .get_mut()
            .unwrap_or_else(PoisonError::into_inner)
            .as_mut()
            .ok_or_else(|| PyRuntimeError::new_err("cannot reuse already awaited coroutine"))?;

        // A pending waiter means another task is suspended on this coroutine. Once
        // it is taken, wakes from now on resume this poll.
        if let Some(waiter) = self.slot.take_waiter()
            && let Err(err) = ensure_done(waiter.bind(py))
        {
            self.slot.set_waiter(waiter);
            return Err(err);
        }

        self.slot.state.store(POLLING, Ordering::Release);
        if let Poll::Ready(result) = future.as_mut().poll(&mut Context::from_waker(&self.waker)) {
            return result
                .and_then(|value| self.complete(py, value))
                .inspect_err(|_| self.abandon());
        }
        // A coroutine that cannot wait ends here, releasing any spawned work.
        self.slot
            .suspend(py)
            .map(Step::Yield)
            .inspect_err(|_| self.abandon())
    }

    pub(super) fn finish(&mut self) {
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
        self.step(py, Some(value)).and_then(Step::into_result)
    }

    // Like a generator's, this also takes the legacy `(type, value, traceback)` form, in
    // which PyPy delegates throws. The exception is built before borrowing the coroutine,
    // as its constructor may use it.
    #[pyo3(signature = (exc, value = None, traceback = None))]
    fn throw(
        slf: &Bound<'_, Self>,
        exc: Bound<'_, PyAny>,
        value: Option<Bound<'_, PyAny>>,
        traceback: Option<Bound<'_, PyAny>>,
    ) -> PyResult<Py<PyAny>> {
        let exc = thrown(exc, value, traceback)?;
        slf.try_borrow_mut()?
            .throw_step(slf.py(), exc)
            .and_then(Step::into_result)
    }

    fn close(&mut self, py: Python<'_>) -> PyResult<()> {
        self.stop(py)
    }

    fn __await__(slf: Py<Self>) -> Py<Self> {
        slf
    }

    fn __next__(&mut self, py: Python<'_>) -> PyResult<Option<Py<PyAny>>> {
        match self.step(py, None)? {
            // A `NULL` return without an error ends iteration with `None`, no `StopIteration`.
            Step::Return(value) if value.is_none(py) => Ok(None),
            step => step.into_result().map(Some),
        }
    }

    fn __traverse__(&self, visit: PyVisit<'_>) -> Result<(), PyTraverseError> {
        self.scope.traverse(&visit)?;
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
        self.scope.clear();
    }
}

// ===== impl Step =====

impl Step {
    pub(super) fn into_result(self) -> PyResult<Py<PyAny>> {
        match self {
            Step::Yield(value) => Ok(value),
            Step::Return(value) => Err(PyStopIteration::new_err((value,))),
        }
    }
}

/// The exception `throw(exc, value, traceback)` raises, built as a generator's `throw`
/// builds it.
fn thrown<'py>(
    exc: Bound<'py, PyAny>,
    value: Option<Bound<'py, PyAny>>,
    traceback: Option<Bound<'py, PyAny>>,
) -> PyResult<Bound<'py, PyBaseException>> {
    if traceback
        .as_ref()
        .is_some_and(|tb| !tb.is_exact_instance_of::<PyTraceback>())
    {
        return Err(PyTypeError::new_err(
            "throw() third argument must be a traceback object",
        ));
    }
    let ty = match exc.cast_into::<PyBaseException>() {
        Ok(_) if value.is_some() => {
            return Err(PyTypeError::new_err(
                "instance exception may not have a separate value",
            ));
        }
        Ok(exc) => {
            if traceback.is_some() {
                set_traceback(&exc, traceback.as_ref())?;
            }
            return Ok(exc);
        }
        Err(err) => match err.into_inner().cast_into::<PyType>() {
            Ok(ty) if ty.is_subclass_of::<PyBaseException>()? => ty,
            _ => {
                return Err(PyTypeError::new_err(
                    "exceptions must derive from BaseException",
                ));
            }
        },
    };
    let py = ty.py();
    let exc = match instantiate(&ty, value) {
        Ok(exc) => exc,
        // A failed constructor raises its own error, keeping that error's traceback if it has
        // one. PyPy reports a missing traceback as `None`.
        Err(err) if err.traceback(py).is_some_and(|tb| !tb.is_none()) => {
            return Ok(err.into_value(py).into_bound(py));
        }
        Err(err) => err.into_value(py).into_bound(py),
    };
    set_traceback(&exc, traceback.as_ref())?;
    Ok(exc)
}

/// `value` itself if it already is a `ty` instance, else `ty()`, `ty(*value)` for a tuple or
/// `ty(value)`. A result that is not an exception is a `TypeError`.
fn instantiate<'py>(
    ty: &Bound<'py, PyType>,
    value: Option<Bound<'py, PyAny>>,
) -> PyResult<Bound<'py, PyBaseException>> {
    let exc = match value {
        Some(value)
            if value.is_instance_of::<PyBaseException>() && value.get_type().is_subclass(ty)? =>
        {
            value
        }
        None => ty.call0()?,
        Some(value) => match value.cast_into::<PyTuple>() {
            Ok(args) => ty.call1(args)?,
            Err(err) => ty.call1((err.into_inner(),))?,
        },
    };
    match exc.cast_into::<PyBaseException>() {
        Ok(exc) => Ok(exc),
        Err(err) => Err(PyTypeError::new_err(format!(
            "calling {ty} should have returned an instance of BaseException, not {}",
            err.into_inner().get_type().name()?
        ))),
    }
}

/// Set or clear `exc`'s traceback through `BaseException`, never an override.
fn set_traceback(
    exc: &Bound<'_, PyBaseException>,
    traceback: Option<&Bound<'_, PyAny>>,
) -> PyResult<()> {
    let py = exc.py();
    py.get_type::<PyBaseException>()
        .getattr(intern!(py, "with_traceback"))?
        .call1((exc, traceback))
        .map(drop)
}

/// Fail if `waiter`, the future a task waits on for this coroutine, is still pending.
pub(super) fn ensure_done(waiter: &Bound<'_, PyAny>) -> PyResult<()> {
    if waiter
        .call_method0(intern!(waiter.py(), "done"))?
        .is_truthy()?
    {
        Ok(())
    } else {
        Err(PyRuntimeError::new_err(
            "coroutine is being awaited already",
        ))
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
        let (waiter, port) = Port::waiter(py)?;
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
