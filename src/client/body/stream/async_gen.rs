//! Request bodies from Python async generators.

use std::{
    ffi::CStr,
    pin::Pin,
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
    },
    task::{Context, Poll, ready},
};

use futures_util::{Stream, StreamExt};
use pyo3::{IntoPyObjectExt, exceptions::PyRuntimeError, intern, prelude::*, sync::PyOnceLock};
use tokio::{runtime::Handle, sync::mpsc::error::TrySendError};

use super::{Item, PyBytesLike, lock, queue};
use crate::{coroutine, runtime};

/// A request body from a Python async generator, forwarded within the upload budget by a
/// task on the loop that was running at extraction. Dropping it cancels that task.
pub(super) struct AsyncStream {
    rx: queue::Rx,
    /// Set by [`Sender::finish`], so the queue closing reads as the end of the body.
    finished: Arc<AtomicBool>,
    /// The forwarding task and its loop, until forwarding ends.
    task: Option<(Py<PyAny>, Py<PyAny>)>,
}

/// The queue end given to the forwarding coroutine: `try_send` queues a chunk within the
/// budget or returns a coroutine that waits for room, and `fail` queues the error that ends
/// the body. Closing it without `finish` fails the body.
#[pyclass(frozen)]
struct Sender {
    tx: Mutex<Option<queue::Tx>>,
    finished: Arc<AtomicBool>,
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
        let finished = Arc::new(AtomicBool::new(false));
        let sender = Sender {
            tx: Mutex::new(Some(tx)),
            finished: finished.clone(),
        };
        let coroutine = forward.bind(py).call1((generator, sender))?;
        // create_task captures the caller's contextvars on the running loop.
        let task = event_loop
            .call_method1(intern!(py, "create_task"), (&coroutine,))
            .inspect_err(|_| {
                let _ = coroutine.call_method0(intern!(py, "close"));
            })?;
        Ok(Self {
            rx,
            finished,
            task: Some((task.unbind(), event_loop.unbind())),
        })
    }
}

impl Stream for AsyncStream {
    type Item = Item;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let this = self.get_mut();
        Poll::Ready(match ready!(this.rx.poll_next_unpin(cx)) {
            Some(item) => Some(item),
            // Every sender is gone: the end after `finish`. Otherwise the forwarding task was
            // cancelled or destroyed, so the body is incomplete.
            None if this.task.take().is_some() && !this.finished.load(Ordering::Acquire) => {
                Some(Err(PyRuntimeError::new_err(
                    "async body generator stopped before it finished",
                )))
            }
            None => None,
        })
    }
}

impl Drop for AsyncStream {
    fn drop(&mut self) {
        // Senders see the body gone at once, and a send waiting for room resolves to `False`.
        self.rx.close();
        if let Some((task, event_loop)) = self.task.take() {
            // Body drop can run on Tokio: cancel from a blocking thread, preferring the current
            // runtime so a drop on a client's own runtime does not start the shared one.
            let handle = Handle::try_current().unwrap_or_else(|_| runtime::get().handle().clone());
            handle.spawn_blocking(move || {
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
                Err(TrySendError::Full(())) => tx.clone(),
                Err(TrySendError::Closed(())) => return false.into_bound_py_any(py),
            },
            None => return false.into_bound_py_any(py),
        };
        // The coroutine keeps the extracted chunk, so it is never copied twice. Budget
        // readiness is runtime-independent, so it waits on the Python loop. Its name is the
        // pending send that shutdown inspects.
        coroutine::local(py, "Sender.send", async move {
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

    /// Mark the normal end of the body. The last chunk may still be queued: it is read
    /// before the closed channel, so finishing never waits for room.
    fn finish(&self) {
        self.finished.store(true, Ordering::Release);
        // Python may retain the sender after completion, especially on PyPy.
        self.close();
    }

    /// Drop the channel end at once, so the body fails instead of waiting for more.
    fn close(&self) {
        let tx = lock(&self.tx).take();
        drop(tx);
    }
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
