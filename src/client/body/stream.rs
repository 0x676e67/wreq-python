use std::{
    pin::Pin,
    sync::Arc,
    task::{Context, Poll},
};

use bytes::Bytes;
use futures_util::{FutureExt, Stream, StreamExt, stream::BoxStream};
use http_body_util::BodyExt;
use pyo3::{coroutine::CancelHandle, exceptions::PyStopIteration, intern, prelude::*};
use tokio::{sync::Mutex, task::JoinHandle};

use crate::{
    buffer::PyBuffer,
    client::nogil::NoGIL,
    error::{self, Error},
    extractor::{BytesInput, StrInput},
    header::HeaderMap,
};

type Pending = Option<JoinHandle<Option<PyResult<PyBytesLike>>>>;

/// Python stream source.
enum PyStreamSource {
    Sync(Arc<Py<PyAny>>),
    Async(Arc<Mutex<BoxStream<'static, Py<PyAny>>>>),
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

/// A response stream yielding read-only memoryviews and any trailing headers.
#[derive(Clone)]
#[pyclass(subclass, frozen, skip_from_py_object)]
pub struct Streamer(Arc<Mutex<Option<wreq::Response>>>);

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

// ===== impl Streamer =====

impl Streamer {
    /// Create a new [`Streamer`] instance.
    #[inline]
    pub fn new(resp: wreq::Response) -> Streamer {
        Streamer(Arc::new(Mutex::new(Some(resp))))
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
        py.detach(|| {
            pyo3_async_runtimes::tokio::get_runtime()
                .block_on(self.clone().next(|| Error::StopIteration))
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
        py.detach(|| self.0.blocking_lock().take());
    }
}

#[pymethods]
impl Streamer {
    #[inline]
    fn __aiter__(slf: PyRef<Self>) -> PyRef<Self> {
        slf
    }

    #[inline]
    fn __anext__<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        pyo3_async_runtimes::tokio::future_into_py(
            py,
            self.clone().next(|| Error::StopAsyncIteration),
        )
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
        NoGIL::new(
            async move {
                if let Some(resp) = this.lock().await.take() {
                    drop(resp)
                }
                Ok(())
            },
            CancelHandle::new(),
        )
        .await
    }
}

// ===== PyBytesLike =====

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
        if ob.hasattr(intern!(ob.py(), "asend"))? {
            pyo3_async_runtimes::tokio::into_stream_v2(ob.to_owned())
                .map(StreamExt::boxed)
                .map(Mutex::new)
                .map(Arc::new)
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
        let mut pending = match this.pending.take() {
            Some(pending) => pending,
            None => {
                let runtime = pyo3_async_runtimes::tokio::get_runtime();

                // Move GIL acquisition to blocking threads to prevent blocking async runtime.
                // This is crucial because holding the GIL in async tasks can block the entire
                // async executor and cause deadlocks or performance degradation.
                match this.inner {
                    PyStreamSource::Sync(ref ob) => {
                        let ob = ob.clone();
                        runtime.spawn_blocking(move || {
                            error::attach(|py| match ob.call_method0(py, intern!(py, "__next__")) {
                                Ok(ob) => Some(ob.extract(py)),
                                Err(err) if err.is_instance_of::<PyStopIteration>(py) => None,
                                Err(err) => Some(Err(err)),
                            })
                            .unwrap_or_else(|err| Some(Err(err.into())))
                        })
                    }
                    PyStreamSource::Async(ref stream) => {
                        let stream = stream.clone();
                        runtime.spawn(async move {
                            let ob = stream.lock().await.next().await;
                            tokio::task::spawn_blocking(move || {
                                error::attach(|py| ob.map(|ob| ob.extract(py)))
                                    .unwrap_or_else(|err| Some(Err(err.into())))
                            })
                            .await
                            .ok()?
                        })
                    }
                }
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
