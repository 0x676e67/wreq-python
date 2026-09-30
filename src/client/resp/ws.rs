mod cmd;
pub mod msg;

use std::{fmt::Display, time::Duration};

use msg::Message;
use pyo3::{coroutine::CancelHandle, prelude::*};
use tokio::sync::mpsc;
use wreq::{
    header::HeaderValue,
    ws::{self, WebSocketResponse, message::Utf8Bytes},
};

use crate::{
    client::{SocketAddr, nogil::NoGIL},
    cookie::Cookie,
    error::Error,
    extractor::StrInput,
    header::HeaderMap,
    http::{StatusCode, Version},
    runtime::Runtime,
};

/// A WebSocket response.
#[pyclass(subclass, frozen, str)]
pub struct WebSocket {
    /// Returns the status code of the response.
    #[pyo3(get)]
    version: Version,

    /// Returns the HTTP version of the response.
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
    cmd: mpsc::UnboundedSender<cmd::Command>,
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
        let (cmd, rx) = mpsc::unbounded_channel();
        tokio::spawn(cmd::task(websocket, rx));

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
    pub fn cookies(&self, py: Python) -> Vec<Cookie> {
        py.detach(|| Cookie::extract_headers_cookies(&self.headers.0))
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
    pub async fn recv(
        &self,
        #[pyo3(cancel_handle)] cancel: CancelHandle,
        timeout: Option<Duration>,
    ) -> PyResult<Option<Message>> {
        let tx = self.cmd.clone();
        NoGIL::with_cancel(&self.runtime, cmd::recv(tx, timeout), cancel).await
    }

    /// Send a message to the WebSocket.
    #[pyo3(signature = (message))]
    pub async fn send(
        &self,
        #[pyo3(cancel_handle)] cancel: CancelHandle,
        message: Message,
    ) -> PyResult<()> {
        let tx = self.cmd.clone();
        NoGIL::with_cancel(&self.runtime, cmd::send(tx, message), cancel).await
    }

    /// Send multiple messages to the WebSocket.
    #[pyo3(signature = (messages))]
    pub async fn send_all(
        &self,
        #[pyo3(cancel_handle)] cancel: CancelHandle,
        messages: Vec<Message>,
    ) -> PyResult<()> {
        let tx = self.cmd.clone();
        NoGIL::with_cancel(&self.runtime, cmd::send_all(tx, messages), cancel).await
    }

    /// Close the WebSocket connection.
    #[pyo3(signature = (code=None, reason=None))]
    pub async fn close(
        &self,
        #[pyo3(cancel_handle)] cancel: CancelHandle,
        code: Option<u16>,
        reason: Option<StrInput>,
    ) -> PyResult<()> {
        let tx = self.cmd.clone();
        NoGIL::with_cancel(&self.runtime, cmd::close(tx, code, reason), cancel).await
    }
}

#[pymethods]
impl WebSocket {
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
        let tx = self.cmd.clone();
        NoGIL::new(&self.runtime, cmd::close(tx, None, None)).await
    }
}

impl Display for WebSocket {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
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
    pub fn cookies(&self, py: Python) -> Vec<Cookie> {
        self.0.cookies(py)
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
        py.detach(|| {
            crate::client::nogil::block_on(&self.0.runtime, cmd::recv(self.0.cmd.clone(), timeout))
        })
    }

    /// Send a message to the WebSocket.
    #[pyo3(signature = (message))]
    pub fn send(&self, py: Python, message: Message) -> PyResult<()> {
        py.detach(|| {
            crate::client::nogil::block_on(&self.0.runtime, cmd::send(self.0.cmd.clone(), message))
        })
    }

    /// Send multiple messages to the WebSocket.
    #[pyo3(signature = (messages))]
    pub fn send_all(&self, py: Python, messages: Vec<Message>) -> PyResult<()> {
        py.detach(|| {
            crate::client::nogil::block_on(
                &self.0.runtime,
                cmd::send_all(self.0.cmd.clone(), messages),
            )
        })
    }

    /// Close the WebSocket connection.
    #[pyo3(signature = (code=None, reason=None))]
    pub fn close(&self, py: Python, code: Option<u16>, reason: Option<StrInput>) -> PyResult<()> {
        py.detach(|| {
            crate::client::nogil::block_on(
                &self.0.runtime,
                cmd::close(self.0.cmd.clone(), code, reason),
            )
        })
    }
}

#[pymethods]
impl BlockingWebSocket {
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
    ) -> PyResult<()> {
        self.close(py, None, None)
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
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}
