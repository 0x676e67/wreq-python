use std::{
    fmt::{self, Display},
    future::Future,
    mem,
    sync::{Arc, Mutex, MutexGuard, PoisonError},
};

use bytes::Bytes;
use futures_util::{
    FutureExt, TryFutureExt,
    future::{self, BoxFuture},
};
use http::response::{Parts, Response as HttpResponse};
use http_body::Body as _;
use http_body_util::{BodyExt, Collected};
use pyo3::{prelude::*, pybacked::PyBackedStr};
use wreq::Uri;

use super::{ext::ResponseExt, stream::Streamer};
use crate::{
    aio::{self, Coroutine},
    buffer::PyBuffer,
    client::{SocketAddr, body::Json, nogil},
    cookie::Cookie,
    error::Error,
    header::HeaderMap,
    http::{StatusCode, Version},
    redirect::History,
    runtime::Runtime,
    tls::TlsInfo,
};

/// A response from a request.
///
/// Body reads are written once as futures ([`Response::read_body`]); the async methods
/// run them on the runtime when awaited and [`BlockingResponse`] waits on the caller.
#[pyclass(subclass, frozen, str, skip_from_py_object)]
pub struct Response {
    uri: Uri,
    /// Response head; rebuilt responses share its extensions, including the connection's
    /// reuse flag.
    parts: Parts,
    /// Shared with a read in flight, so it can cache the bytes it reads.
    body: Arc<Mutex<Body>>,
    /// Runs body reads and keeps the runtime alive while the response does.
    runtime: Runtime,
    /// Captured at receipt; `None` for a body without a known length.
    content_length: Option<u64>,
    remote_addr: Option<SocketAddr>,
    local_addr: Option<SocketAddr>,
}

/// The response body slot.
enum Body {
    /// Unread; taken by the first read or `stream()`.
    Unread(wreq::Body),
    /// Taken by a read in flight or by `stream()`.
    Taken,
    /// Read in full and shared by later `text`, `json` and `bytes` calls.
    Cached(Bytes),
    /// Released by `close` or a context exit; later reads fail.
    Released,
}

/// A blocking response from a request.
#[pyclass(name = "Response", subclass, frozen, str, skip_from_py_object)]
pub struct BlockingResponse(Response);

/// Forbids connection reuse on drop unless disarmed by taking the parts. Held while
/// [`Response::cache_response`] collects the body, so a failed or cancelled read is not
/// pooled.
struct RecycleGuard(Option<Parts>);

// ===== impl Response =====

impl Response {
    /// Bodies up to this size are read on a blocking caller without first releasing the GIL.
    const READ_ATTACHED: u64 = 64 * 1024;

    /// Create a new [`Response`] instance.
    pub fn new(response: wreq::Response, runtime: Runtime) -> Self {
        let uri = response.uri().clone();
        let content_length = response.content_length();
        let remote_addr = response.remote_addr().map(SocketAddr);
        let local_addr = response.local_addr().map(SocketAddr);
        let (parts, body) = HttpResponse::from(response).into_parts();
        Response {
            uri,
            parts,
            body: Arc::new(Mutex::new(Body::Unread(body))),
            runtime,
            content_length,
            remote_addr,
            local_addr,
        }
    }

    #[inline]
    fn slot(&self) -> MutexGuard<'_, Body> {
        lock(&self.body)
    }

    /// Builds a [`wreq::Response`] from the current response metadata and the given body.
    #[inline]
    fn build_response<T: Into<wreq::Body>>(&self, body: T) -> wreq::Response {
        let response = HttpResponse::from_parts(self.parts.clone(), body);
        wreq::Response::from(response)
    }

    /// Take the body and return a future that reads it in full, caching the bytes for later
    /// reads; a cached body is shared at once. While a first read runs, overlapping reads and
    /// `stream()` fail with [`Error::Memory`], as do later ones if it fails or is dropped.
    fn cache_response(&self) -> BoxFuture<'static, Result<wreq::Response, Error>> {
        let mut slot = self.slot();
        let stream = match mem::replace(&mut *slot, Body::Taken) {
            Body::Unread(stream) => stream,
            other => {
                let cached = match &other {
                    Body::Cached(bytes) => Some(bytes.clone()),
                    _ => None,
                };
                *slot = other;
                drop(slot);
                let response = cached.map(|bytes| self.build_response(bytes));
                return future::ready(response.ok_or(Error::Memory)).boxed();
            }
        };
        drop(slot);
        let parts = self.parts.clone();
        let body = self.body.clone();
        async move {
            // Keep the connection out of the pool unless the body is read in full.
            let mut guard = RecycleGuard(Some(parts));
            let bytes = stream
                .collect()
                .await
                .map(Collected::to_bytes)
                .map_err(Error::Library)?;
            let parts = guard.0.take().ok_or(Error::Memory)?;
            // A release during the read wins over caching.
            let mut slot = lock(&body);
            if let Body::Taken = *slot {
                *slot = Body::Cached(bytes.clone());
            }
            drop(slot);
            Ok(wreq::Response::from(HttpResponse::from_parts(parts, bytes)))
        }
        .boxed()
    }

    /// Take the unread body for a [`Streamer`]; fails with [`Error::Memory`] otherwise,
    /// leaving any cached bytes readable.
    fn stream_response(&self) -> Result<wreq::Response, Error> {
        let mut slot = self.slot();
        let body = match mem::replace(&mut *slot, Body::Taken) {
            Body::Unread(body) => body,
            other => {
                *slot = other;
                return Err(Error::Memory);
            }
        };
        drop(slot);
        Ok(self.build_response(body))
    }

    /// Read the body with `read`: the body is taken now and its bytes cached for later reads.
    fn read_body<F, Fut, T>(&self, read: F) -> impl Future<Output = PyResult<T>> + Send + 'static
    where
        F: FnOnce(wreq::Response) -> Fut + Send + 'static,
        Fut: Future<Output = Result<T, Error>> + Send + 'static,
    {
        self.cache_response().and_then(read).map_err(Into::into)
    }

    /// Read the body on the runtime once awaited; the body is taken on first await.
    fn read<'py, F, Fut, T>(
        slf: Bound<'py, Self>,
        qualname: &'static str,
        read: F,
    ) -> PyResult<Bound<'py, Coroutine>>
    where
        F: FnOnce(wreq::Response) -> Fut + Send + 'static,
        Fut: Future<Output = Result<T, Error>> + Send + 'static,
        T: for<'a> IntoPyObject<'a> + Send + 'static,
    {
        let py = slf.py();
        let slf = slf.unbind();
        aio::local(py, qualname, async move {
            let this = slf.get();
            aio::run(this.runtime.clone(), this.read_body(read)).await
        })
    }

    /// Whether the body is small enough for a blocking read to finish without first
    /// releasing the GIL. An unknown length, as of a chunked or decompressed body, is not:
    /// decoding everything already buffered could hold the GIL for long.
    fn read_attached(&self) -> bool {
        match &*self.slot() {
            Body::Unread(body) => body
                .size_hint()
                .exact()
                .is_some_and(|len| len <= Self::READ_ATTACHED),
            Body::Cached(bytes) => bytes.len() as u64 <= Self::READ_ATTACHED,
            Body::Taken | Body::Released => true,
        }
    }

    /// Keep the connection out of the pool; a rebuilt response shares its reuse flag.
    fn forbid_recycle(&self) {
        let mut response = HttpResponse::new(Bytes::new());
        *response.extensions_mut() = self.parts.extensions.clone();
        wreq::Response::from(response).forbid_recycle();
    }

    /// Drop the body if still held, so later reads fail. Unlike `close`, this keeps a fully
    /// read connection reusable.
    #[inline]
    fn destroy(&self) {
        let body = mem::replace(&mut *self.slot(), Body::Released);
        drop(body);
    }

    /// Discard the body and keep its connection out of the pool.
    fn discard(&self) {
        self.forbid_recycle();
        self.destroy();
    }
}

/// Lock a body slot, recovering it from a panicked holder.
#[inline]
fn lock(body: &Mutex<Body>) -> MutexGuard<'_, Body> {
    body.lock().unwrap_or_else(PoisonError::into_inner)
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
    pub fn content_length(&self) -> Option<u64> {
        self.content_length
    }

    /// Get the remote address of the response.
    #[getter]
    pub fn remote_addr(&self) -> Option<SocketAddr> {
        self.remote_addr
    }

    /// Get the local address of the response.
    #[getter]
    pub fn local_addr(&self) -> Option<SocketAddr> {
        self.local_addr
    }

    /// Get the redirect history of the Response.
    #[getter]
    pub fn history(&self) -> Vec<History> {
        self.parts
            .extensions
            .get::<wreq::redirect::History>()
            .map_or_else(Vec::new, |history| {
                history.into_iter().cloned().map(History).collect()
            })
    }

    /// Get the TLS information of the response.
    #[getter]
    pub fn tls_info(&self) -> Option<TlsInfo> {
        self.parts
            .extensions
            .get::<wreq::tls::TlsInfo>()
            .cloned()
            .map(TlsInfo)
    }

    /// Turn a response into an error if the server returned an error.
    pub fn raise_for_status(&self) -> PyResult<()> {
        let status = self.parts.status;
        if !status.is_client_error() && !status.is_server_error() {
            return Ok(());
        }
        self.build_response(Bytes::new())
            .error_for_status()
            .map(|_| ())
            .map_err(Error::Library)
            .map_err(Into::into)
    }

    /// Stream read-only memoryviews and any trailing headers from the body.
    pub fn stream(&self) -> PyResult<Streamer> {
        self.stream_response()
            .map(|response| Streamer::new(response, self.runtime.clone()))
            .map_err(Into::into)
    }

    /// Get the text content with the response encoding, defaulting to utf-8 when unspecified.
    #[pyo3(signature = (encoding = None))]
    pub fn text(
        slf: Bound<'_, Self>,
        encoding: Option<PyBackedStr>,
    ) -> PyResult<Bound<'_, Coroutine>> {
        Self::read(slf, "Response.text", |resp| {
            ResponseExt::text(resp, encoding)
        })
    }

    /// Get the JSON content of the response.
    pub fn json(slf: Bound<'_, Self>) -> PyResult<Bound<'_, Coroutine>> {
        Self::read(slf, "Response.json", ResponseExt::json::<Json>)
    }

    /// Read the body as a read-only memoryview, retaining its data after the response closes.
    pub fn bytes(slf: Bound<'_, Self>) -> PyResult<Bound<'_, Coroutine>> {
        Self::read(slf, "Response.bytes", ResponseExt::bytes)
    }

    /// Discard the retained body and mark its connection as non-reusable.
    /// This does not guarantee an immediate socket shutdown or cancel an active read.
    /// Cancel and await any body-read task before closing. A body transferred to a
    /// Streamer is managed separately; previously returned memoryviews remain valid.
    /// `async with` instead releases the body and keeps a fully read connection reusable.
    pub fn close(slf: Bound<'_, Self>) -> PyResult<Bound<'_, Coroutine>> {
        let py = slf.py();
        let slf = slf.unbind();
        aio::local(py, "Response.close", async move {
            slf.get().discard();
            Ok(())
        })
    }
}

#[pymethods]
impl Response {
    fn __aenter__(slf: Bound<'_, Self>) -> PyResult<Bound<'_, Coroutine>> {
        aio::ready("Response.__aenter__", slf)
    }

    /// Release the body without forbidding reuse: a fully read connection returns
    /// to the pool, while an unread HTTP/1 body drains or closes its connection.
    fn __aexit__<'py>(
        slf: Bound<'py, Self>,
        _exc_type: Py<PyAny>,
        _exc_val: Py<PyAny>,
        _traceback: Py<PyAny>,
    ) -> PyResult<Bound<'py, Coroutine>> {
        let py = slf.py();
        let slf = slf.unbind();
        aio::local(py, "Response.__aexit__", async move {
            slf.get().destroy();
            Ok(())
        })
    }
}

impl Display for Response {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
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

// ===== impl RecycleGuard =====

impl Drop for RecycleGuard {
    fn drop(&mut self) {
        if let Some(parts) = self.0.take() {
            wreq::Response::from(HttpResponse::from_parts(parts, Bytes::new())).forbid_recycle();
        }
    }
}

// ===== impl BlockingResponse =====

impl BlockingResponse {
    /// Read the body with `read` on the calling thread, like [`Response::read`] on the
    /// runtime. A small body is read without releasing the GIL unless it must wait.
    fn read<F, Fut, T>(&self, py: Python, read: F) -> PyResult<T>
    where
        F: FnOnce(wreq::Response) -> Fut + Send + 'static,
        Fut: Future<Output = Result<T, Error>> + Send + 'static,
        T: Send,
    {
        let (response, runtime) = (&self.0, &self.0.runtime);
        // Decide before the read takes the body.
        let attached = response.read_attached();
        let fut = response.read_body(read);
        if attached {
            nogil::run(py, runtime, fut)
        } else {
            py.detach(|| runtime.handle().block_on(fut))
        }
    }
}

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
    pub fn content_length(&self) -> Option<u64> {
        self.0.content_length()
    }

    /// Get the remote address of the response.
    #[getter]
    pub fn remote_addr(&self) -> Option<SocketAddr> {
        self.0.remote_addr()
    }

    /// Get the local address of the response.
    #[getter]
    pub fn local_addr(&self) -> Option<SocketAddr> {
        self.0.local_addr()
    }

    /// Get the redirect history of the Response.
    #[getter]
    pub fn history(&self) -> Vec<History> {
        self.0.history()
    }

    /// Get the TLS information of the response.
    #[getter]
    pub fn tls_info(&self) -> Option<TlsInfo> {
        self.0.tls_info()
    }

    /// Turn a response into an error if the server returned an error.
    pub fn raise_for_status(&self) -> PyResult<()> {
        self.0.raise_for_status()
    }

    /// Stream read-only memoryviews and any trailing headers from the body.
    pub fn stream(&self) -> PyResult<Streamer> {
        self.0.stream()
    }

    /// Get the text content with the response encoding, defaulting to utf-8 when unspecified.
    #[pyo3(signature = (encoding = None))]
    pub fn text(&self, py: Python, encoding: Option<PyBackedStr>) -> PyResult<String> {
        self.read(py, |resp| ResponseExt::text(resp, encoding))
    }

    /// Get the JSON content of the response.
    pub fn json(&self, py: Python) -> PyResult<Json> {
        self.read(py, ResponseExt::json::<Json>)
    }

    /// Read the body as a read-only memoryview, retaining its data after the response closes.
    pub fn bytes(&self, py: Python) -> PyResult<PyBuffer> {
        self.read(py, ResponseExt::bytes)
    }

    /// Discard the retained body and mark its connection as non-reusable.
    /// This does not guarantee an immediate socket shutdown or interrupt an active read.
    /// Do not close concurrently with a body read. A body transferred to a Streamer
    /// is managed separately; previously returned memoryviews remain valid.
    /// `with` instead releases the body and keeps a fully read connection reusable.
    pub fn close(&self) {
        self.0.discard();
    }
}

#[pymethods]
impl BlockingResponse {
    fn __enter__(slf: PyRef<Self>) -> PyRef<Self> {
        slf
    }

    /// Release the body without forbidding reuse: a fully read connection returns
    /// to the pool, while an unread HTTP/1 body drains or closes its connection.
    fn __exit__<'py>(
        &self,
        _exc_type: &Bound<'py, PyAny>,
        _exc_value: &Bound<'py, PyAny>,
        _traceback: &Bound<'py, PyAny>,
    ) {
        self.0.destroy();
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
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}
