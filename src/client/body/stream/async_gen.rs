//! Request bodies from Python async generators.

use std::{
    ffi::CStr,
    pin::Pin,
    sync::Mutex,
    task::{Context, Poll, ready},
};

use futures_util::{Stream, StreamExt};
use pyo3::{
    IntoPyObjectExt, intern,
    prelude::*,
    sync::PyOnceLock,
    types::{PyDict, PyType},
};
use tokio::sync::TryAcquireError;

use super::{Item, PyBytesLike, lock, queue};
use crate::{coroutine, runtime::Runtime};

/// A request body from a Python async generator, forwarded within the upload budget by a
/// task on the loop that was running at extraction. Dropping it cancels that task.
pub(super) struct AsyncStream {
    rx: queue::Rx,
    /// The forwarding task and its loop, until forwarding ends.
    task: Option<(Py<PyAny>, Py<PyAny>)>,
}

/// The queue end given to the forwarding coroutine: `try_send` queues a chunk within the
/// budget or returns a coroutine that waits for room, and `fail` queues the error that ends
/// the body. Closing it without `finish` fails the body.
#[pyclass(frozen)]
struct Sender {
    tx: Mutex<Option<queue::Tx>>,
}

// ===== impl AsyncStream =====

impl AsyncStream {
    /// Start forwarding on the running loop, so the body must be extracted in a coroutine.
    pub(super) fn new(generator: Bound<'_, PyAny>) -> PyResult<Self> {
        static FORWARD: PyOnceLock<Py<PyAny>> = PyOnceLock::new();
        let py = generator.py();
        let event_loop = coroutine::running_loop(py)?;
        let forward = FORWARD.get_or_try_init(py, || {
            PyModule::from_code(
                py,
                FORWARD_SOURCE,
                c"wreq/_async_stream.py",
                c"wreq._async_stream",
            )?
            .getattr("forward")
            .map(Bound::unbind)
        })?;
        let (tx, rx) = queue::channel();
        let sender = Sender {
            tx: Mutex::new(Some(tx)),
        };
        let coroutine = forward.bind(py).call1((generator, sender))?;
        let task = start(&event_loop, &coroutine).inspect_err(|_| {
            let _ = coroutine.call_method0(intern!(py, "close"));
        })?;
        Ok(Self {
            rx,
            task: Some((task.unbind(), event_loop.unbind())),
        })
    }
}

impl Stream for AsyncStream {
    type Item = Item;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let this = self.get_mut();
        let item = ready!(this.rx.poll_next_unpin(cx));
        // Forwarding is over once the body ends or fails, so a drop need not cancel it.
        if !matches!(item, Some(Ok(_))) {
            this.task = None;
        }
        Poll::Ready(item)
    }
}

impl Drop for AsyncStream {
    fn drop(&mut self) {
        // Senders see the body gone at once, and a send waiting for room resolves to `False`.
        self.rx.close();
        if let Some((task, event_loop)) = self.task.take() {
            // Body drop can run on Tokio: cancel from a blocking thread.
            Runtime::spawn_blocking(move || {
                Python::try_attach(|py| {
                    if let Ok(cancel) = task.bind(py).getattr(intern!(py, "cancel")) {
                        let _ = event_loop.call_method1(
                            py,
                            intern!(py, "call_soon_threadsafe"),
                            (cancel,),
                        );
                    }
                });
            });
        }
    }
}

// ===== impl Sender =====

#[pymethods]
impl Sender {
    /// Queue a chunk: `True` once queued within the budget, `False` once the body is dropped
    /// or the sender closed, and otherwise a coroutine that queues it once there is room.
    fn try_send<'py>(&self, item: Bound<'py, PyAny>) -> PyResult<Bound<'py, PyAny>> {
        let py = item.py();
        let chunk = item.extract::<PyBytesLike>()?;
        // Dropping a chunk runs no Python code, so a failed send may drop it under the lock.
        let tx = match &*lock(&self.tx) {
            Some(tx) => match tx.try_reserve(&chunk) {
                Ok(permit) => return permit.send(chunk).into_bound_py_any(py),
                Err(TryAcquireError::NoPermits) => tx.clone(),
                Err(TryAcquireError::Closed) => return false.into_bound_py_any(py),
            },
            None => return false.into_bound_py_any(py),
        };
        // The coroutine keeps the extracted chunk, so it is never copied twice. Budget
        // readiness is runtime-independent, so it waits on the Python loop.
        coroutine::local(py, "Sender.try_send", async move {
            let permit = tx.reserve(&chunk).await;
            Ok(permit.is_some_and(|permit| permit.send(chunk)))
        })
        .map(Bound::into_any)
    }

    /// Queue `error` as the end of the body, without waiting for room.
    fn fail(&self, error: Bound<'_, PyAny>) {
        // Released first: dropping an unsent error can run Python code.
        let tx = lock(&self.tx).clone();
        if let Some(tx) = tx {
            tx.fail(PyErr::from_value(error));
        }
    }

    /// Mark the normal end of the body, which skips the budget, so finishing never waits.
    fn finish(&self) {
        // Python may retain the sender after completion, especially on PyPy.
        let tx = lock(&self.tx).take();
        if let Some(tx) = tx {
            tx.finish();
        }
    }

    /// Drop the channel end at once, so the body fails instead of waiting for more.
    fn close(&self) {
        let tx = lock(&self.tx).take();
        drop(tx);
    }
}

/// Run `coroutine` as a task on `event_loop`, capturing the caller's contextvars.
///
/// A standard loop whose `create_task` passes extra keywords to `Task`, as on Python 3.14 and
/// later 3.13 releases, receives an eager-start hint so chunks can be queued before the
/// request is sent. Task creation always goes through the loop, including wrappers that
/// capture context or instrument tasks.
fn start<'py>(
    event_loop: &Bound<'py, PyAny>,
    coroutine: &Bound<'py, PyAny>,
) -> PyResult<Bound<'py, PyAny>> {
    let py = event_loop.py();
    let create_task = event_loop.getattr(intern!(py, "create_task"))?;
    if eager(event_loop, &create_task) {
        let kwargs = PyDict::new(py);
        kwargs.set_item(intern!(py, "eager_start"), true)?;
        return create_task.call((coroutine,), Some(&kwargs));
    }
    create_task.call1((coroutine,))
}

/// Only pass the hint where the standard loop API supports it. Older wrappers and custom
/// factories keep their usual arguments; the actual loop method still controls scheduling.
fn eager(event_loop: &Bound<'_, PyAny>, create_task: &Bound<'_, PyAny>) -> bool {
    static BASE: PyOnceLock<Py<PyType>> = PyOnceLock::new();
    let py = event_loop.py();
    if py.version_info() < (3, 12) {
        return false;
    }
    let standard = || -> PyResult<bool> {
        let base = BASE.import(py, "asyncio", "BaseEventLoop")?;
        let name = intern!(py, "create_task");
        let method = base.getattr(name)?;
        // Python 3.14 always passes extra keywords to `Task`, 3.12 never does, and 3.13 only
        // in later releases; there only the unwrapped standard method shows whether it does.
        let forwards = py.version_info() >= (3, 14)
            || (method
                .getattr(intern!(py, "__module__"))?
                .eq("asyncio.base_events")?
                && takes_keywords(&method)?);
        Ok(forwards
            && event_loop.is_instance(base)?
            && event_loop.get_type().getattr(name)?.is(&method)
            && event_loop
                .call_method0(intern!(py, "get_task_factory"))?
                .is_none()
            // Don't send a new keyword to an old-signature wrapper.
            && takes_keywords(create_task)?)
    };
    standard().unwrap_or(false)
}

/// Whether the Python function `func` accepts `**kwargs` (`inspect.CO_VARKEYWORDS`).
fn takes_keywords(func: &Bound<'_, PyAny>) -> PyResult<bool> {
    let flags = func
        .getattr(intern!(func.py(), "__code__"))?
        .getattr(intern!(func.py(), "co_flags"))?
        .extract::<u32>()?;
    Ok(flags & 0x08 != 0)
}

/// The coroutine that forwards an async generator into its [`Sender`].
const FORWARD_SOURCE: &CStr = c"import asyncio

async def forward(gen, sender):
    try:
        try:
            async for item in gen:
                # Queued at once while the upload budget has room, otherwise awaited.
                sent = sender.try_send(item)
                if not isinstance(sent, bool):
                    sent = await sent
                if not sent:
                    return
        finally:
            close = getattr(gen, 'aclose', None)
            if close is not None:
                await close()
    except asyncio.CancelledError as error:
        # On task cancellation, close the sender so the body fails as incomplete at once,
        # even while a traceback keeps this frame and its sender alive. Any other
        # CancelledError came from the generator and ends the body as its error.
        if asyncio.current_task().cancelling():
            sender.close()
        else:
            sender.fail(error)
        raise
    except BaseException as error:
        sender.fail(error)
    else:
        sender.finish()
";
