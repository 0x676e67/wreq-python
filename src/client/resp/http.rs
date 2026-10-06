use std::{
    fmt::{self, Display},
    future::Future,
    mem,
    sync::{Arc, Mutex, MutexGuard, PoisonError},
};

use bytes::Bytes;
use futures_util::{FutureExt, TryFutureExt, future::Either};
use http::response::{Parts, Response as HttpResponse};
use http_body::Body as _;
use http_body_util::{BodyExt, Collected, combinators::Collect};
use pyo3::{prelude::*, pybacked::PyBackedStr, sync::PyOnceLock};
use wreq::Uri;

use super::{READ_ATTACHED, ext::ResponseExt, loop_limit, stream::Streamer};
use crate::{
    buffer::PyBuffer,
    client::{SocketAddr, body::Json, nogil},
    cookie::Cookie,
    coroutine::{self, Coroutine, EntersSelf},
    error::Error,
    header::HeaderMap,
    http::{StatusCode, Version},
    redirect::History,
    runtime::Runtime,
    tls::TlsInfo,
};

/// A response from a request.
///
/// Reads take the body with [`Response::take_bytes`]: a buffered body of known length up to
/// the read's limit finishes on the caller, the event loop or a blocking thread, and any other
/// body is collected on the runtime by [`Response::collect_later`].
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

/// A body taken for reading.
enum BodyRead {
    /// Read in full now.
    Ready(Bytes),
    /// Not read now: still arriving, over the read's limit or of unknown length; collected
    /// on the runtime by [`Response::collect_later`].
    Pending(Collect<wreq::Body>),
}

/// A blocking response from a request.
#[pyclass(name = "Response", subclass, frozen, str, skip_from_py_object)]
pub struct BlockingResponse(Response);

/// Forbids connection reuse on drop unless disarmed by taking the parts. Held by
/// [`Response::collect_later`] from its creation until the body is read in full, so a failed
/// or cancelled read is not pooled.
struct RecycleGuard(Option<Parts>);

// ===== impl Response =====

impl Response {
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

    /// Take the body for reading. Cached bytes are shared at once, and a body of known
    /// length up to `limit` that is already buffered is read now on the calling thread, none
    /// without a `limit`; overlapping reads and `stream()` fail with [`Error::Memory`] while
    /// a read runs.
    fn take_bytes(&self, limit: Option<u64>) -> Result<BodyRead, Error> {
        let mut slot = self.slot();
        let body = match mem::replace(&mut *slot, Body::Taken) {
            Body::Unread(body) => body,
            Body::Cached(bytes) => {
                *slot = Body::Cached(bytes.clone());
                return Ok(BodyRead::Ready(bytes));
            }
            other => {
                *slot = other;
                return Err(Error::Memory);
            }
        };
        drop(slot);
        let attached = body
            .size_hint()
            .exact()
            .zip(limit)
            .is_some_and(|(len, limit)| len <= limit);
        let mut collect = body.collect();
        let ready = if attached {
            // A read timeout starts a timer on its first poll, which needs the runtime.
            let _runtime = self.runtime.handle().enter();
            (&mut collect).now_or_never()
        } else {
            None
        };
        match ready {
            Some(Ok(collected)) => {
                let bytes = collected.to_bytes();
                cache(&self.body, &bytes);
                Ok(BodyRead::Ready(bytes))
            }
            Some(Err(err)) => {
                self.forbid_recycle();
                Err(Error::Library(err))
            }
            None => Ok(BodyRead::Pending(collect)),
        }
    }

    /// Collect a taken body, caching its bytes and returning them with a copy of the head.
    /// The connection stays out of the pool unless the body is read in full, including
    /// when the future is dropped before its first poll.
    fn collect_later(
        &self,
        collect: Collect<wreq::Body>,
    ) -> impl Future<Output = Result<(Parts, Bytes), Error>> + Send + 'static {
        let mut guard = RecycleGuard(Some(self.parts.clone()));
        let body = self.body.clone();
        async move {
            let bytes = collect
                .await
                .map(Collected::to_bytes)
                .map_err(Error::Library)?;
            let parts = guard.0.take().ok_or(Error::Memory)?;
            cache(&body, &bytes);
            Ok((parts, bytes))
        }
    }

    /// Take the body and return a future decoding it with `read`, and whether that future
    /// finishes at once: the bytes, cached or already buffered, are at most `limit`. Otherwise
    /// the body is still arriving or too large to decode on the caller.
    fn read_body<F, Fut, T>(
        &self,
        limit: Option<u64>,
        read: F,
    ) -> Result<(impl Future<Output = PyResult<T>> + Send + 'static, bool), Error>
    where
        F: FnOnce(wreq::Response) -> Fut + Send + 'static,
        Fut: Future<Output = Result<T, Error>> + Send + 'static,
    {
        let (decode, inline) = match self.take_bytes(limit)? {
            BodyRead::Ready(bytes) => {
                let inline = limit.is_some_and(|limit| bytes.len() as u64 <= limit);
                (Either::Left(read(self.build_response(bytes))), inline)
            }
            BodyRead::Pending(collect) => (
                Either::Right(self.collect_later(collect).and_then(|(parts, bytes)| {
                    read(wreq::Response::from(HttpResponse::from_parts(parts, bytes)))
                })),
                false,
            ),
        };
        Ok((decode.map_err(Into::into), inline))
    }

    /// Read and decode the body once awaited; the body is taken on first await. Bytes that
    /// [`read_body`](Self::read_body) can decode at once are decoded on the event loop.
    fn read<'py, F, Fut, T>(
        slf: Bound<'py, Self>,
        qualname: &'static str,
        limit: u64,
        read: F,
    ) -> PyResult<Bound<'py, Coroutine>>
    where
        F: FnOnce(wreq::Response) -> Fut + Send + 'static,
        Fut: Future<Output = Result<T, Error>> + Send + 'static,
        T: for<'a> IntoPyObject<'a> + Send + 'static,
    {
        let py = slf.py();
        let slf = slf.unbind();
        coroutine::local(py, qualname, async move {
            let this = slf.get();
            let (read, inline) = this.read_body(loop_limit(this.parts.version, limit), read)?;
            if inline {
                read.await
            } else {
                coroutine::run(this.runtime.clone(), read).await
            }
        })
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

/// Cache the bytes of a finished read; a release during the read wins over caching.
fn cache(body: &Mutex<Body>, bytes: &Bytes) {
    let mut slot = lock(body);
    if let Body::Taken = *slot {
        *slot = Body::Cached(bytes.clone());
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

    /// Stream read-only memoryviews and any trailing headers from the body. Only an unread
    /// body can be streamed; cached bytes stay readable.
    pub fn stream(&self) -> PyResult<Streamer> {
        let mut slot = self.slot();
        let body = match mem::replace(&mut *slot, Body::Taken) {
            Body::Unread(body) => body,
            other => {
                *slot = other;
                return Err(Error::Memory.into());
            }
        };
        drop(slot);
        Ok(Streamer::new(
            self.build_response(body),
            self.runtime.clone(),
        ))
    }

    /// Get the text content with the response encoding, defaulting to utf-8 when unspecified.
    #[pyo3(signature = (encoding = None))]
    pub fn text(
        slf: Bound<'_, Self>,
        encoding: Option<PyBackedStr>,
    ) -> PyResult<Bound<'_, Coroutine>> {
        Self::read(slf, "Response.text", READ_ATTACHED, |resp| {
            ResponseExt::text(resp, encoding)
        })
    }

    /// Get the JSON content of the response.
    pub fn json(slf: Bound<'_, Self>) -> PyResult<Bound<'_, Coroutine>> {
        // Buffered HTTP/1 JSON up to 8 KiB is parsed on the event loop; see `loop_limit`.
        Self::read(slf, "Response.json", 8 * 1024, ResponseExt::json::<Json>)
    }

    /// Read the body as a read-only memoryview, retaining its data after the response closes.
    pub fn bytes(slf: Bound<'_, Self>) -> PyResult<Bound<'_, Coroutine>> {
        let py = slf.py();
        let slf = slf.unbind();
        coroutine::local(py, "Response.bytes", async move {
            let this = slf.get();
            match this.take_bytes(loop_limit(this.parts.version, READ_ATTACHED))? {
                BodyRead::Ready(bytes) => Ok(PyBuffer::from(bytes)),
                BodyRead::Pending(collect) => {
                    let read = this
                        .collect_later(collect)
                        .map_ok(|(_, bytes)| PyBuffer::from(bytes))
                        .map_err(Into::into);
                    coroutine::run(this.runtime.clone(), read).await
                }
            }
        })
    }

    /// Discard the retained body and mark its connection as non-reusable.
    /// This does not guarantee an immediate socket shutdown or cancel an active read.
    /// Cancel and await any body-read task before closing. A body transferred to a
    /// Streamer is managed separately; previously returned memoryviews remain valid.
    /// `async with` instead releases the body and keeps a fully read connection reusable.
    pub fn close(slf: Bound<'_, Self>) -> PyResult<Bound<'_, Coroutine>> {
        let py = slf.py();
        let slf = slf.unbind();
        coroutine::local(py, "Response.close", async move {
            slf.get().discard();
            Ok(())
        })
    }
}

#[pymethods]
impl Response {
    fn __aenter__(slf: Bound<'_, Self>) -> PyResult<Bound<'_, Coroutine>> {
        coroutine::ready("Response.__aenter__", slf)
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
        coroutine::local(py, "Response.__aexit__", async move {
            slf.get().destroy();
            Ok(())
        })
    }
}

impl EntersSelf for Response {
    fn native_aenter() -> &'static PyOnceLock<Py<PyAny>> {
        static NATIVE: PyOnceLock<Py<PyAny>> = PyOnceLock::new();
        &NATIVE
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
    /// Read the body with `read` on the calling thread. Bytes that
    /// [`Response::read_body`] can decode at once are decoded without releasing the GIL.
    fn read<F, Fut, T>(&self, py: Python, read: F) -> PyResult<T>
    where
        F: FnOnce(wreq::Response) -> Fut + Send + 'static,
        Fut: Future<Output = Result<T, Error>> + Send + 'static,
        T: Send,
    {
        let runtime = &self.0.runtime;
        let (read, inline) = self.0.read_body(Some(READ_ATTACHED), read)?;
        if inline {
            nogil::run(py, runtime, read)
        } else {
            py.detach(|| runtime.handle().block_on(read))
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
        let response = &self.0;
        match response.take_bytes(Some(READ_ATTACHED))? {
            BodyRead::Ready(bytes) => Ok(PyBuffer::from(bytes)),
            BodyRead::Pending(collect) => {
                let read = response
                    .collect_later(collect)
                    .map_ok(|(_, bytes)| PyBuffer::from(bytes))
                    .map_err(Into::into);
                py.detach(|| response.runtime.handle().block_on(read))
            }
        }
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
