use std::{
    collections::VecDeque,
    pin::Pin,
    sync::{
        Arc, PoisonError,
        atomic::{AtomicBool, AtomicUsize, Ordering},
    },
    task::{Context, Poll, Wake, Waker},
};

use bytes::Bytes;
use futures_util::{FutureExt, Stream, future::poll_fn, ready};
use http_body_util::BodyExt;
use pyo3::{
    coroutine::CancelHandle, exceptions::PyStopIteration, intern, prelude::*, sync::PyOnceLock,
};
use tokio::{sync::mpsc, task::JoinHandle};
use tokio_util::task::AbortOnDropHandle;

use crate::{
    buffer::PyBuffer,
    client::nogil,
    error::Error,
    extractor::{BytesInput, StrInput},
    header::HeaderMap,
    runtime::Runtime,
};

type Pending = Option<JoinHandle<Option<PyResult<PyBytesLike>>>>;

/// Python stream source.
enum PyStreamSource {
    Sync(Arc<Py<PyAny>>),
    Async(PyAsyncStream),
}

/// A bytes-like object that can be extracted from Python.
#[derive(FromPyObject)]
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
pub struct PyStream {
    inner: PyStreamSource,
    pending: Pending,
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
#[derive(Clone)]
#[pyclass(subclass, frozen, skip_from_py_object)]
pub struct Streamer(Arc<Inner>);

/// Streamer state shared by Python iterators and context managers.
struct Inner {
    buffer: Arc<Buffer>,
    task: std::sync::Mutex<Task>,
    started: AtomicBool,
    runtime: Runtime,
    /// Consecutive `__anext__` calls that returned without suspending.
    ready: AtomicUsize,
}

enum Task {
    /// Not read yet, so a read timeout has not started.
    Idle(Box<wreq::Response>),
    /// Dropping the handle aborts the read-ahead task.
    Running {
        _abort: AbortOnDropHandle<()>,
    },
    Closed,
}

/// Frames read ahead by a Tokio task, bounded by bytes and shared with readers.
/// Locks are held only within a single poll, so no waiting reader holds them.
#[derive(Default)]
struct Buffer(std::sync::Mutex<BufferState>);

#[derive(Default)]
struct BufferState {
    items: VecDeque<Item>,
    buffered: usize,
    end: bool,
    closed: bool,
    readers: Vec<Waker>,
    producer: Option<Waker>,
}

enum Item {
    Data(Bytes),
    Trailers(http::HeaderMap),
    Error(wreq::Error),
}

/// Ends the buffer however the read-ahead task stops, so no reader waits forever.
struct EndGuard(Arc<Buffer>);

/// Wakes a thread blocked in a synchronous read.
struct ThreadWaker(std::thread::Thread);

// ===== impl Streamer =====

impl Streamer {
    /// Buffered frames returned before `__anext__` yields to the event loop once.
    const YIELD_EVERY: usize = 8;

    /// Create a new [`Streamer`]; reading starts on the first iteration.
    pub fn new(resp: wreq::Response, runtime: &Runtime) -> Streamer {
        Streamer(Arc::new(Inner {
            buffer: Arc::default(),
            task: std::sync::Mutex::new(Task::Idle(Box::new(resp))),
            started: AtomicBool::new(false),
            runtime: runtime.clone(),
            ready: AtomicUsize::new(0),
        }))
    }

    async fn read_ahead(mut resp: wreq::Response, buffer: Arc<Buffer>) {
        let _end = EndGuard(buffer.clone());
        while let Some(frame) = resp.frame().await {
            let item = match frame {
                Ok(frame) => match frame.into_data() {
                    Ok(bytes) => Item::Data(bytes),
                    Err(frame) => match frame.into_trailers() {
                        Ok(trailers) => Item::Trailers(trailers),
                        Err(_) => continue,
                    },
                },
                Err(err) => {
                    // A failed body may leave a stalled HTTP/2 connection behind.
                    resp.forbid_recycle();
                    buffer.push(Item::Error(err));
                    return;
                }
            };
            if !buffer.push(item) || !poll_fn(|cx| buffer.poll_capacity(cx)).await {
                return;
            }
        }
    }

    /// Start the read-ahead task on the first read.
    fn start(&self) {
        if self.0.started.load(Ordering::Acquire) {
            return;
        }
        let mut task = self.0.task.lock().unwrap_or_else(PoisonError::into_inner);
        if let Task::Idle(_) = *task
            && let Task::Idle(resp) = std::mem::replace(&mut *task, Task::Closed)
        {
            let handle = self
                .0
                .runtime
                .handle()
                .spawn(Self::read_ahead(*resp, self.0.buffer.clone()));
            *task = Task::Running {
                _abort: AbortOnDropHandle::new(handle),
            };
        }
        self.0.started.store(true, Ordering::Release);
    }

    fn poll_next(&self, cx: &mut Context<'_>, error: fn() -> Error) -> Poll<PyResult<Frame>> {
        self.start();
        Poll::Ready(match ready!(self.0.buffer.poll_item(cx)) {
            Some(Item::Data(bytes)) => Ok(Frame::Bytes(PyBuffer::from(bytes))),
            Some(Item::Trailers(trailers)) => Ok(Frame::Trailers(HeaderMap(trailers))),
            Some(Item::Error(err)) => Err(Error::Library(err).into()),
            None => Err(error().into()),
        })
    }

    /// Release the body and wake any reader still waiting on it.
    fn close(&self) {
        self.0.started.store(true, Ordering::Release);
        self.0.buffer.close();
        let task = {
            let mut task = self.0.task.lock().unwrap_or_else(PoisonError::into_inner);
            std::mem::replace(&mut *task, Task::Closed)
        };
        drop(task);
    }
}

#[pymethods]
impl Streamer {
    #[inline]
    fn __iter__(slf: PyRef<Self>) -> PyRef<Self> {
        slf
    }

    fn __next__(&self, py: Python) -> PyResult<Frame> {
        py.detach(|| {
            let waker = Waker::from(Arc::new(ThreadWaker(std::thread::current())));
            let mut cx = Context::from_waker(&waker);
            loop {
                match self.poll_next(&mut cx, || Error::StopIteration) {
                    Poll::Ready(frame) => return frame,
                    Poll::Pending => std::thread::park(),
                }
            }
        })
    }

    #[inline]
    fn __enter__(slf: PyRef<Self>) -> PyRef<Self> {
        slf
    }

    #[inline]
    fn __exit__<'py>(
        &self,
        py: Python,
        _exc_type: &Bound<'py, PyAny>,
        _exc_value: &Bound<'py, PyAny>,
        _traceback: &Bound<'py, PyAny>,
    ) {
        py.detach(|| self.close());
    }
}

#[pymethods]
impl Streamer {
    #[inline]
    fn __aiter__(slf: PyRef<Self>) -> PyRef<Self> {
        slf
    }

    /// Read the next frame when awaited; returns a coroutine, not a Future.
    fn __anext__<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        let this = self.clone();
        let mut yielded = false;
        let mut suspended = false;
        // PyO3 0.29 cannot wrap an async __anext__ slot; use its macro constructor.
        // Recheck this internal API when upgrading PyO3. Without a throw callback,
        // cancellation drops the poll, which holds no lock and loses no frame.
        Bound::new(
            py,
            pyo3::impl_::coroutine::new_coroutine(
                intern!(py, "__anext__"),
                Some("Streamer"),
                None,
                async move {
                    let error = || Error::StopAsyncIteration;
                    let frame = poll_fn(|cx| {
                        // Buffered frames complete without suspending; yield to the event
                        // loop periodically so timeouts, cancellation and other tasks run.
                        if !yielded && this.0.ready.load(Ordering::Relaxed) >= Self::YIELD_EVERY {
                            yielded = true;
                            this.0.ready.store(0, Ordering::Relaxed);
                            cx.waker().wake_by_ref();
                            return Poll::Pending;
                        }
                        if let Poll::Ready(frame) =
                            this.poll_next(&mut Context::from_waker(Waker::noop()), error)
                        {
                            if !suspended {
                                this.0.ready.fetch_add(1, Ordering::Relaxed);
                            }
                            return Poll::Ready(frame);
                        }
                        // The read-ahead task wakes this reader from a Tokio thread.
                        suspended = true;
                        this.0.ready.store(0, Ordering::Relaxed);
                        this.poll_next(&mut Context::from_waker(&nogil::guard(cx.waker())), error)
                    })
                    .await?;
                    Python::attach(|py| frame.into_pyobject(py).map(|obj| obj.unbind()))
                },
            ),
        )
        .map(Bound::into_any)
    }

    #[inline]
    async fn __aenter__(slf: Py<Self>) -> PyResult<Py<Self>> {
        Ok(slf)
    }

    #[inline]
    async fn __aexit__(
        &self,
        _exc_type: Py<PyAny>,
        _exc_val: Py<PyAny>,
        _traceback: Py<PyAny>,
    ) -> PyResult<()> {
        self.close();
        Ok(())
    }
}

// ===== impl Buffer =====

impl Buffer {
    /// Bytes read ahead while Python lags; the reader resumes below half of it.
    const READ_AHEAD: usize = 128 * 1024;

    fn lock(&self) -> std::sync::MutexGuard<'_, BufferState> {
        self.0.lock().unwrap_or_else(PoisonError::into_inner)
    }

    /// Queue an item for readers; returns `false` once the stream is closed.
    fn push(&self, item: Item) -> bool {
        let readers = {
            let mut state = self.lock();
            if state.closed {
                return false;
            }
            if let Item::Data(bytes) = &item {
                state.buffered = state.buffered.saturating_add(bytes.len());
            }
            state.items.push_back(item);
            std::mem::take(&mut state.readers)
        };
        readers.into_iter().for_each(Waker::wake);
        true
    }

    /// Wait until the buffer has room; resolves to `false` once the stream is closed.
    fn poll_capacity(&self, cx: &mut Context<'_>) -> Poll<bool> {
        let mut state = self.lock();
        if state.closed {
            Poll::Ready(false)
        } else if state.buffered < Self::READ_AHEAD {
            Poll::Ready(true)
        } else {
            state.producer = Some(cx.waker().clone());
            Poll::Pending
        }
    }

    /// Take the next item, or `None` at the end of a finished or closed stream.
    fn poll_item(&self, cx: &mut Context<'_>) -> Poll<Option<Item>> {
        let (item, producer) = {
            let mut state = self.lock();
            if state.closed {
                return Poll::Ready(None);
            }
            let Some(item) = state.items.pop_front() else {
                if state.end {
                    return Poll::Ready(None);
                }
                if !state.readers.iter().any(|w| w.will_wake(cx.waker())) {
                    state.readers.push(cx.waker().clone());
                }
                return Poll::Pending;
            };
            if let Item::Data(bytes) = &item {
                state.buffered = state.buffered.saturating_sub(bytes.len());
            }
            let producer = if state.buffered <= Self::READ_AHEAD / 2 {
                state.producer.take()
            } else {
                None
            };
            (item, producer)
        };
        if let Some(producer) = producer {
            producer.wake();
        }
        Poll::Ready(Some(item))
    }

    /// Mark the end of the body and wake readers waiting for more.
    fn finish(&self) {
        let readers = {
            let mut state = self.lock();
            state.end = true;
            std::mem::take(&mut state.readers)
        };
        readers.into_iter().for_each(Waker::wake);
    }

    /// Discard buffered frames and wake both sides.
    fn close(&self) {
        let (items, readers, producer) = {
            let mut state = self.lock();
            state.closed = true;
            state.buffered = 0;
            (
                std::mem::take(&mut state.items),
                std::mem::take(&mut state.readers),
                state.producer.take(),
            )
        };
        drop(items);
        readers.into_iter().for_each(Waker::wake);
        if let Some(producer) = producer {
            producer.wake();
        }
    }
}

// ===== impl EndGuard =====

impl Drop for EndGuard {
    fn drop(&mut self) {
        self.0.finish();
    }
}

// ===== impl ThreadWaker =====

impl Wake for ThreadWaker {
    fn wake(self: Arc<Self>) {
        self.0.unpark();
    }

    fn wake_by_ref(self: &Arc<Self>) {
        self.0.unpark();
    }
}

// ===== impl PyBytesLike =====

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

impl From<PyStreamSource> for PyStream {
    #[inline]
    fn from(inner: PyStreamSource) -> Self {
        PyStream {
            inner,
            pending: None,
        }
    }
}

impl FromPyObject<'_, '_> for PyStream {
    type Error = PyErr;

    fn extract(ob: Borrowed<PyAny>) -> PyResult<Self> {
        if ob.hasattr(intern!(ob.py(), "asend"))? {
            PyAsyncStream::new(ob.to_owned())
                .map(PyStreamSource::Async)
                .map(PyStream::from)
        } else {
            ob.extract::<Py<PyAny>>()
                .map(Arc::new)
                .map(PyStreamSource::Sync)
                .map(PyStream::from)
                .map_err(Into::into)
        }
    }
}

impl Stream for PyStream {
    type Item = PyResult<PyBytesLike>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let this = self.as_mut().get_mut();
        let ob = match &mut this.inner {
            PyStreamSource::Async(stream) => return Pin::new(stream).poll_next(cx),
            PyStreamSource::Sync(ob) => ob,
        };
        let mut pending = match this.pending.take() {
            Some(pending) => pending,
            None => {
                // Acquiring the interpreter must not block a Tokio worker.
                let ob = ob.clone();
                tokio::task::spawn_blocking(move || {
                    Python::try_attach(|py| match ob.call_method0(py, intern!(py, "__next__")) {
                        Ok(ob) => Some(ob.extract(py)),
                        Err(err) if err.is_instance_of::<PyStopIteration>(py) => None,
                        Err(err) => Some(Err(err)),
                    })
                    // Once Python is unavailable, stop reading without creating
                    // a PyErr that could require another attachment to format.
                    .flatten()
                })
            }
        };

        match pending.poll_unpin(cx) {
            Poll::Ready(Ok(res)) => Poll::Ready(res),
            Poll::Ready(Err(_)) => Poll::Ready(None),
            Poll::Pending => {
                this.pending.replace(pending);
                Poll::Pending
            }
        }
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
            // Body drop can run on Tokio: acquire the interpreter on a blocking thread.
            crate::runtime::get().handle().spawn_blocking(move || {
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
    async fn send(
        &self,
        item: Py<PyAny>,
        error: bool,
        #[pyo3(cancel_handle)] cancel: CancelHandle,
    ) -> PyResult<bool> {
        let item = Python::attach(|py| {
            if error {
                Ok(Err(PyErr::from_value(item.into_bound(py))))
            } else {
                item.extract(py).map(Ok)
            }
        })?;
        self.send_item(Some(item), cancel).await
    }

    async fn finish(&self, #[pyo3(cancel_handle)] cancel: CancelHandle) -> PyResult<bool> {
        // Python may retain the sender after completion, especially on PyPy.
        self.send_item(None, cancel).await
    }
}

impl Sender {
    async fn send_item(
        &self,
        item: Option<PyResult<PyBytesLike>>,
        mut cancel: CancelHandle,
    ) -> PyResult<bool> {
        let item = match self.0.try_send(item) {
            Ok(()) => return Ok(true),
            Err(mpsc::error::TrySendError::Closed(_)) => return Ok(false),
            Err(mpsc::error::TrySendError::Full(item)) => item,
        };
        let tx = self.0.clone();
        // Channel readiness is runtime-independent; keep this on the Python loop.
        let mut send = std::pin::pin!(tx.send(item));
        tokio::select! {
            biased;
            exception = poll_fn(|cx| cancel.poll_cancelled(cx)) => {
                Err(Python::attach(|py| PyErr::from_value(exception.into_bound(py))))
            }
            result = poll_fn(|cx| nogil::poll_with_guard(send.as_mut(), cx)) => Ok(result.is_ok()),
        }
    }
}
