//! Request bodies streamed from Python iterators and async generators.

use std::{
    pin::Pin,
    sync::{Arc, Mutex, MutexGuard, PoisonError},
    task::{Context, Poll},
    time::Duration,
};

use bytes::Bytes;
use futures_util::Stream;
use pyo3::{
    exceptions::{PyRuntimeError, PyStopIteration},
    intern,
    prelude::*,
    sync::PyOnceLock,
    types::{PyIterator, PyString},
};
use tokio::{
    runtime::Handle,
    sync::mpsc::{self, error::TrySendError},
    task::spawn_blocking,
    time,
};

use crate::{
    coroutine::{self, Coroutine},
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

/// A request body from a Python iterator, read on Tokio's blocking pool.
///
/// Neither a Tokio worker nor a blocked caller runs the iterator, so a slow `__next__`
/// cannot delay the response, a timeout or cancellation. A pump task reads an item only
/// once the channel has room, staying at most one item ahead of the upload. When the
/// upload stalls, it parks the iterator and frees its thread; the next item taken restarts it.
struct SyncStream {
    rx: mpsc::Receiver<Item>,
    /// The iterator and sender while no pump runs: before the first poll, or once a pump
    /// parked at a full channel.
    parked: Arc<Mutex<Option<Pump>>>,
}

/// What a pump task needs to read the iterator into the body.
struct Pump {
    iter: Py<PyAny>,
    tx: mpsc::Sender<Item>,
}

/// A request body from a Python async generator, forwarded with one chunk of buffering
/// by a task on the loop that was running at extraction. Dropping it cancels that task.
struct PyAsyncStream {
    rx: mpsc::Receiver<Option<Item>>,
    task: Option<(Py<PyAny>, Py<PyAny>)>,
}

/// The channel end given to the forwarding coroutine; awaiting `send` applies upload
/// backpressure. Closing it without `finish` fails the body.
#[pyclass(frozen)]
struct Sender(Mutex<Option<mpsc::Sender<Option<Item>>>>);

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
            Source::Sync(SyncStream::new(ob.to_owned().unbind()))
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
    fn new(iter: Py<PyAny>) -> Self {
        let (tx, rx) = mpsc::channel(1);
        SyncStream {
            rx,
            parked: Arc::new(Mutex::new(Some(Pump { iter, tx }))),
        }
    }

    fn poll_next(&mut self, cx: &mut Context<'_>) -> Poll<Option<Item>> {
        let poll = self.rx.poll_recv(cx);
        // A parked pump waits for room, which the first poll or a taken item makes. The
        // check follows the take, so a pump parking concurrently is seen here.
        let parked = lock(&self.parked).take();
        if let Some(pump) = parked {
            let parked = self.parked.clone();
            spawn_blocking(move || pump.run(&parked));
        }
        poll
    }
}

// ===== impl Pump =====

impl Pump {
    /// How long a pump waits for room before parking to free its thread.
    const PARK_AFTER: Duration = Duration::from_millis(10);

    /// Send items until the iterator ends or raises, the body is dropped, or the channel
    /// stays full past [`PARK_AFTER`](Self::PARK_AFTER).
    fn run(self, parked: &Mutex<Option<Pump>>) {
        let handle = Handle::current();
        // Once Python is unavailable, stop reading without creating a PyErr that could
        // require another attachment to format.
        Python::try_attach(|py| {
            loop {
                let permit = match self.tx.clone().try_reserve_owned() {
                    Ok(permit) => permit,
                    Err(TrySendError::Closed(_)) => return,
                    Err(TrySendError::Full(tx)) => {
                        let wait = time::timeout(Self::PARK_AFTER, tx.reserve_owned());
                        match py.detach(|| handle.block_on(wait)) {
                            Ok(Ok(permit)) => permit,
                            Ok(Err(_)) => return,
                            Err(_) => {
                                // Park under the lock the body takes after receiving, so
                                // either it sees the pump parked or the pump sees room.
                                let mut slot = lock(parked);
                                match self.tx.clone().try_reserve_owned() {
                                    Ok(permit) => permit,
                                    Err(TrySendError::Closed(_)) => return,
                                    Err(TrySendError::Full(_)) => {
                                        *slot = Some(self);
                                        return;
                                    }
                                }
                            }
                        }
                    }
                };
                let Some(item) = next_item(py, &self.iter) else {
                    return;
                };
                let failed = item.is_err();
                permit.send(item);
                if failed {
                    return;
                }
            }
        });
    }
}

#[inline]
fn lock(parked: &Mutex<Option<Pump>>) -> MutexGuard<'_, Option<Pump>> {
    parked.lock().unwrap_or_else(PoisonError::into_inner)
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
        # Task cancellation must not wait for space in a retained body, and must not leave
        # the body waiting while a traceback keeps this frame and its sender alive.
        if asyncio.current_task().cancelling():
            sender.close()
        else:
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
        let coroutine = forward
            .bind(py)
            .call1((generator, Sender(Mutex::new(Some(tx)))))?;
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
            // Every sender is gone without `finish`: the forwarding task was cancelled or
            // destroyed, so the body is incomplete.
            Poll::Ready(None) if this.task.take().is_some() => Poll::Ready(Some(Err(
                PyRuntimeError::new_err("async body generator stopped before it finished"),
            ))),
            Poll::Ready(None) => Poll::Ready(None),
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
    /// once the body is dropped or the sender closed, which stops forwarding.
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
        let tx = self.sender();
        // Channel readiness is runtime-independent, so this waits on the Python loop.
        coroutine::local(py, "Sender.send", async move {
            Ok(match tx {
                Some(tx) => tx.send(Some(item)).await.is_ok(),
                None => false,
            })
        })
    }

    /// Mark the normal end of the body.
    fn finish<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, Coroutine>> {
        // Python may retain the sender after completion, especially on PyPy.
        let tx = self.sender();
        coroutine::local(py, "Sender.finish", async move {
            Ok(match tx {
                Some(tx) => tx.send(None).await.is_ok(),
                None => false,
            })
        })
    }

    /// Drop the channel end at once, so the body fails instead of waiting for more.
    fn close(&self) {
        let tx = self.0.lock().unwrap_or_else(PoisonError::into_inner).take();
        drop(tx);
    }
}

impl Sender {
    #[inline]
    fn sender(&self) -> Option<mpsc::Sender<Option<Item>>> {
        self.0
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .clone()
    }
}
