use std::{
    future::Future,
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
use pyo3::{exceptions::PyStopIteration, intern, prelude::*, sync::PyOnceLock};
use tokio::{
    sync::{
        Notify,
        mpsc::{self, error::TryRecvError},
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
pub struct Streamer(Arc<Reader>, Runtime);

/// Frames read ahead by a Tokio task, which starts on the first read so a read
/// timeout does not start before iteration. Readers poll the buffer from Python
/// and wait on `arrived`, which the task signals once per burst of frames.
struct Reader {
    state: Mutex<State>,
    arrived: Arc<Notify>,
    /// Buffered frames returned since `__anext__` last yielded to the event loop.
    ready: AtomicUsize,
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
    /// Frames buffered ahead of Python, about 128 KiB of full TLS records.
    const READ_AHEAD: usize = 8;

    /// Buffered frames returned before `__anext__` yields to the event loop once.
    const YIELD_EVERY: usize = 8;

    /// Create a new [`Streamer`] instance.
    #[inline]
    pub fn new(resp: wreq::Response, runtime: Runtime) -> Streamer {
        let reader = Reader {
            state: Mutex::new(State::Idle(Box::new(resp))),
            arrived: Arc::new(Notify::new()),
            ready: AtomicUsize::new(0),
        };
        Streamer(Arc::new(reader), runtime)
    }

    async fn read_ahead(
        resp: wreq::Response,
        tx: mpsc::Sender<PyResult<Frame>>,
        arrived: Arc<Notify>,
    ) {
        // Readers wake only after `pump` drops the sender, so they see the end.
        let _end = NotifyOnDrop(arrived.clone());
        Self::pump(resp, tx, &arrived).await;
    }

    async fn pump(mut resp: wreq::Response, tx: mpsc::Sender<PyResult<Frame>>, arrived: &Notify) {
        while let Some(frame) = burst(resp.frame(), arrived).await {
            let frame = match frame.map(|frame| frame.into_data()) {
                Ok(Ok(bytes)) => Ok(Frame::Bytes(PyBuffer::from(bytes))),
                Ok(Err(frame)) => match frame.into_trailers() {
                    Ok(trailers) => Ok(Frame::Trailers(HeaderMap(trailers))),
                    Err(_) => continue,
                },
                Err(err) => {
                    // A failed body may leave a stalled HTTP/2 connection behind.
                    resp.forbid_recycle();
                    Err(Error::Library(err).into())
                }
            };
            let failed = frame.is_err();
            if burst(tx.send(frame), arrived).await.is_err() || failed {
                break;
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
    #[inline]
    fn __iter__(slf: PyRef<Self>) -> PyRef<Self> {
        slf
    }

    fn __next__(&self, py: Python) -> PyResult<Frame> {
        // Buffered frames are returned without releasing the GIL.
        if let Some(frame) = self.try_next(|| Error::StopIteration) {
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
        py.detach(|| self.0.close());
    }
}

#[pymethods]
impl Streamer {
    #[inline]
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
            if this.0.ready.fetch_add(1, Ordering::Relaxed) >= Self::YIELD_EVERY {
                this.0.ready.store(0, Ordering::Relaxed);
                aio::yield_now().await;
            }
            loop {
                // Readers are woken by `notify_waiters`, which reaches a `Notified`
                // from its creation, so it needs no `enable`.
                let arrived = pin!(this.0.arrived.notified());
                if let Some(frame) = this.try_next(|| Error::StopAsyncIteration) {
                    return frame;
                }
                this.0.ready.store(0, Ordering::Relaxed);
                arrived.await;
            }
        })
    }

    #[inline]
    fn __aenter__(slf: Bound<'_, Self>) -> PyResult<Bound<'_, Coroutine>> {
        aio::ready("Streamer.__aenter__", slf)
    }

    #[inline]
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
        aio::local(
            py,
            "Sender.send",
            Self::send_item(self.0.clone(), Some(item)),
        )
    }

    fn finish<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, Coroutine>> {
        // Python may retain the sender after completion, especially on PyPy.
        aio::local(py, "Sender.finish", Self::send_item(self.0.clone(), None))
    }
}

impl Sender {
    /// Channel readiness is runtime-independent, so this waits on the Python loop.
    async fn send_item(
        tx: mpsc::Sender<Option<PyResult<PyBytesLike>>>,
        item: Option<PyResult<PyBytesLike>>,
    ) -> PyResult<bool> {
        Ok(tx.send(item).await.is_ok())
    }
}
