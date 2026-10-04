use std::{
    collections::VecDeque,
    future::Future,
    panic::AssertUnwindSafe,
    pin::{Pin, pin},
    sync::{
        Arc, Mutex, MutexGuard, PoisonError,
        atomic::{AtomicUsize, Ordering},
    },
    task::{Context, Poll},
};

use bytes::Bytes;
use futures_util::{FutureExt, Stream, future::poll_fn};
use http_body_util::BodyExt;
use pyo3::{
    exceptions::{PyRuntimeError, PyStopIteration},
    intern,
    prelude::*,
    sync::PyOnceLock,
    types::{PyIterator, PyString},
};
use tokio::{
    runtime::Handle,
    sync::{
        Notify,
        mpsc::{self, error::TryRecvError},
        oneshot,
    },
    task::JoinHandle,
};
use tokio_util::task::AbortOnDropHandle;

use crate::{
    aio::{self, Coroutine},
    buffer::PyBuffer,
    error::Error,
    extractor::{BytesInput, StrInput},
    header::HeaderMap,
    runtime::Runtime,
};

type Item = PyResult<PyBytesLike>;

/// Python stream source.
enum PyStreamSource {
    Sync(SyncStream),
    Async(PyAsyncStream),
}

/// A bytes-like object that can be extracted from Python.
pub enum PyBytesLike {
    Bytes(BytesInput),
    String(StrInput),
}

/// A response frame exposed as a read-only memoryview or a header map.
#[derive(IntoPyObject)]
pub enum Frame {
    Bytes(PyBuffer),
    Trailers(HeaderMap),
}

/// A Python stream wrapper.
pub struct PyStream(PyStreamSource);

/// Pulls a Python iterator without blocking Tokio workers: on a blocking thread, or on
/// a blocking caller that read it ahead and serves later pulls while it waits.
struct SyncStream {
    iter: Arc<Py<PyAny>>,
    /// Items read ahead by the caller; `done` once the iterator has ended.
    ahead: VecDeque<Item>,
    done: bool,
    /// The blocked caller, until it returns and later pulls fall back to a blocking thread.
    caller: Option<mpsc::UnboundedSender<Pull>>,
    pending: Option<Pulling>,
}

enum Pulling {
    Caller(oneshot::Receiver<Batch>),
    Pool(JoinHandle<Option<Item>>),
}

/// A pull of the next body items, served by the caller blocked on the request.
pub struct Pull {
    iter: Arc<Py<PyAny>>,
    reply: oneshot::Sender<Batch>,
}

/// Items pulled while holding the GIL once; `done` if they end the iterator.
struct Batch {
    items: Vec<Item>,
    done: bool,
}

/// Adapts a Python async generator into a byte stream with bounded buffering.
/// Dropping the stream cancels its producer on the Python event loop.
struct PyAsyncStream {
    rx: mpsc::Receiver<Option<PyResult<PyBytesLike>>>,
    task: Option<(Py<PyAny>, Py<PyAny>)>,
}

#[pyclass(frozen)]
struct Sender(mpsc::Sender<Option<PyResult<PyBytesLike>>>);

/// A response stream yielding read-only memoryviews and any trailing headers.
#[pyclass(subclass, frozen, skip_from_py_object)]
pub struct Streamer(Arc<Reader>, Runtime);

/// Frames read ahead by a Tokio task, which starts on the first read so a read
/// timeout does not start before iteration. Readers poll the buffer from Python
/// and wait on `arrived`, which the task signals once per burst of frames.
struct Reader {
    state: Mutex<State>,
    arrived: Arc<Notify>,
    /// Buffered frames returned since `__anext__` last yielded to the event loop.
    since_yield: AtomicUsize,
}

enum State {
    Idle(Box<wreq::Response>),
    Reading {
        rx: mpsc::Receiver<PyResult<Frame>>,
        _task: AbortOnDropHandle<()>,
    },
    Closed,
}

/// Wakes readers however the read-ahead task ends, including when aborted.
struct NotifyOnDrop(Arc<Notify>);

// ===== impl Streamer =====

impl Streamer {
    /// Frames buffered ahead of Python. A frame holds at most one transport read, up to
    /// 408 KiB on HTTP/1 by default, so the 8 queued frames plus the one being sent stay
    /// under about 3.6 MiB.
    const READ_AHEAD: usize = 8;

    /// Buffered frames returned before `__anext__` yields to the event loop once.
    const YIELD_EVERY: usize = 8;

    /// Create a new [`Streamer`] instance.
    #[inline]
    pub fn new(resp: wreq::Response, runtime: Runtime) -> Streamer {
        let reader = Reader {
            state: Mutex::new(State::Idle(Box::new(resp))),
            arrived: Arc::new(Notify::new()),
            since_yield: AtomicUsize::new(0),
        };
        Streamer(Arc::new(reader), runtime)
    }

    async fn read_ahead(
        resp: wreq::Response,
        tx: mpsc::Sender<PyResult<Frame>>,
        arrived: Arc<Notify>,
    ) {
        let end = NotifyOnDrop(arrived);
        // Without this a panic would drop `tx` and read as the end of the body.
        if AssertUnwindSafe(Self::pump(resp, &tx, &end.0))
            .catch_unwind()
            .await
            .is_err()
        {
            let panicked = Err(PyRuntimeError::new_err("response body reader panicked"));
            let _ = burst(tx.send(panicked), &end.0).await;
        }
        // Readers wake only after the sender drops, so they see the end.
        drop(tx);
    }

    async fn pump(mut resp: wreq::Response, tx: &mpsc::Sender<PyResult<Frame>>, arrived: &Notify) {
        while let Some(frame) = burst(resp.frame(), arrived).await {
            let Some(frame) = Self::convert(frame, &mut resp) else {
                continue;
            };
            let failed = frame.is_err();
            if burst(tx.send(frame), arrived).await.is_err() || failed {
                break;
            }
        }
    }

    /// Convert a body frame, skipping unknown kinds.
    fn convert(
        frame: Result<http_body::Frame<Bytes>, wreq::Error>,
        resp: &mut wreq::Response,
    ) -> Option<PyResult<Frame>> {
        match frame.map(|frame| frame.into_data()) {
            Ok(Ok(bytes)) => Some(Ok(Frame::Bytes(PyBuffer::from(bytes)))),
            Ok(Err(frame)) => frame
                .into_trailers()
                .ok()
                .map(|trailers| Ok(Frame::Trailers(HeaderMap(trailers)))),
            Err(err) => {
                // Conservatively keep the connection out of the pool after a body error.
                resp.forbid_recycle();
                Some(Err(Error::Library(err).into()))
            }
        }
    }

    /// Read a frame the body already holds, before any read-ahead task starts, so a
    /// synchronous reader of a short body never waits on another thread.
    fn ready_frame(&self) -> Option<PyResult<Frame>> {
        let mut state = self.0.lock();
        let State::Idle(resp) = &mut *state else {
            return None;
        };
        loop {
            let frame = {
                let _runtime = self.1.handle().enter();
                resp.frame().now_or_never()?
            };
            let Some(frame) = frame else {
                *state = State::Closed;
                return Some(Err(Error::StopIteration.into()));
            };
            if let Some(frame) = Self::convert(frame, resp) {
                if frame.is_err() {
                    *state = State::Closed;
                }
                return Some(frame);
            }
        }
    }

    /// Return a buffered frame, or `None` if the read must wait for `arrived`.
    fn try_next(&self, error: fn() -> Error) -> Option<PyResult<Frame>> {
        let mut state = self.0.lock();
        if let State::Idle(_) = *state
            && let State::Idle(resp) = std::mem::replace(&mut *state, State::Closed)
        {
            let (tx, rx) = mpsc::channel(Self::READ_AHEAD);
            let task = self
                .1
                .handle()
                .spawn(Self::read_ahead(*resp, tx, self.0.arrived.clone()));
            *state = State::Reading {
                rx,
                _task: AbortOnDropHandle::new(task),
            };
        }
        let State::Reading { rx, .. } = &mut *state else {
            return Some(Err(error().into()));
        };
        match rx.try_recv() {
            Ok(frame) => Some(frame),
            Err(TryRecvError::Empty) => None,
            Err(TryRecvError::Disconnected) => {
                *state = State::Closed;
                Some(Err(error().into()))
            }
        }
    }
}

#[pymethods]
impl Streamer {
    fn __iter__(slf: PyRef<Self>) -> PyRef<Self> {
        slf
    }

    fn __next__(&self, py: Python) -> PyResult<Frame> {
        // Buffered frames are returned without releasing the GIL.
        if let Some(frame) = self
            .ready_frame()
            .or_else(|| self.try_next(|| Error::StopIteration))
        {
            return frame;
        }
        py.detach(|| {
            loop {
                let arrived = pin!(self.0.arrived.notified());
                if let Some(frame) = self.try_next(|| Error::StopIteration) {
                    return frame;
                }
                self.1.handle().block_on(arrived);
            }
        })
    }

    fn __enter__(slf: PyRef<Self>) -> PyRef<Self> {
        slf
    }

    /// Release the body and end any pending read; returned views stay valid.
    fn __exit__<'py>(
        &self,
        _exc_type: &Bound<'py, PyAny>,
        _exc_value: &Bound<'py, PyAny>,
        _traceback: &Bound<'py, PyAny>,
    ) {
        self.0.close();
    }
}

#[pymethods]
impl Streamer {
    fn __aiter__(slf: PyRef<Self>) -> PyRef<Self> {
        slf
    }

    /// Read the next frame when awaited.
    fn __anext__(slf: Bound<'_, Self>) -> PyResult<Bound<'_, Coroutine>> {
        let py = slf.py();
        let slf = slf.unbind();
        aio::local(py, "Streamer.__anext__", async move {
            let this = slf.get();
            // Buffered frames complete without suspending; yield to the event loop
            // periodically so timeouts, cancellation and other tasks run.
            if this.0.since_yield.fetch_add(1, Ordering::Relaxed) >= Self::YIELD_EVERY {
                this.0.since_yield.store(0, Ordering::Relaxed);
                aio::yield_now().await;
            }
            loop {
                // Readers are woken by `notify_waiters`, which reaches a `Notified`
                // from its creation, so it needs no `enable`.
                let arrived = pin!(this.0.arrived.notified());
                if let Some(frame) = this.try_next(|| Error::StopAsyncIteration) {
                    return frame;
                }
                this.0.since_yield.store(0, Ordering::Relaxed);
                arrived.await;
            }
        })
    }

    fn __aenter__(slf: Bound<'_, Self>) -> PyResult<Bound<'_, Coroutine>> {
        aio::ready("Streamer.__aenter__", slf)
    }

    /// Release the body and end any pending read; returned views stay valid.
    fn __aexit__<'py>(
        &self,
        py: Python<'py>,
        _exc_type: Py<PyAny>,
        _exc_val: Py<PyAny>,
        _traceback: Py<PyAny>,
    ) -> PyResult<Bound<'py, Coroutine>> {
        let reader = self.0.clone();
        aio::local(py, "Streamer.__aexit__", async move {
            reader.close();
            Ok(())
        })
    }
}

/// Await `fut`, notifying readers whenever it suspends, so a burst of ready
/// frames costs one wake while a slow stream still delivers each frame at once.
async fn burst<F: Future>(fut: F, arrived: &Notify) -> F::Output {
    let mut fut = pin!(fut);
    poll_fn(|cx| {
        let poll = fut.as_mut().poll(cx);
        if poll.is_pending() {
            arrived.notify_waiters();
        }
        poll
    })
    .await
}

// ===== impl Reader =====

impl Reader {
    #[inline]
    fn lock(&self) -> MutexGuard<'_, State> {
        self.state.lock().unwrap_or_else(PoisonError::into_inner)
    }

    /// Release the body and end any waiting read.
    fn close(&self) {
        let state = std::mem::replace(&mut *self.lock(), State::Closed);
        drop(state);
        self.arrived.notify_waiters();
    }
}

// ===== impl NotifyOnDrop =====

impl Drop for NotifyOnDrop {
    fn drop(&mut self) {
        self.0.notify_waiters();
    }
}

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
    /// Read a synchronous iterator ahead on the calling thread. Unless that ends it, return
    /// the pulls the caller must serve while it blocks on the request.
    pub fn feed(&mut self, py: Python<'_>) -> Option<mpsc::UnboundedReceiver<Pull>> {
        match &mut self.0 {
            PyStreamSource::Sync(stream) => stream.feed(py),
            PyStreamSource::Async(_) => None,
        }
    }
}

impl FromPyObject<'_, '_> for PyStream {
    type Error = PyErr;

    fn extract(ob: Borrowed<PyAny>) -> PyResult<Self> {
        // Iterators are checked first; probing `asend` would raise for each of them.
        let source = if ob.cast::<PyIterator>().is_err() && ob.hasattr(intern!(ob.py(), "asend"))? {
            PyStreamSource::Async(PyAsyncStream::new(ob.to_owned())?)
        } else {
            PyStreamSource::Sync(SyncStream::new(ob.to_owned().unbind()))
        };
        Ok(PyStream(source))
    }
}

impl Stream for PyStream {
    type Item = Item;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        match &mut self.get_mut().0 {
            PyStreamSource::Sync(stream) => stream.poll_next(cx),
            PyStreamSource::Async(stream) => Pin::new(stream).poll_next(cx),
        }
    }
}

// ===== impl SyncStream =====

impl SyncStream {
    /// Bytes read ahead before the request is sent, so short bodies need no later pulls.
    const READ_AHEAD: usize = 64 * 1024;

    /// Bytes the caller pulls per wake once the request is in flight.
    const PULL_BATCH: usize = 256 * 1024;

    fn new(iter: Py<PyAny>) -> Self {
        SyncStream {
            iter: Arc::new(iter),
            ahead: VecDeque::new(),
            done: false,
            caller: None,
            pending: None,
        }
    }

    fn feed(&mut self, py: Python<'_>) -> Option<mpsc::UnboundedReceiver<Pull>> {
        let batch = Batch::pull(py, &self.iter, Self::READ_AHEAD);
        self.ahead.extend(batch.items);
        self.done = batch.done;
        if self.done {
            return None;
        }
        let (tx, rx) = mpsc::unbounded_channel();
        self.caller = Some(tx);
        Some(rx)
    }

    fn poll_next(&mut self, cx: &mut Context<'_>) -> Poll<Option<Item>> {
        loop {
            if let Some(item) = self.ahead.pop_front() {
                return Poll::Ready(Some(item));
            }
            if self.done {
                return Poll::Ready(None);
            }
            let mut pending = match self.pending.take() {
                Some(pending) => pending,
                None => Self::pull(&self.iter, &mut self.caller),
            };
            match &mut pending {
                Pulling::Caller(reply) => match reply.poll_unpin(cx) {
                    Poll::Ready(Ok(batch)) => {
                        self.ahead.extend(batch.items);
                        self.done = batch.done;
                    }
                    // The caller returned without serving it, so the iterator did not advance.
                    Poll::Ready(Err(_)) => self.caller = None,
                    Poll::Pending => {
                        self.pending = Some(pending);
                        return Poll::Pending;
                    }
                },
                Pulling::Pool(task) => match task.poll_unpin(cx) {
                    Poll::Ready(item) => {
                        let item = item.ok().flatten();
                        self.done = !matches!(item, Some(Ok(_)));
                        return Poll::Ready(item);
                    }
                    Poll::Pending => {
                        self.pending = Some(pending);
                        return Poll::Pending;
                    }
                },
            }
        }
    }

    fn pull(iter: &Arc<Py<PyAny>>, caller: &mut Option<mpsc::UnboundedSender<Pull>>) -> Pulling {
        if let Some(tx) = caller {
            let (reply, rx) = oneshot::channel();
            if tx
                .send(Pull {
                    iter: iter.clone(),
                    reply,
                })
                .is_ok()
            {
                return Pulling::Caller(rx);
            }
            *caller = None;
        }
        // Acquiring the interpreter must not block a Tokio worker.
        let iter = iter.clone();
        Pulling::Pool(tokio::task::spawn_blocking(move || {
            // Once Python is unavailable, stop reading without creating a PyErr that
            // could require another attachment to format.
            Python::try_attach(|py| next_item(py, &iter)).flatten()
        }))
    }
}

// ===== impl Pull =====

impl Pull {
    /// Pull the next items on the calling thread.
    pub fn serve(self) {
        let batch = Python::attach(|py| Batch::pull(py, &self.iter, SyncStream::PULL_BATCH));
        let _ = self.reply.send(batch);
    }
}

// ===== impl Batch =====

impl Batch {
    /// Pull items until `budget` bytes, an error or the end of the iterator.
    fn pull(py: Python<'_>, iter: &Py<PyAny>, budget: usize) -> Batch {
        let mut batch = Batch {
            items: Vec::new(),
            done: false,
        };
        let mut read = 0;
        while read < budget {
            match next_item(py, iter) {
                Some(Ok(item)) => {
                    read += item.len();
                    batch.items.push(Ok(item));
                }
                Some(Err(err)) => {
                    batch.items.push(Err(err));
                    batch.done = true;
                    break;
                }
                None => {
                    batch.done = true;
                    break;
                }
            }
        }
        batch
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
    type Item = PyResult<PyBytesLike>;

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
            let handle =
                Handle::try_current().unwrap_or_else(|_| crate::runtime::get().handle().clone());
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

    fn finish<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, Coroutine>> {
        // Python may retain the sender after completion, especially on PyPy.
        let tx = self.0.clone();
        aio::local(py, "Sender.finish", async move {
            Ok(tx.send(None).await.is_ok())
        })
    }
}
