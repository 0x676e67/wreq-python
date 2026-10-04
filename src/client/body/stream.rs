//! Request bodies streamed from Python iterators and async generators.

use std::{
    pin::Pin,
    sync::Arc,
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
use tokio::{
    runtime::Handle,
    sync::mpsc::{self, error::SendError},
    task::spawn_blocking,
};

use crate::{
    aio::{self, Coroutine},
    extractor::{Binary, Text},
    runtime,
};

type Item = PyResult<PyBytesLike>;

/// Pulls a blocked caller serves for its request body; see [`PyStream::feed`].
pub type Pulls = mpsc::UnboundedReceiver<Pull>;

/// A request body chunk: `bytes` or `bytearray` data, or a `str` sent as UTF-8.
pub enum PyBytesLike {
    Bytes(Binary),
    String(Text),
}

/// A request body read from a Python iterator or async generator, for `body=` and
/// multipart parts. A blocking request may [`feed`](Self::feed) it before sending.
pub struct PyStream(Source);

/// The source behind a [`PyStream`].
enum Source {
    Sync(SyncStream),
    Async(PyAsyncStream),
}

/// A request body from a Python iterator, never advanced on a Tokio worker.
///
/// Items arrive in pulls, each item sent on as soon as the iterator yields it. A blocking
/// request serves pulls of up to [`PULL_BATCH`](Self::PULL_BATCH) bytes on its own waiting
/// thread; without that caller, or once it returns, each item is pulled on Tokio's blocking
/// pool as the body is polled.
struct SyncStream {
    /// Shared with the pull in flight, which runs on another thread.
    iter: Arc<Py<PyAny>>,
    /// Items of the pull in flight.
    pulling: Option<Pulling>,
    /// Set once the iterator ends or raises; nothing is pulled after.
    done: bool,
    /// Sends pulls to the blocked caller; cleared once it returns.
    caller: Option<mpsc::UnboundedSender<Pull>>,
}

/// The receiving end of a pull in flight.
struct Pulling {
    rx: mpsc::UnboundedReceiver<Option<Item>>,
    /// Served by the blocked caller rather than the blocking pool.
    by_caller: bool,
    /// Whether the pull has sent an item yet.
    received: bool,
}

/// A pull of the next body items: they are sent one by one on `items`, followed by `None`
/// once the iterator ends. Dropping it unserved closes `items` empty.
pub struct Pull {
    iter: Arc<Py<PyAny>>,
    items: mpsc::UnboundedSender<Option<Item>>,
    /// Bytes read before the pull ends: a batch for the caller, one item on the pool.
    budget: usize,
    read: usize,
    /// Set once the iterator has ended or raised, so no further pull will follow.
    ended: bool,
}

/// A request body from a Python async generator, forwarded with one chunk of buffering
/// by a task on the loop that was running at extraction. Dropping it cancels that task.
struct PyAsyncStream {
    /// Chunks from [`Sender`]; `None` marks the end, as does a closed channel.
    rx: mpsc::Receiver<Option<Item>>,
    /// The forwarding task and its loop, kept until forwarding ends.
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

impl PyBytesLike {
    #[inline]
    fn len(&self) -> usize {
        match self {
            PyBytesLike::Bytes(b) => b.0.len(),
            PyBytesLike::String(s) => s.0.len(),
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

impl PyStream {
    /// Let the calling thread serve a sync iterator's pulls while it blocks on the request,
    /// starting with the first, so the request goes out before any item is read.
    pub fn feed(&mut self) -> Option<Pulls> {
        match &mut self.0 {
            Source::Sync(stream) => Some(stream.feed()),
            Source::Async(_) => None,
        }
    }
}

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
    /// Bytes a pull may read before it ends and the next one starts.
    const PULL_BATCH: usize = 256 * 1024;

    fn new(iter: Py<PyAny>) -> Self {
        SyncStream {
            iter: Arc::new(iter),
            pulling: None,
            done: false,
            caller: None,
        }
    }

    fn feed(&mut self) -> Pulls {
        let (tx, rx) = mpsc::unbounded_channel();
        self.caller = Some(tx);
        self.pulling = Some(Self::start(&self.iter, &mut self.caller));
        rx
    }

    fn poll_next(&mut self, cx: &mut Context<'_>) -> Poll<Option<Item>> {
        while !self.done {
            let pulling = self
                .pulling
                .get_or_insert_with(|| Self::start(&self.iter, &mut self.caller));
            match pulling.rx.poll_recv(cx) {
                Poll::Ready(Some(Some(item))) => {
                    pulling.received = true;
                    self.done = item.is_err();
                    return Poll::Ready(Some(item));
                }
                Poll::Ready(Some(None)) => self.done = true,
                // The pull ended; an empty one was never served.
                Poll::Ready(None) => {
                    let (received, by_caller) = (pulling.received, pulling.by_caller);
                    self.pulling = None;
                    if !received {
                        if by_caller {
                            // The caller returned first; the iterator did not advance.
                            self.caller = None;
                        } else {
                            // Python is no longer available.
                            self.done = true;
                        }
                    }
                }
                Poll::Pending => return Poll::Pending,
            }
        }
        Poll::Ready(None)
    }

    /// Send a pull to the blocked caller, or run it on the blocking pool once the caller
    /// is gone. Acquiring the interpreter must not block a Tokio worker.
    fn start(iter: &Arc<Py<PyAny>>, caller: &mut Option<mpsc::UnboundedSender<Pull>>) -> Pulling {
        let (items, rx) = mpsc::unbounded_channel();
        let mut pull = Pull {
            iter: iter.clone(),
            items,
            budget: Self::PULL_BATCH,
            read: 0,
            ended: false,
        };
        if let Some(tx) = caller {
            match tx.send(pull) {
                Ok(()) => {
                    return Pulling {
                        rx,
                        by_caller: true,
                        received: false,
                    };
                }
                Err(SendError(unsent)) => {
                    *caller = None;
                    pull = unsent;
                }
            }
        }
        // Without a waiting caller, read one item so the iterator advances only as the body
        // is polled.
        pull.budget = 1;
        spawn_blocking(move || {
            // Once Python is unavailable, stop reading without creating a PyErr that
            // could require another attachment to format.
            Python::try_attach(|py| pull.serve(py, || false));
        });
        Pulling {
            rx,
            by_caller: false,
            received: false,
        }
    }
}

// ===== impl Pull =====

impl Pull {
    /// Read the next item and send it on. Returns `false` once the pull is over: its budget
    /// is spent, the iterator ended or raised, or the body is gone.
    fn step(&mut self, py: Python<'_>) -> bool {
        // A dropped body must not advance the iterator.
        if self.items.is_closed() {
            return false;
        }
        let Some(item) = next_item(py, &self.iter) else {
            self.ended = true;
            let _ = self.items.send(None);
            return false;
        };
        // Count at least a byte per item, so empty chunks also end the pull.
        self.read += item.as_ref().map_or(0, PyBytesLike::len).max(1);
        self.ended = item.is_err();
        self.items.send(Some(item)).is_ok() && !self.ended && self.read < self.budget
    }

    /// Step until the pull is over, or until `stop` returns true before an item. Returns
    /// whether the iterator has ended, so no further pull will follow.
    pub fn serve(mut self, py: Python<'_>, stop: impl Fn() -> bool) -> bool {
        while !stop() && self.step(py) {}
        self.ended
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
