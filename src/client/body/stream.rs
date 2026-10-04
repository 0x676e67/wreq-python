//! Request bodies streamed from Python iterators and async generators.

use std::{
    mem,
    pin::Pin,
    task::{Context, Poll},
};

use bytes::Bytes;
use futures_util::Stream;
use pyo3::{
    exceptions::PyStopIteration,
    intern,
    prelude::*,
    sync::PyOnceLock,
    types::{PyIterator, PyString},
};
use tokio::{runtime::Handle, sync::mpsc, task::spawn_blocking};

use crate::{
    aio::{self, Coroutine},
    extractor::{Binary, Text},
    runtime,
};

type Item = PyResult<PyBytesLike>;

/// A request body chunk: `bytes` or `bytearray` data, or a `str` sent as UTF-8.
pub enum PyBytesLike {
    Bytes(Binary),
    String(Text),
}

/// A request body read from a Python iterator or async generator, for `body=` and
/// multipart parts.
pub struct PyStream(Source);

/// The source behind a [`PyStream`].
enum Source {
    Sync(SyncStream),
    Async(PyAsyncStream),
}

/// A request body from a Python iterator, advanced by a task on Tokio's blocking pool.
///
/// Neither a Tokio worker nor a blocked caller runs the iterator, so a slow `__next__`
/// cannot delay the response, a timeout or cancellation. The task reads an item only once
/// the previous one is taken, staying at most one item ahead of the upload.
enum SyncStream {
    Idle(Py<PyAny>),
    Pumping(mpsc::Receiver<Item>),
}

/// A request body from a Python async generator, forwarded with one chunk of buffering
/// by a task on the loop that was running at extraction. Dropping it cancels that task.
struct PyAsyncStream {
    rx: mpsc::Receiver<Option<Item>>,
    task: Option<(Py<PyAny>, Py<PyAny>)>,
}

/// The channel end given to the forwarding coroutine; awaiting `send` applies upload
/// backpressure.
#[pyclass(frozen)]
struct Sender(mpsc::Sender<Option<Item>>);

// ===== impl PyBytesLike =====

impl FromPyObject<'_, '_> for PyBytesLike {
    type Error = PyErr;

    fn extract(ob: Borrowed<PyAny>) -> PyResult<Self> {
        // Checked by type, so a str chunk does not first fail as bytes.
        if ob.is_instance_of::<PyString>() {
            ob.extract().map(PyBytesLike::String)
        } else {
            ob.extract().map(PyBytesLike::Bytes)
        }
    }
}

impl From<PyBytesLike> for Bytes {
    #[inline]
    fn from(value: PyBytesLike) -> Self {
        match value {
            PyBytesLike::Bytes(b) => b.0,
            PyBytesLike::String(s) => s.0,
        }
    }
}

// ===== impl PyStream =====

impl FromPyObject<'_, '_> for PyStream {
    type Error = PyErr;

    fn extract(ob: Borrowed<PyAny>) -> PyResult<Self> {
        // Iterators are checked first; probing `asend` would raise for each of them.
        let source = if ob.cast::<PyIterator>().is_err() && ob.hasattr(intern!(ob.py(), "asend"))? {
            Source::Async(PyAsyncStream::new(ob.to_owned())?)
        } else {
            Source::Sync(SyncStream::Idle(ob.to_owned().unbind()))
        };
        Ok(PyStream(source))
    }
}

impl Stream for PyStream {
    type Item = Item;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        match &mut self.get_mut().0 {
            Source::Sync(stream) => stream.poll_next(cx),
            Source::Async(stream) => Pin::new(stream).poll_next(cx),
        }
    }
}

// ===== impl SyncStream =====

impl SyncStream {
    fn poll_next(&mut self, cx: &mut Context<'_>) -> Poll<Option<Item>> {
        if let SyncStream::Idle(_) = self {
            let (tx, rx) = mpsc::channel(1);
            if let SyncStream::Idle(iter) = mem::replace(self, SyncStream::Pumping(rx)) {
                spawn_blocking(move || Self::pump(iter, tx));
            }
        }
        match self {
            SyncStream::Pumping(rx) => rx.poll_recv(cx),
            SyncStream::Idle(_) => Poll::Ready(None),
        }
    }

    /// Send items until the iterator ends or raises, or the body is dropped.
    fn pump(iter: Py<PyAny>, tx: mpsc::Sender<Item>) {
        let handle = Handle::current();
        // Read an item only once the body has room for it.
        while let Ok(permit) = handle.block_on(tx.reserve()) {
            // Once Python is unavailable, stop reading without creating a PyErr that
            // could require another attachment to format.
            let Some(Some(item)) = Python::try_attach(|py| next_item(py, &iter)) else {
                return;
            };
            let failed = item.is_err();
            permit.send(item);
            if failed {
                return;
            }
        }
    }
}

/// Call `__next__`, ending the stream at StopIteration.
fn next_item(py: Python<'_>, iter: &Py<PyAny>) -> Option<Item> {
    match iter.call_method0(py, intern!(py, "__next__")) {
        Ok(ob) => Some(ob.extract(py)),
        Err(err) if err.is_instance_of::<PyStopIteration>(py) => None,
        Err(err) => Some(Err(err)),
    }
}

// ===== impl PyAsyncStream =====

impl PyAsyncStream {
    /// Start forwarding on the running loop, so the body must be extracted in a coroutine.
    fn new(generator: Bound<'_, PyAny>) -> PyResult<Self> {
        static FORWARD: PyOnceLock<Py<PyAny>> = PyOnceLock::new();
        let py = generator.py();
        let event_loop = py.import("asyncio")?.call_method0("get_running_loop")?;
        let forward = FORWARD.get_or_try_init(py, || {
            PyModule::from_code(
                py,
                c"import asyncio

async def forward(gen, sender):
    try:
        try:
            async for item in gen:
                if not await sender.send(item, False):
                    return
        finally:
            close = getattr(gen, 'aclose', None)
            if close is not None:
                await close()
    except asyncio.CancelledError as error:
        # Task cancellation must not wait for space in a retained body.
        if not asyncio.current_task().cancelling():
            await sender.send(error, True)
        raise
    except BaseException as error:
        await sender.send(error, True)
    else:
        await sender.finish()
",
                c"wreq/_async_stream.py",
                c"wreq._async_stream",
            )?
            .getattr("forward")
            .map(Bound::unbind)
        })?;
        let (tx, rx) = mpsc::channel(1);
        let coroutine = forward.bind(py).call1((generator, Sender(tx)))?;
        // create_task captures the caller's contextvars on the running loop.
        let task = match event_loop.call_method1("create_task", (&coroutine,)) {
            Ok(task) => task,
            Err(err) => {
                let _ = coroutine.call_method0("close");
                return Err(err);
            }
        };
        Ok(Self {
            rx,
            task: Some((task.unbind(), event_loop.unbind())),
        })
    }
}

impl Stream for PyAsyncStream {
    type Item = Item;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let this = self.get_mut();
        match this.rx.poll_recv(cx) {
            Poll::Ready(Some(Some(item))) => Poll::Ready(Some(item)),
            Poll::Ready(Some(None)) => {
                this.rx.close();
                this.task.take();
                Poll::Ready(None)
            }
            Poll::Ready(_) => {
                this.rx.close();
                Poll::Ready(None)
            }
            Poll::Pending => Poll::Pending,
        }
    }
}

impl Drop for PyAsyncStream {
    fn drop(&mut self) {
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
    /// Queue a chunk, or `item` as the error that ends the body. Resolves to `False`
    /// once the body is dropped, which stops forwarding.
    fn send<'py>(
        &self,
        py: Python<'py>,
        item: Bound<'py, PyAny>,
        error: bool,
    ) -> PyResult<Bound<'py, Coroutine>> {
        let item = if error {
            Err(PyErr::from_value(item))
        } else {
            Ok(item.extract()?)
        };
        let tx = self.0.clone();
        // Channel readiness is runtime-independent, so this waits on the Python loop.
        aio::local(py, "Sender.send", async move {
            Ok(tx.send(Some(item)).await.is_ok())
        })
    }

    /// Mark the normal end of the body.
    fn finish<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, Coroutine>> {
        // Python may retain the sender after completion, especially on PyPy.
        let tx = self.0.clone();
        aio::local(py, "Sender.finish", async move {
            Ok(tx.send(None).await.is_ok())
        })
    }
}
