//! The future an asyncio task waits on while a coroutine's Rust work is pending.

use std::sync::{
    Mutex, MutexGuard, TryLockError,
    atomic::{AtomicBool, Ordering},
};

use pyo3::{
    PyTraverseError, PyVisit,
    exceptions::{
        PyKeyboardInterrupt, PyRuntimeError, PySystemExit,
        asyncio::{CancelledError, InvalidStateError},
    },
    intern,
    prelude::*,
    sync::PyOnceLock,
    types::PyDict,
};

/// The future a task waits on while a coroutine's Rust work is pending.
///
/// It implements the protocol `asyncio.Task` follows for futures from other libraries. A
/// drain resolves it on the loop thread and runs the task's wakeup at once, where an
/// `asyncio.Future` would schedule the wakeup for the next loop iteration.
///
/// Its state is only locked on the loop thread and never while Python runs, so a lock
/// already held means a reentrant call; it fails instead of deadlocking the loop.
#[pyclass(frozen, module = "wreq")]
pub(super) struct Waiter {
    event_loop: Py<PyAny>,
    state: Mutex<State>,
    /// `_asyncio_future_blocking`: set when yielded and cleared by the task accepting it.
    blocking: AtomicBool,
}

#[derive(Default)]
struct State {
    status: Status,
    /// Usually one: the wakeup of the task awaiting the coroutine.
    callbacks: Vec<Callback>,
    cancel_message: Option<Py<PyAny>>,
}

#[derive(Clone, Copy, Default, PartialEq, Eq)]
enum Status {
    #[default]
    Pending,
    Done,
    Cancelled,
}

struct Callback {
    func: Py<PyAny>,
    context: Py<PyAny>,
}

// ===== impl Waiter =====

impl Waiter {
    /// A pending waiter on `event_loop`, marked as yielded by `await`.
    pub(super) fn new(event_loop: Bound<'_, PyAny>) -> Self {
        Waiter {
            event_loop: event_loop.unbind(),
            state: Mutex::default(),
            blocking: AtomicBool::new(true),
        }
    }

    #[inline]
    fn lock(&self) -> PyResult<MutexGuard<'_, State>> {
        match self.state.try_lock() {
            Ok(state) => Ok(state),
            Err(TryLockError::Poisoned(err)) => Ok(err.into_inner()),
            Err(TryLockError::WouldBlock) => {
                Err(PyRuntimeError::new_err("waiter state is already in use"))
            }
        }
    }

    #[inline]
    fn status(&self) -> PyResult<Status> {
        Ok(self.lock()?.status)
    }

    #[inline]
    pub(super) fn is_done(&self) -> PyResult<bool> {
        Ok(self.status()? != Status::Pending)
    }

    /// Mark the waiter done and run its callbacks now, as `Handle` would run them.
    ///
    /// A `KeyboardInterrupt` or `SystemExit` from a callback is returned after the remaining
    /// callbacks are scheduled, so it stops the loop like one raised by a handle.
    pub(super) fn resolve(slf: &Bound<'_, Self>) -> PyResult<()> {
        let callbacks = {
            let mut state = slf.get().lock()?;
            if state.status != Status::Pending {
                return Ok(());
            }
            state.status = Status::Done;
            std::mem::take(&mut state.callbacks)
        };
        let py = slf.py();
        let mut callbacks = callbacks.into_iter();
        while let Some(callback) = callbacks.next() {
            let ran = callback
                .context
                .bind(py)
                .call_method1(intern!(py, "run"), (callback.func.bind(py), slf));
            let Err(err) = ran else {
                continue;
            };
            if !err.is_instance_of::<PyKeyboardInterrupt>(py)
                && !err.is_instance_of::<PySystemExit>(py)
            {
                slf.get().report(py, &callback, err);
                continue;
            }
            for callback in callbacks {
                if let Err(err) = Self::schedule(slf, &callback) {
                    err.write_unraisable(py, Some(slf.as_any()));
                }
            }
            return Err(err);
        }
        Ok(())
    }

    /// Run `callback` with this waiter on the next loop iteration.
    fn schedule(slf: &Bound<'_, Self>, callback: &Callback) -> PyResult<()> {
        let py = slf.py();
        let kwargs = PyDict::new(py);
        kwargs.set_item(intern!(py, "context"), callback.context.bind(py))?;
        slf.get()
            .event_loop
            .bind(py)
            .call_method(
                intern!(py, "call_soon"),
                (callback.func.bind(py), slf),
                Some(&kwargs),
            )
            .map(drop)
    }

    /// Pass a callback's error to the loop's exception handler, as `Handle` does.
    fn report(&self, py: Python<'_>, callback: &Callback, err: PyErr) {
        let reported = (|| {
            let context = PyDict::new(py);
            let message = format!("Exception in callback {}", callback.func.bind(py).repr()?);
            context.set_item(intern!(py, "message"), message)?;
            context.set_item(intern!(py, "exception"), err.value(py))?;
            self.event_loop
                .bind(py)
                .call_method1(intern!(py, "call_exception_handler"), (context,))
                .map(drop)
        })();
        if let Err(err) = reported {
            err.write_unraisable(py, Some(callback.func.bind(py)));
        }
    }

    fn cancelled_error(&self, py: Python<'_>) -> PyErr {
        let message = self
            .lock()
            .map(|state| state.cancel_message.as_ref().map(|m| m.clone_ref(py)));
        match message {
            Ok(Some(message)) => CancelledError::new_err((message,)),
            Ok(None) => CancelledError::new_err(()),
            Err(err) => err,
        }
    }
}

#[pymethods]
impl Waiter {
    #[getter(_asyncio_future_blocking)]
    fn get_blocking(&self) -> bool {
        self.blocking.load(Ordering::Relaxed)
    }

    #[setter(_asyncio_future_blocking)]
    fn set_blocking(&self, value: bool) {
        self.blocking.store(value, Ordering::Relaxed);
    }

    fn get_loop(&self, py: Python<'_>) -> Py<PyAny> {
        self.event_loop.clone_ref(py)
    }

    #[pyo3(signature = (func, *, context = None))]
    fn add_done_callback(
        slf: &Bound<'_, Self>,
        func: Bound<'_, PyAny>,
        context: Option<Bound<'_, PyAny>>,
    ) -> PyResult<()> {
        static COPY_CONTEXT: PyOnceLock<Py<PyAny>> = PyOnceLock::new();
        let py = slf.py();
        let context = match context {
            Some(context) if !context.is_none() => context,
            _ => COPY_CONTEXT
                .import(py, "contextvars", "copy_context")?
                .call0()?,
        };
        let callback = Callback {
            func: func.unbind(),
            context: context.unbind(),
        };
        {
            let mut state = slf.get().lock()?;
            if state.status == Status::Pending {
                state.callbacks.push(callback);
                return Ok(());
            }
        }
        Self::schedule(slf, &callback)
    }

    /// Remove callbacks equal to `func`, comparing outside the lock as `__eq__` may run Python.
    fn remove_done_callback(&self, func: &Bound<'_, PyAny>) -> PyResult<usize> {
        let py = func.py();
        let candidates: Vec<Py<PyAny>> = self
            .lock()?
            .callbacks
            .iter()
            .map(|callback| callback.func.clone_ref(py))
            .collect();
        let mut matched = Vec::new();
        for candidate in candidates {
            if candidate.is(func) || candidate.bind(py).eq(func)? {
                matched.push(candidate);
            }
        }
        if matched.is_empty() {
            return Ok(0);
        }
        // Removed callbacks are released after the lock, as dropping them may run Python.
        let removed: Vec<Callback> = {
            let mut state = self.lock()?;
            let (removed, kept) = std::mem::take(&mut state.callbacks)
                .into_iter()
                .partition(|callback| matched.iter().any(|func| func.is(&callback.func)));
            state.callbacks = kept;
            removed
        };
        Ok(removed.len())
    }

    #[pyo3(signature = (msg = None))]
    fn cancel(slf: &Bound<'_, Self>, msg: Option<Py<PyAny>>) -> PyResult<bool> {
        let callbacks = {
            let mut state = slf.get().lock()?;
            if state.status != Status::Pending {
                return Ok(false);
            }
            state.status = Status::Cancelled;
            state.cancel_message = msg;
            std::mem::take(&mut state.callbacks)
        };
        for callback in &callbacks {
            Self::schedule(slf, callback)?;
        }
        Ok(true)
    }

    fn cancelled(&self) -> PyResult<bool> {
        Ok(self.status()? == Status::Cancelled)
    }

    fn done(&self) -> PyResult<bool> {
        self.is_done()
    }

    fn result(&self, py: Python<'_>) -> PyResult<()> {
        match self.status()? {
            Status::Pending => Err(InvalidStateError::new_err("Result is not set.")),
            Status::Cancelled => Err(self.cancelled_error(py)),
            Status::Done => Ok(()),
        }
    }

    fn exception(&self, py: Python<'_>) -> PyResult<Option<Py<PyAny>>> {
        match self.status()? {
            Status::Pending => Err(InvalidStateError::new_err("Exception is not set.")),
            Status::Cancelled => Err(self.cancelled_error(py)),
            Status::Done => Ok(None),
        }
    }

    fn __repr__(&self) -> &'static str {
        match self.status() {
            Ok(Status::Pending) => "<wreq.Waiter pending>",
            Ok(Status::Done) => "<wreq.Waiter finished>",
            Ok(Status::Cancelled) => "<wreq.Waiter cancelled>",
            Err(_) => "<wreq.Waiter>",
        }
    }

    fn __traverse__(&self, visit: PyVisit<'_>) -> Result<(), PyTraverseError> {
        visit.call(&self.event_loop)?;
        let Ok(state) = self.lock() else {
            return Ok(());
        };
        for callback in &state.callbacks {
            visit.call(&callback.func)?;
            visit.call(&callback.context)?;
        }
        visit.call(&state.cancel_message)
    }

    fn __clear__(&self) {
        let Ok(mut state) = self.lock() else {
            return;
        };
        let cleared = (
            std::mem::take(&mut state.callbacks),
            state.cancel_message.take(),
        );
        // Dropping references may run Python, so release the lock first.
        drop(state);
        drop(cleared);
    }
}
