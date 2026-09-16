use std::{fmt::Display, sync::Arc};

use arc_swap::ArcSwapOption;
use bytes::Bytes;
use futures_util::{
    TryFutureExt,
    future::{self, BoxFuture},
};
use http::response::{Parts, Response as HttpResponse};
use http_body_util::{BodyExt, Collected};
use pyo3::{coroutine::CancelHandle, prelude::*, pybacked::PyBackedStr};
use wreq::{self, Uri};

use crate::{
    buffer::PyBuffer,
    client::{
        SocketAddr,
        body::{Json, Streamer},
        nogil::NoGIL,
        resp::ext::ResponseExt,
    },
    cookie::Cookie,
    error::Error,
    header::HeaderMap,
    http::{StatusCode, Version},
    redirect::History,
    tls::TlsInfo,
};

/// A response from a request.
#[pyclass(subclass, frozen, str, skip_from_py_object)]
pub struct Response {
    uri: Uri,
    parts: Parts,
    body: Arc<ArcSwapOption<Body>>,
}

/// Represents the state of the HTTP response body.
enum Body {
    /// The body can be streamed once (not yet buffered).
    Streamable(wreq::Body),
    /// The body has been fully read into memory and can be reused.
    Reusable(Bytes),
}

/// A blocking response from a request.
#[pyclass(name = "Response", subclass, frozen, str, skip_from_py_object)]
pub struct BlockingResponse(Response);

// ===== impl Response =====

impl Response {
    /// Create a new [`Response`] instance.
    pub fn new(response: wreq::Response) -> Self {
        let uri = response.uri().clone();
        let response = HttpResponse::from(response)
            .map(Body::Streamable)
            .map(ArcSwapOption::from_pointee)
            .map(Arc::new);
        let (parts, body) = response.into_parts();
        Response { uri, parts, body }
    }

    /// Builds a [`wreq::Response`] from the current response metadata and the given body.
    #[inline]
    fn build_response<T: Into<wreq::Body>>(&self, body: T) -> wreq::Response {
        let response = HttpResponse::from_parts(self.parts.clone(), body);
        wreq::Response::from(response)
    }

    /// Creates an empty [`wreq::Response`] with the same metadata but no body content.
    #[inline]
    fn empty_response(&self) -> wreq::Response {
        self.build_response(Bytes::new())
    }

    /// Consumes the response [`Body`] and caches it in memory for reuse.
    fn cache_response(&self) -> BoxFuture<'static, Result<wreq::Response, Error>> {
        if let Some(arc) = self.body.swap(None) {
            let parts = self.parts.clone();
            let body = self.body.clone();
            match Arc::into_inner(arc) {
                Some(Body::Streamable(stream)) => {
                    return Box::pin(async move {
                        let bytes = stream
                            .collect()
                            .await
                            .map(Collected::to_bytes)
                            .map_err(Error::Library)?;

                        body.store(Some(Arc::new(Body::Reusable(bytes.clone()))));
                        let response = HttpResponse::from_parts(parts, bytes);
                        Ok(wreq::Response::from(response))
                    });
                }
                Some(Body::Reusable(bytes)) => {
                    body.store(Some(Arc::new(Body::Reusable(bytes.clone()))));
                    let response = HttpResponse::from_parts(parts, bytes);
                    return Box::pin(future::ok(wreq::Response::from(response)));
                }
                None => unreachable!("Arc should never be empty here"),
            }
        }

        Box::pin(future::err(Error::Memory))
    }

    /// Consumes the response [`Body`] for streaming without caching.
    fn stream_response(&self) -> Result<wreq::Response, Error> {
        if let Some(arc) = self.body.swap(None)
            && let Ok(Body::Streamable(body)) = Arc::try_unwrap(arc)
        {
            return Ok(self.build_response(body));
        }
        Err(Error::Memory)
    }

    /// Forcefully destroys the response [`Body`], preventing any further reads.
    fn destroy(&self) {
        #[allow(clippy::option_map_unit_fn)]
        self.body
            .swap(None)
            .and_then(Arc::into_inner)
            .map(::std::mem::drop);
    }
}

#[pymethods]
impl Response {
    /// Get the URL of the response.
    #[getter]
    pub fn url(&self) -> String {
        self.uri.to_string()
    }

    /// Get the status code of the response.
    #[getter]
    pub fn status(&self) -> StatusCode {
        StatusCode(self.parts.status)
    }

    /// Get the HTTP version of the response.
    #[getter]
    pub fn version(&self) -> Version {
        Version::from_ffi(self.parts.version)
    }

    /// Get the headers of the response.
    #[getter]
    pub fn headers(&self) -> HeaderMap {
        HeaderMap(self.parts.headers.clone())
    }

    /// Get the cookies of the response.
    #[getter]
    pub fn cookies(&self) -> Vec<Cookie> {
        Cookie::extract_headers_cookies(&self.parts.headers)
    }

    /// Get the content length of the response.
    #[getter]
    pub fn content_length(&self, py: Python) -> Option<u64> {
        py.detach(|| self.empty_response().content_length())
    }

    /// Get the remote address of the response.
    #[getter]
    pub fn remote_addr(&self, py: Python) -> Option<SocketAddr> {
        py.detach(|| self.empty_response().remote_addr().map(SocketAddr))
    }

    /// Get the local address of the response.
    #[getter]
    pub fn local_addr(&self, py: Python) -> Option<SocketAddr> {
        py.detach(|| self.empty_response().local_addr().map(SocketAddr))
    }

    /// Get the redirect history of the Response.
    #[getter]
    pub fn history(&self, py: Python) -> Vec<History> {
        py.detach(|| {
            self.empty_response()
                .extensions()
                .get::<wreq::redirect::History>()
                .map_or_else(Vec::new, |history| {
                    history.into_iter().cloned().map(History).collect()
                })
        })
    }

    /// Get the TLS information of the response.
    #[getter]
    pub fn tls_info(&self, py: Python) -> Option<TlsInfo> {
        py.detach(|| {
            self.empty_response()
                .extensions()
                .get::<wreq::tls::TlsInfo>()
                .cloned()
                .map(TlsInfo)
        })
    }

    /// Turn a response into an error if the server returned an error.
    pub fn raise_for_status(&self) -> PyResult<()> {
        self.empty_response()
            .error_for_status()
            .map(|_| ())
            .map_err(Error::Library)
            .map_err(Into::into)
    }

    /// Get the response into a `Stream` of `Bytes` from the body.
    pub fn stream(&self) -> PyResult<Streamer> {
        self.stream_response()
            .map(Streamer::new)
            .map_err(Into::into)
    }

    /// Get the text content with the response encoding, defaulting to utf-8 when unspecified.
    #[pyo3(signature = (encoding = None))]
    pub async fn text(
        &self,
        #[pyo3(cancel_handle)] cancel: CancelHandle,
        encoding: Option<PyBackedStr>,
    ) -> PyResult<String> {
        let fut = self
            .cache_response()
            .and_then(|resp| ResponseExt::text(resp, encoding))
            .map_err(Into::into);
        NoGIL::new(fut, cancel).await
    }

    /// Get the JSON content of the response.
    pub async fn json(&self, #[pyo3(cancel_handle)] cancel: CancelHandle) -> PyResult<Json> {
        let fut = self
            .cache_response()
            .and_then(ResponseExt::json::<Json>)
            .map_err(Into::into);
        NoGIL::new(fut, cancel).await
    }

    /// Get the bytes content of the response.
    pub async fn bytes(&self, #[pyo3(cancel_handle)] cancel: CancelHandle) -> PyResult<PyBuffer> {
        let fut = self
            .cache_response()
            .and_then(ResponseExt::bytes)
            .map_ok(PyBuffer::from)
            .map_err(Into::into);
        NoGIL::new(fut, cancel).await
    }

    /// Close the response.
    ///
    /// This method closes the network connection regardless of whether connection pooling is
    /// enabled or not. It is recommended to use async context managers (`async with` statement)
    /// to properly manage response lifecycle instead of calling this method manually.
    pub async fn close(&self) {
        Python::attach(|py| {
            py.detach(|| {
                self.empty_response().forbid_recycle();
                self.destroy()
            });
        });
    }
}

#[pymethods]
impl Response {
    #[inline]
    async fn __aenter__(slf: Py<Self>) -> PyResult<Py<Self>> {
        Ok(slf)
    }

    #[inline]
    async fn __aexit__(&self, _exc_type: Py<PyAny>, _exc_val: Py<PyAny>, _traceback: Py<PyAny>) {
        self.close().await
    }
}

impl Display for Response {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "<{}({}) [{}] >",
            stringify!(Response),
            self.uri,
            self.parts.status,
        )
    }
}

impl Drop for Response {
    #[inline]
    fn drop(&mut self) {
        self.destroy();
    }
}

// ===== impl BlockingResponse =====

#[pymethods]
impl BlockingResponse {
    /// Get the URL of the response.
    #[getter]
    pub fn url(&self) -> String {
        self.0.url()
    }

    /// Get the status code of the response.
    #[getter]
    pub fn status(&self) -> StatusCode {
        self.0.status()
    }

    /// Get the HTTP version of the response.
    #[getter]
    pub fn version(&self) -> Version {
        self.0.version()
    }

    /// Get the headers of the response.
    #[getter]
    pub fn headers(&self) -> HeaderMap {
        self.0.headers()
    }

    /// Get the cookies of the response.
    #[getter]
    pub fn cookies(&self) -> Vec<Cookie> {
        self.0.cookies()
    }

    /// Get the content length of the response.
    #[getter]
    pub fn content_length(&self, py: Python) -> Option<u64> {
        self.0.content_length(py)
    }

    /// Get the remote address of the response.
    #[getter]
    pub fn remote_addr(&self, py: Python) -> Option<SocketAddr> {
        self.0.remote_addr(py)
    }

    /// Get the local address of the response.
    #[getter]
    pub fn local_addr(&self, py: Python) -> Option<SocketAddr> {
        self.0.local_addr(py)
    }

    /// Get the redirect history of the Response.
    #[getter]
    pub fn history(&self, py: Python) -> Vec<History> {
        self.0.history(py)
    }

    /// Get the TLS information of the response.
    #[getter]
    pub fn tls_info(&self, py: Python) -> Option<TlsInfo> {
        self.0.tls_info(py)
    }

    /// Turn a response into an error if the server returned an error.
    #[inline]
    pub fn raise_for_status(&self) -> PyResult<()> {
        self.0.raise_for_status()
    }

    /// Get the response into a `Stream` of `Bytes` from the body.
    #[inline]
    pub fn stream(&self) -> PyResult<Streamer> {
        self.0.stream()
    }

    /// Get the text content with the response encoding, defaulting to utf-8 when unspecified.
    #[pyo3(signature = (encoding = None))]
    pub fn text(&self, py: Python, encoding: Option<PyBackedStr>) -> PyResult<String> {
        py.detach(|| {
            let fut = self
                .0
                .cache_response()
                .and_then(|resp| ResponseExt::text(resp, encoding))
                .map_err(Into::into);
            pyo3_async_runtimes::tokio::get_runtime().block_on(fut)
        })
    }

    /// Get the JSON content of the response.
    pub fn json(&self, py: Python) -> PyResult<Json> {
        py.detach(|| {
            let fut = self
                .0
                .cache_response()
                .and_then(ResponseExt::json::<Json>)
                .map_err(Into::into);
            pyo3_async_runtimes::tokio::get_runtime().block_on(fut)
        })
    }

    /// Get the bytes content of the response.
    pub fn bytes(&self, py: Python) -> PyResult<PyBuffer> {
        py.detach(|| {
            let fut = self
                .0
                .cache_response()
                .and_then(ResponseExt::bytes)
                .map_ok(PyBuffer::from)
                .map_err(Into::into);
            pyo3_async_runtimes::tokio::get_runtime().block_on(fut)
        })
    }

    /// Close the response.
    ///
    /// This method closes the network connection regardless of whether connection pooling is
    /// enabled or not. It is recommended to use context managers (`with` statement) to properly
    /// manage response lifecycle instead of calling this method manually.
    #[inline]
    pub fn close(&self, py: Python) {
        py.detach(|| {
            self.0.empty_response().forbid_recycle();
            self.0.destroy();
        });
    }
}

#[pymethods]
impl BlockingResponse {
    #[inline]
    fn __enter__(slf: PyRef<Self>) -> PyRef<Self> {
        slf
    }

    #[inline]
    fn __exit__<'py>(
        &self,
        py: Python<'py>,
        _exc_type: &Bound<'py, PyAny>,
        _exc_value: &Bound<'py, PyAny>,
        _traceback: &Bound<'py, PyAny>,
    ) {
        self.close(py)
    }
}

impl From<Response> for BlockingResponse {
    #[inline]
    fn from(response: Response) -> Self {
        Self(response)
    }
}

impl Display for BlockingResponse {
    #[inline]
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

impl Drop for BlockingResponse {
    fn drop(&mut self) {
        self.0.destroy();
    }
}
