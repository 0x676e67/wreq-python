//! `async with` on request coroutines: entering awaits the request, then the result's own
//! `__aenter__`, as `async with await` would, and exiting calls the result's `__aexit__`.

use std::mem;

use pyo3::{
    PyTraverseError, PyVisit,
    exceptions::{PyBaseException, PyRuntimeError, PyStopIteration, PyTypeError},
    intern,
    prelude::*,
};

use super::awaitable::{Coroutine, Step, ensure_done};

/// Whether a result's `__aenter__` returns the result itself, so it can be skipped.
pub(super) type EntersSelfFn = fn(Python<'_>) -> bool;

/// The `async with` state of a coroutine.
pub(super) enum Scope {
    /// `async with` is unsupported.
    Unsupported,
    /// Not yet awaited; `async with` may enter it.
    Ready(EntersSelfFn),
    /// Awaited, entered or exited already.
    Spent,
    /// Awaited by `async with` until the result, an async context manager, is ready.
    Entering(EntersSelfFn),
    /// Awaiting the manager's `__aenter__` through `delegate`. Like `async with`, its
    /// bound `__aexit__` is looked up before entering.
    Opening {
        exit: Py<PyAny>,
        delegate: Delegate,
        /// The future the delegate last yielded, which a suspended task waits on.
        awaiting: Option<Py<PyAny>>,
    },
    /// The entered manager's bound `__aexit__`, kept until exit.
    Entered(Py<PyAny>),
}

/// The awaitable a context manager's `__aenter__` returned.
pub(super) enum Delegate {
    /// A wreq coroutine that is not itself entering, stepped directly.
    Native(Py<Coroutine>),
    /// The `__await__` iterator of any other awaitable.
    Foreign(Py<PyAny>),
}

// ===== impl Scope =====

impl Scope {
    #[inline]
    pub(super) fn is_opening(&self) -> bool {
        matches!(self, Scope::Opening { .. })
    }

    /// Mark a first await, after which `async with` can no longer enter.
    #[inline]
    pub(super) fn start(&mut self) {
        if let Scope::Ready(_) = self {
            *self = Scope::Spent;
        }
    }

    pub(super) fn traverse(&self, visit: &PyVisit<'_>) -> Result<(), PyTraverseError> {
        match self {
            Scope::Opening {
                exit,
                delegate,
                awaiting,
            } => {
                visit.call(exit)?;
                delegate.traverse(visit)?;
                visit.call(awaiting)
            }
            Scope::Entered(exit) => visit.call(exit),
            _ => Ok(()),
        }
    }

    /// Drop every Python reference held for `async with`.
    pub(super) fn clear(&mut self) {
        if !matches!(self, Scope::Unsupported) {
            *self = Scope::Spent;
        }
    }
}

// ===== impl Coroutine =====

impl Coroutine {
    /// Finish with `value`, first entering it when awaited by `async with`.
    pub(super) fn complete(&mut self, py: Python<'_>, value: Py<PyAny>) -> PyResult<Step> {
        match mem::replace(&mut self.scope, Scope::Spent) {
            // Its `__aenter__` would return the result itself: enter without awaiting it.
            Scope::Entering(enters_self) if enters_self(py) => {
                let exit = value.bind(py).getattr(intern!(py, "__aexit__"))?;
                self.scope = Scope::Entered(exit.unbind());
            }
            Scope::Entering(_) => return self.open(py, value),
            Scope::Opening { exit, .. } => self.scope = Scope::Entered(exit),
            scope => self.scope = scope,
        }
        self.finish();
        Ok(Step::Return(value))
    }

    /// Await the manager's `__aenter__`, as `async with` would.
    fn open(&mut self, py: Python<'_>, manager: Py<PyAny>) -> PyResult<Step> {
        self.finish();
        let manager = manager.into_bound(py);
        // Results have no instance dict, so binding on the instance matches `async with`.
        let (Ok(enter), Ok(exit)) = (
            manager.getattr(intern!(py, "__aenter__")),
            manager.getattr(intern!(py, "__aexit__")),
        ) else {
            return Err(PyTypeError::new_err(format!(
                "'{}' object does not support the asynchronous context manager protocol",
                manager.get_type().name()?
            )));
        };
        self.scope = Scope::Opening {
            exit: exit.unbind(),
            delegate: Delegate::new(enter.call0()?)?,
            awaiting: None,
        };
        self.forward(py, None)
    }

    /// Step the `__aenter__` delegate, unless a task is still suspended on it.
    pub(super) fn forward(
        &mut self,
        py: Python<'_>,
        sent: Option<&Bound<'_, PyAny>>,
    ) -> PyResult<Step> {
        let Scope::Opening {
            delegate, awaiting, ..
        } = &self.scope
        else {
            return Err(PyRuntimeError::new_err(
                "cannot reuse already awaited coroutine",
            ));
        };
        if let Some(awaiting) = awaiting {
            ensure_done(awaiting.bind(py))?;
        }
        let step = delegate.clone_ref(py).step(py, sent);
        self.resume(py, step)
    }

    /// Pass on what the delegate yields, and finish once it returns or raises.
    fn resume(&mut self, py: Python<'_>, step: PyResult<Step>) -> PyResult<Step> {
        match step {
            Ok(Step::Yield(value)) => {
                if let Scope::Opening { awaiting, .. } = &mut self.scope {
                    // A bare yield leaves no task suspended on a future.
                    let future = value
                        .bind(py)
                        .hasattr(intern!(py, "_asyncio_future_blocking"))
                        .unwrap_or(false);
                    *awaiting = future.then(|| value.clone_ref(py));
                }
                Ok(Step::Yield(value))
            }
            Ok(Step::Return(value)) => self.complete(py, value),
            Err(err) => {
                self.abandon();
                Err(err)
            }
        }
    }

    /// Raise `exc` at the await point: inside the `__aenter__` awaitable being driven, if
    /// any, otherwise ending the coroutine.
    pub(super) fn throw_step(
        &mut self,
        py: Python<'_>,
        exc: Bound<'_, PyBaseException>,
    ) -> PyResult<Step> {
        if let Scope::Opening { delegate, .. } = &self.scope {
            let step = delegate.clone_ref(py).throw(py, exc);
            return self.resume(py, step);
        }
        self.abandon();
        Err(PyErr::from_value(exc.into_any()))
    }

    /// Finish without entering, dropping any context manager being entered.
    pub(super) fn abandon(&mut self) {
        self.finish();
        if let Scope::Entering(_) | Scope::Opening { .. } = self.scope {
            self.scope = Scope::Spent;
        }
    }

    /// Finish without entering, closing any `__aenter__` awaitable being driven.
    pub(super) fn stop(&mut self, py: Python<'_>) -> PyResult<()> {
        let delegate = match &self.scope {
            Scope::Opening { delegate, .. } => Some(delegate.clone_ref(py)),
            _ => None,
        };
        self.abandon();
        delegate.map_or(Ok(()), |delegate| delegate.close(py))
    }
}

#[pymethods]
impl Coroutine {
    /// Enter by awaiting the coroutine itself, which also awaits the result's `__aenter__`.
    fn __aenter__(mut slf: PyRefMut<'_, Self>) -> PyResult<PyRefMut<'_, Self>> {
        let pending = slf.future().is_some();
        match slf.scope {
            Scope::Unsupported => Err(PyTypeError::new_err(
                "coroutine does not support the asynchronous context manager protocol",
            )),
            Scope::Ready(enters_self) if pending => {
                slf.scope = Scope::Entering(enters_self);
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
        let exit = match mem::replace(&mut self.scope, Scope::Spent) {
            Scope::Entered(exit) => exit,
            scope => {
                self.scope = scope;
                return Err(PyRuntimeError::new_err("coroutine was not entered"));
            }
        };
        exit.into_bound(py).call1((exc_type, exc_val, traceback))
    }
}

// ===== impl Delegate =====

impl Delegate {
    fn new(awaitable: Bound<'_, PyAny>) -> PyResult<Self> {
        let py = awaitable.py();
        // A plain wreq coroutine is stepped directly. One that is entering goes through
        // Python like any other awaitable, so nesting counts against the recursion limit.
        if let Ok(coroutine) = awaitable.cast::<Coroutine>()
            && coroutine
                .try_borrow()
                .is_ok_and(|coroutine| matches!(coroutine.scope, Scope::Unsupported))
        {
            return Ok(Delegate::Native(coroutine.clone().unbind()));
        }
        // Like `await`, refuse a coroutine that another task is suspended in.
        if awaitable
            .getattr(intern!(py, "cr_await"))
            .is_ok_and(|value| !value.is_none())
        {
            return Err(PyRuntimeError::new_err(
                "coroutine is being awaited already",
            ));
        }
        // A generator-based coroutine from `types.coroutine` is its own iterator.
        const CO_ITERABLE_COROUTINE: u32 = 0x100;
        let iterable = awaitable
            .getattr(intern!(py, "gi_code"))
            .and_then(|code| code.getattr(intern!(py, "co_flags")))
            .and_then(|flags| flags.extract::<u32>())
            .is_ok_and(|flags| flags & CO_ITERABLE_COROUTINE != 0);
        if iterable {
            return Ok(Delegate::Foreign(awaitable.unbind()));
        }
        let Ok(await_) = awaitable.getattr(intern!(py, "__await__")) else {
            return Err(PyTypeError::new_err(format!(
                "'async with' received an object from __aenter__ that does not implement \
                 __await__: {}",
                awaitable.get_type().name()?
            )));
        };
        let iter = await_.call0()?;
        if !iter.hasattr(intern!(py, "__next__"))? {
            return Err(PyTypeError::new_err(format!(
                "__await__() returned non-iterator of type '{}'",
                iter.get_type().name()?
            )));
        }
        Ok(Delegate::Foreign(iter.unbind()))
    }

    fn step(&self, py: Python<'_>, sent: Option<&Bound<'_, PyAny>>) -> PyResult<Step> {
        match self {
            Delegate::Native(coroutine) => coroutine.bind(py).try_borrow_mut()?.step(py, sent),
            Delegate::Foreign(iter) => {
                let iter = iter.bind(py);
                iter_step(
                    py,
                    match sent {
                        Some(value) if !value.is_none() => {
                            iter.call_method1(intern!(py, "send"), (value,))
                        }
                        _ => iter.call_method0(intern!(py, "__next__")),
                    },
                )
            }
        }
    }

    fn throw(&self, py: Python<'_>, exc: Bound<'_, PyBaseException>) -> PyResult<Step> {
        match self {
            Delegate::Native(coroutine) => coroutine.bind(py).try_borrow_mut()?.throw_step(py, exc),
            Delegate::Foreign(iter) => match iter.bind(py).getattr(intern!(py, "throw")) {
                Ok(throw) => iter_step(py, throw.call1((exc,))),
                Err(_) => Err(PyErr::from_value(exc.into_any())),
            },
        }
    }

    fn close(&self, py: Python<'_>) -> PyResult<()> {
        match self {
            Delegate::Native(coroutine) => coroutine.bind(py).try_borrow_mut()?.stop(py),
            Delegate::Foreign(iter) => match iter.bind(py).getattr(intern!(py, "close")) {
                Ok(close) => close.call0().map(drop),
                Err(_) => Ok(()),
            },
        }
    }

    fn clone_ref(&self, py: Python<'_>) -> Self {
        match self {
            Delegate::Native(coroutine) => Delegate::Native(coroutine.clone_ref(py)),
            Delegate::Foreign(iter) => Delegate::Foreign(iter.clone_ref(py)),
        }
    }

    fn traverse(&self, visit: &PyVisit<'_>) -> Result<(), PyTraverseError> {
        match self {
            Delegate::Native(coroutine) => visit.call(coroutine),
            Delegate::Foreign(iter) => visit.call(iter),
        }
    }
}

/// Convert a step of a Python iterator, which returns by raising `StopIteration`.
fn iter_step(py: Python<'_>, result: PyResult<Bound<'_, PyAny>>) -> PyResult<Step> {
    match result {
        Ok(value) => Ok(Step::Yield(value.unbind())),
        Err(err) if err.is_instance_of::<PyStopIteration>(py) => err
            .value(py)
            .getattr(intern!(py, "value"))
            .map(|value| Step::Return(value.unbind())),
        Err(err) => Err(err),
    }
}
