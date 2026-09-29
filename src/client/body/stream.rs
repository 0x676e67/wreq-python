mod upload;

use std::{
    pin::Pin,
    sync::Arc,
    task::{Context, Poll},
};

use bytes::Bytes;
use futures_util::{FutureExt, Stream};
use http_body_util::BodyExt;
use pyo3::{
    coroutine::CancelHandle,
    intern,
    prelude::*,
    pybacked::{PyBackedBytes, PyBackedStr},
};
use tokio::{sync::Mutex, task::JoinHandle};

use crate::{
    buffer::PyBuffer, client::nogil::NoGIL, error::Error, header::HeaderMap, runtime::Executor,
};

type Pending = Option<JoinHandle<Option<PyResult<PyBytesLike>>>>;

/// Python stream source.
enum PyStreamSource {
    Sync(Arc<Py<PyAny>>),
    Async(upload::Upload),
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

/// A bytes stream response.
#[derive(Clone)]
#[pyclass(subclass, frozen, skip_from_py_object)]
pub struct Streamer(Arc<Mutex<Option<wreq::Response>>>, Executor);

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
    pub fn new(resp: wreq::Response, runtime: Executor) -> Streamer {
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
        py.detach(|| self.1.block_on(self.clone().next(|| Error::StopIteration)))
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
                    let frame =
                        NoGIL::new(&runtime, this.next(|| Error::StopAsyncIteration), cancel)?
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
        NoGIL::new(
            &self.1,
            async move {
                if let Some(resp) = this.lock().await.take() {
                    drop(resp)
                }
                Ok(())
            },
            CancelHandle::new(),
        )?
        .await
    }
}

// ===== PyBytesLike =====

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

impl FromPyObject<'_, '_> for PyStream {
    type Error = PyErr;

    fn extract(ob: Borrowed<PyAny>) -> PyResult<Self> {
        if ob.hasattr(intern!(ob.py(), "asend"))? {
            upload::Upload::new(ob.to_owned())
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
                    Python::attach(|py| {
                        ob.call_method0(py, intern!(py, "__next__"))
                            .ok()
                            .map(|ob| ob.extract(py))
                    })
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
