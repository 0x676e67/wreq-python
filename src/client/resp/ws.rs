mod cmd;
pub mod msg;

use std::{
    fmt::{self, Display},
    time::Duration,
};

use msg::Message;
use pyo3::{prelude::*, sync::PyOnceLock};
use wreq::{header::HeaderValue, ws::WebSocketResponse};

use crate::{
    client::{SocketAddr, nogil},
    cookie::Cookie,
    coroutine::{self, Coroutine, EntersSelf},
    extractor::Text,
    header::HeaderMap,
    http::{StatusCode, Version},
    runtime::Runtime,
};

/// A WebSocket response.
#[pyclass(subclass, frozen, str)]
pub struct WebSocket {
    /// Returns the HTTP version of the response.
    #[pyo3(get)]
    version: Version,

    /// Returns the status code of the response.
    #[pyo3(get)]
    status: StatusCode,

    /// Returns the remote address of the response.
    #[pyo3(get)]
    remote_addr: Option<SocketAddr>,

    /// Returns the local address of the response.
    #[pyo3(get)]
    local_addr: Option<SocketAddr>,

    /// Returns the headers of the response.
    #[pyo3(get)]
    headers: HeaderMap,
    protocol: Option<HeaderValue>,
    cmd: cmd::Handle,
    runtime: Runtime,
}

/// A blocking WebSocket response.
#[pyclass(name = "WebSocket", subclass, frozen, str)]
pub struct BlockingWebSocket(WebSocket);

// ===== impl WebSocket =====

impl WebSocket {
    /// Creates a new [`WebSocket`] instance.
    pub async fn new(response: WebSocketResponse, runtime: Runtime) -> wreq::Result<WebSocket> {
        let (version, status, remote_addr, local_addr, headers) = (
            Version::from_ffi(response.version()),
            StatusCode(response.status()),
            response.remote_addr().map(SocketAddr),
            response.local_addr().map(SocketAddr),
            HeaderMap(response.headers().clone()),
        );
        let websocket = response.into_websocket().await?;
        let protocol = websocket.protocol().cloned();
        let cmd = cmd::spawn(websocket);

        Ok(WebSocket {
            runtime,
            version,
            status,
            remote_addr,
            local_addr,
            headers,
            protocol,
            cmd,
        })
    }
}

#[pymethods]
impl WebSocket {
    /// Returns the cookies of the response.
    #[getter]
    pub fn cookies(&self) -> Vec<Cookie> {
        Cookie::extract_headers_cookies(&self.headers.0)
    }

    /// Returns the WebSocket protocol.
    #[getter]
    pub fn protocol(&self) -> Option<&str> {
        self.protocol
            .as_ref()
            .map(HeaderValue::to_str)
            .transpose()
            .ok()
            .flatten()
    }

    /// Receive a message from the WebSocket.
    #[pyo3(signature = (timeout=None))]
    pub fn recv<'py>(
        &self,
        py: Python<'py>,
        timeout: Option<Duration>,
    ) -> PyResult<Bound<'py, Coroutine>> {
        coroutine::spawn(
            py,
            "WebSocket.recv",
            &self.runtime,
            cmd::recv(self.cmd.clone(), timeout),
        )
    }

    /// Send a message to the WebSocket.
    #[pyo3(signature = (message))]
    pub fn send<'py>(&self, py: Python<'py>, message: Message) -> PyResult<Bound<'py, Coroutine>> {
        coroutine::spawn(
            py,
            "WebSocket.send",
            &self.runtime,
            cmd::send(self.cmd.clone(), message),
        )
    }

    /// Send multiple messages to the WebSocket.
    #[pyo3(signature = (messages))]
    pub fn send_all<'py>(
        &self,
        py: Python<'py>,
        messages: Vec<Message>,
    ) -> PyResult<Bound<'py, Coroutine>> {
        coroutine::spawn(
            py,
            "WebSocket.send_all",
            &self.runtime,
            cmd::send_all(self.cmd.clone(), messages),
        )
    }

    /// Close the WebSocket connection.
    #[pyo3(signature = (code=None, reason=None))]
    pub fn close<'py>(
        &self,
        py: Python<'py>,
        code: Option<u16>,
        reason: Option<Text>,
    ) -> PyResult<Bound<'py, Coroutine>> {
        coroutine::spawn(
            py,
            "WebSocket.close",
            &self.runtime,
            cmd::close(self.cmd.clone(), code, reason),
        )
    }
}

#[pymethods]
impl WebSocket {
    fn __aenter__(slf: Bound<'_, Self>) -> PyResult<Bound<'_, Coroutine>> {
        coroutine::ready("WebSocket.__aenter__", slf)
    }

    /// Close the WebSocket connection without a close code or reason, unless already closed.
    fn __aexit__<'py>(
        &self,
        py: Python<'py>,
        _exc_type: Py<PyAny>,
        _exc_val: Py<PyAny>,
        _traceback: Py<PyAny>,
    ) -> PyResult<Bound<'py, Coroutine>> {
        coroutine::spawn(
            py,
            "WebSocket.__aexit__",
            &self.runtime,
            cmd::close_on_exit(self.cmd.clone()),
        )
    }
}

impl EntersSelf for WebSocket {
    fn native_aenter() -> &'static PyOnceLock<Py<PyAny>> {
        static NATIVE: PyOnceLock<Py<PyAny>> = PyOnceLock::new();
        &NATIVE
    }
}

impl Display for WebSocket {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "<{} [{}] >", stringify!(WebSocket), self.status.0)
    }
}

// ===== impl BlockingWebSocket =====

#[pymethods]
impl BlockingWebSocket {
    /// Returns the status code of the response.
    #[getter]
    pub fn status(&self) -> StatusCode {
        self.0.status
    }

    /// Returns the HTTP version of the response.
    #[getter]
    pub fn version(&self) -> Version {
        self.0.version
    }

    /// Returns the headers of the response.
    #[getter]
    pub fn headers(&self) -> HeaderMap {
        self.0.headers.clone()
    }

    /// Returns the cookies of the response.
    #[getter]
    pub fn cookies(&self) -> Vec<Cookie> {
        self.0.cookies()
    }

    /// Returns the remote address of the response.
    #[getter]
    pub fn remote_addr(&self) -> Option<SocketAddr> {
        self.0.remote_addr
    }

    /// Returns the local address of the response.
    #[getter]
    pub fn local_addr(&self) -> Option<SocketAddr> {
        self.0.local_addr
    }

    /// Returns the WebSocket protocol.
    #[getter]
    pub fn protocol(&self) -> Option<&str> {
        self.0.protocol()
    }

    /// Receive a message from the WebSocket.
    #[pyo3(signature = (timeout=None))]
    pub fn recv(&self, py: Python, timeout: Option<Duration>) -> PyResult<Option<Message>> {
        nogil::run(py, &self.0.runtime, cmd::recv(self.0.cmd.clone(), timeout))
    }

    /// Send a message to the WebSocket.
    #[pyo3(signature = (message))]
    pub fn send(&self, py: Python, message: Message) -> PyResult<()> {
        nogil::run(py, &self.0.runtime, cmd::send(self.0.cmd.clone(), message))
    }

    /// Send multiple messages to the WebSocket.
    #[pyo3(signature = (messages))]
    pub fn send_all(&self, py: Python, messages: Vec<Message>) -> PyResult<()> {
        nogil::run(
            py,
            &self.0.runtime,
            cmd::send_all(self.0.cmd.clone(), messages),
        )
    }

    /// Close the WebSocket connection.
    #[pyo3(signature = (code=None, reason=None))]
    pub fn close(&self, py: Python, code: Option<u16>, reason: Option<Text>) -> PyResult<()> {
        nogil::run(
            py,
            &self.0.runtime,
            cmd::close(self.0.cmd.clone(), code, reason),
        )
    }
}

#[pymethods]
impl BlockingWebSocket {
    fn __enter__(slf: PyRef<Self>) -> PyRef<Self> {
        slf
    }

    /// Close the WebSocket connection without a close code or reason, unless already closed.
    fn __exit__<'py>(
        &self,
        py: Python<'py>,
        _exc_type: &Bound<'py, PyAny>,
        _exc_value: &Bound<'py, PyAny>,
        _traceback: &Bound<'py, PyAny>,
    ) -> PyResult<()> {
        nogil::run(py, &self.0.runtime, cmd::close_on_exit(self.0.cmd.clone()))
    }
}

impl From<WebSocket> for BlockingWebSocket {
    #[inline]
    fn from(inner: WebSocket) -> Self {
        Self(inner)
    }
}

impl Display for BlockingWebSocket {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}
