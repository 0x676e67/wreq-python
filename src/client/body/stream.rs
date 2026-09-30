use std::{
    pin::Pin,
    sync::Arc,
    task::{Context, Poll},
};

use bytes::Bytes;
use futures_util::{FutureExt, Stream, future::poll_fn};
use http_body_util::BodyExt;
use pyo3::{
    coroutine::CancelHandle,
    intern,
    prelude::*,
    pybacked::{PyBackedBytes, PyBackedStr},
    sync::PyOnceLock,
};
use tokio::{
    sync::{Mutex, mpsc},
    task::JoinHandle,
};

use crate::{
    buffer::PyBuffer,
    client::nogil::{self, NoGIL},
    error::Error,
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
    Bytes(PyBackedBytes),
    String(PyBackedStr),
}

/// A bytes-like object that can be into Python.
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
    rx: mpsc::Receiver<PyResult<PyBytesLike>>,
    task: Option<(Py<PyAny>, Py<PyAny>)>,
}

#[pyclass(frozen)]
struct Sender(mpsc::Sender<PyResult<PyBytesLike>>);

/// A bytes stream response.
#[derive(Clone)]
#[pyclass(subclass, frozen, skip_from_py_object)]
pub struct Streamer(Arc<Mutex<Option<wreq::Response>>>, Runtime);

// ===== impl Streamer =====

impl Streamer {
    /// Create a new [`Streamer`] instance.
    #[inline]
    pub fn new(resp: wreq::Response, runtime: Runtime) -> Streamer {
        Streamer(Arc::new(Mutex::new(Some(resp))), runtime)
    }

    async fn next(self, error: fn() -> Error) -> PyResult<Frame> {
        let frame = self
            .0
            .lock()
            .await
            .as_mut()
            .ok_or_else(error)?
            .frame()
            .await
            .ok_or_else(error)?
            .map_err(Error::Library)?
            .into_data()
            .map_err(|frame| frame.into_trailers());

        match frame {
            Ok(bytes) => Ok(Frame::Bytes(PyBuffer::from(bytes))),
            Err(Ok(trailers)) => Ok(Frame::Trailers(HeaderMap(trailers))),
            Err(Err(frame)) => {
                // This branch should be unreachable, as `http_body::Frame` can only be `Data` or
                // `Trailers`. The `debug_assert!` will help catch any future
                // changes that violate this assumption.
                debug_assert!(false, "Unexpected frame type: {:?}", frame);
                Err(error().into())
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

    #[inline]
    fn __next__(&self, py: Python) -> PyResult<Frame> {
        py.detach(|| nogil::block_on(&self.1, self.clone().next(|| Error::StopIteration)))
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
        py.detach(|| self.0.blocking_lock().take());
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
        let cancel = CancelHandle::new();
        // PyO3 0.29 cannot wrap an async __anext__ slot; use its macro constructor.
        // Recheck this internal API when upgrading PyO3.
        Bound::new(
            py,
            pyo3::impl_::coroutine::new_coroutine(
                intern!(py, "__anext__"),
                Some("Streamer"),
                Some(cancel.throw_callback()),
                async move {
                    let runtime = this.1.clone();
                    let frame = NoGIL::with_cancel(
                        &runtime,
                        this.next(|| Error::StopAsyncIteration),
                        cancel,
                    )
                    .await?;
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
        let this = self.0.clone();
        NoGIL::new(&self.1, async move {
            if let Some(resp) = this.lock().await.take() {
                drop(resp)
            }
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
            PyBytesLike::Bytes(b) => Bytes::from_owner(b),
            PyBytesLike::String(s) => Bytes::from_owner(s),
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
                    Python::try_attach(|py| {
                        ob.call_method0(py, intern!(py, "__next__"))
                            .ok()
                            .map(|ob| ob.extract(py))
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
        self.get_mut().rx.poll_recv(cx)
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
        #[pyo3(cancel_handle)] mut cancel: CancelHandle,
    ) -> PyResult<bool> {
        let item = Python::attach(|py| {
            if error {
                Ok(Err(PyErr::from_value(item.into_bound(py))))
            } else {
                item.extract(py).map(Ok)
            }
        })?;
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
