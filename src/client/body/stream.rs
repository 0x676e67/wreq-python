use std::{
    pin::Pin,
    sync::{
        Arc, PoisonError,
        atomic::{AtomicUsize, Ordering},
    },
    task::{Context, Poll},
};

use bytes::Bytes;
use futures_util::{FutureExt, Stream, future::poll_fn};
use http_body_util::BodyExt;
use pyo3::{
    coroutine::CancelHandle, exceptions::PyStopIteration, intern, prelude::*, sync::PyOnceLock,
};
use tokio::{
    sync::{
        Mutex,
        mpsc::{self, error::TryRecvError},
    },
    task::JoinHandle,
};
use tokio_util::task::AbortOnDropHandle;

use crate::{
    buffer::PyBuffer,
    client::nogil::{self, NoGIL},
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
/// timeout does not start before iteration. Waiting reads hold the lock in a
/// Tokio task, never in a suspended Python coroutine.
struct Reader {
    state: Mutex<State>,
    task: std::sync::Mutex<Option<AbortOnDropHandle<()>>>,
    /// Buffered frames returned since `__anext__` last yielded to the event loop.
    ready: AtomicUsize,
}

enum State {
    Idle(Box<wreq::Response>),
    Reading(mpsc::Receiver<PyResult<Frame>>),
    Closed,
}

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
            task: std::sync::Mutex::new(None),
            ready: AtomicUsize::new(0),
        };
        Streamer(Arc::new(reader), runtime)
    }

    async fn read_ahead(mut resp: wreq::Response, tx: mpsc::Sender<PyResult<Frame>>) {
        while let Some(frame) = resp.frame().await {
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
            if tx.send(frame).await.is_err() || failed {
                break;
            }
        }
    }

    /// Return a buffered frame, or `None` if the read must wait.
    fn poll_state(&self, state: &mut State, error: fn() -> Error) -> Option<PyResult<Frame>> {
        if let State::Idle(_) = state
            && let State::Idle(resp) = std::mem::replace(state, State::Closed)
        {
            let (tx, rx) = mpsc::channel(Self::READ_AHEAD);
            let task = self.1.handle().spawn(Self::read_ahead(*resp, tx));
            *self.0.task.lock().unwrap_or_else(PoisonError::into_inner) =
                Some(AbortOnDropHandle::new(task));
            *state = State::Reading(rx);
            return None;
        }
        let State::Reading(rx) = state else {
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

    /// Return a buffered frame without waiting or spawning.
    fn try_next(&self, error: fn() -> Error) -> Option<PyResult<Frame>> {
        let mut state = self.0.state.try_lock().ok()?;
        self.poll_state(&mut state, error)
    }

    async fn next(self, error: fn() -> Error) -> PyResult<Frame> {
        let mut state = self.0.state.lock().await;
        if let Some(frame) = self.poll_state(&mut state, error) {
            return frame;
        }
        let State::Reading(rx) = &mut *state else {
            return Err(error().into());
        };
        match rx.recv().await {
            Some(frame) => frame,
            None => {
                *state = State::Closed;
                Err(error().into())
            }
        }
    }

    /// Stop reading ahead; a waiting read then ends and releases the state.
    fn abort(&self) {
        drop(
            self.0
                .task
                .lock()
                .unwrap_or_else(PoisonError::into_inner)
                .take(),
        );
    }
}

#[pymethods]
impl Streamer {
    #[inline]
    fn __iter__(slf: PyRef<Self>) -> PyRef<Self> {
        slf
    }

    #[inline]
    fn __next__(&self, py: Python) -> PyResult<Frame> {
        py.detach(|| {
            self.try_next(|| Error::StopIteration).unwrap_or_else(|| {
                nogil::block_on(&self.1, self.clone().next(|| Error::StopIteration))
            })
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
        py.detach(|| {
            self.abort();
            *self.0.state.blocking_lock() = State::Closed;
        });
    }
}

#[pymethods]
impl Streamer {
    #[inline]
    fn __aiter__(slf: PyRef<Self>) -> PyRef<Self> {
        slf
    }

    /// Read the next frame when awaited; returns a coroutine, not a Future.
    #[inline]
    fn __anext__<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        let this = self.clone();
        // PyO3 0.29 cannot wrap an async __anext__ slot; use its macro constructor.
        // Recheck this internal API when upgrading PyO3. Without a throw callback,
        // cancellation drops the read, which aborts any spawned wait.
        Bound::new(
            py,
            pyo3::impl_::coroutine::new_coroutine(
                intern!(py, "__anext__"),
                Some("Streamer"),
                None,
                async move {
                    let error = || Error::StopAsyncIteration;
                    // Buffered frames complete without suspending; yield to the event
                    // loop periodically so timeouts, cancellation and other tasks run.
                    if this.0.ready.fetch_add(1, Ordering::Relaxed) >= Self::YIELD_EVERY {
                        this.0.ready.store(0, Ordering::Relaxed);
                        let mut yielded = false;
                        poll_fn(|cx| {
                            if std::mem::replace(&mut yielded, true) {
                                return Poll::Ready(());
                            }
                            cx.waker().wake_by_ref();
                            Poll::Pending
                        })
                        .await;
                    }
                    let frame = match this.try_next(error) {
                        Some(frame) => frame?,
                        None => {
                            this.0.ready.store(0, Ordering::Relaxed);
                            let runtime = this.1.clone();
                            NoGIL::new(&runtime, this.next(error)).await?
                        }
                    };
                    // PyO3 polls this coroutine while attached, outside the Tokio task.
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
        self.abort();
        if let Ok(mut state) = self.0.state.try_lock() {
            *state = State::Closed;
            return Ok(());
        }
        let reader = self.0.clone();
        NoGIL::new(&self.1, async move {
            *reader.state.lock().await = State::Closed;
            Ok(())
        })
        .await
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
