use std::{
    any::Any,
    fmt,
    panic::{self, AssertUnwindSafe},
};

use pyo3::{
    PyErr, Python, create_exception,
    exceptions::{PyException, PyRuntimeError, PyStopAsyncIteration, PyStopIteration},
};
use wreq::header;

const RACE_CONDITION_ERROR_MSG: &str = r#"Due to Rust's memory management with borrowing,
you cannot use certain instances multiple times as they may be consumed.

This error can occur in the following cases:
1) You passed a non-clonable instance to a function that requires ownership.
2) You attempted to use a method that consumes ownership more than once (e.g., reading a response body twice).
3) You tried to reference an instance after it was borrowed.

Potential solutions:
1) Avoid sharing instances; create a new instance each time you use it.
2) Refrain from performing actions that consume ownership multiple times.
3) Change the order of operations to reference the instance before borrowing it.
"#;

// System-level and runtime errors
create_exception!(exceptions, RustPanic, PyException);

// Network connection errors
create_exception!(exceptions, ConnectionError, PyException);
create_exception!(exceptions, ProxyConnectionError, PyException);
create_exception!(exceptions, ConnectionResetError, PyException);
create_exception!(exceptions, TlsError, PyException);

// HTTP protocol and request/response errors
create_exception!(exceptions, RequestError, PyException);
create_exception!(exceptions, StatusError, PyException);
create_exception!(exceptions, RedirectError, PyException);
create_exception!(exceptions, TimeoutError, PyException);

// Data processing and encoding errors
create_exception!(exceptions, BodyError, PyException);
create_exception!(exceptions, DecodingError, PyException);

// Configuration and builder errors
create_exception!(exceptions, BuilderError, PyException);

// Protocol upgrade and WebSocket errors
create_exception!(exceptions, UpgradeError, PyException);
create_exception!(exceptions, WebSocketError, PyException);

macro_rules! wrap_error {
    ($error:expr, $($variant:ident => $exception:ident),*) => {
        {
            $(
                if $error.$variant() {
                    return $exception::new_err(format!(concat!(stringify!($variant), " error: {:?}"), $error));
                }
            )*
            UpgradeError::new_err(format!("error: {:?}", $error))
        }
    };
}

const INTERPRETER_UNAVAILABLE_MSG: &str =
    "The Python interpreter is not available (not initialized or shutting down)";

/// Unified error enum
#[derive(Debug)]
pub enum Error {
    Memory,
    StopIteration,
    StopAsyncIteration,
    WebSocketDisconnected,
    InvalidHeaderName(header::InvalidHeaderName),
    InvalidHeaderValue(header::InvalidHeaderValue),
    Timeout(tokio::time::error::Elapsed),
    Builder(http::Error),
    Dns(hickory_resolver::net::NetError),
    IO(std::io::Error),
    Decode(cookie::ParseError),
    Json(serde_json::Error),
    Form(serde_urlencoded::ser::Error),
    Library(wreq::Error),
    InterpreterUnavailable,
    Panic(Box<str>),
}

impl From<Error> for PyErr {
    fn from(err: Error) -> Self {
        match err {
            Error::Memory => PyRuntimeError::new_err(RACE_CONDITION_ERROR_MSG),
            Error::StopIteration => PyStopIteration::new_err("The iterator is exhausted"),
            Error::StopAsyncIteration => {
                PyStopAsyncIteration::new_err("The async iterator is exhausted")
            }
            Error::WebSocketDisconnected => {
                PyRuntimeError::new_err("The WebSocket has been disconnected")
            }
            Error::InvalidHeaderName(err) => {
                PyRuntimeError::new_err(format!("Invalid header name: {err:?}"))
            }
            Error::InvalidHeaderValue(err) => {
                PyRuntimeError::new_err(format!("Invalid header value: {err:?}"))
            }
            Error::Timeout(err) => TimeoutError::new_err(format!("Timeout error: {err:?}")),
            Error::IO(err) => PyRuntimeError::new_err(format!("IO error: {err:?}")),
            Error::Decode(err) => DecodingError::new_err(format!("Decode error: {err:?}")),
            Error::Builder(err) => BuilderError::new_err(format!("Builder error: {err:?}")),
            Error::Dns(err) => BuilderError::new_err(format!("DNS resolver error: {err:?}")),
            Error::Json(err) => PyRuntimeError::new_err(format!("JSON error: {err:?}")),
            Error::Form(err) => PyRuntimeError::new_err(format!("Form error: {err:?}")),
            Error::InterpreterUnavailable => PyRuntimeError::new_err(INTERPRETER_UNAVAILABLE_MSG),
            Error::Panic(msg) => RustPanic::new_err(msg.into_string()),
            Error::Library(err) => wrap_error!(err,
                is_body => BodyError,
                is_tls => TlsError,
                is_websocket => WebSocketError,
                is_connect => ConnectionError,
                is_proxy_connect => ProxyConnectionError,
                is_connection_reset => ConnectionResetError,
                is_decode => DecodingError,
                is_redirect => RedirectError,
                is_timeout => TimeoutError,
                is_status => StatusError,
                is_request => RequestError,
                is_builder => BuilderError
            ),
        }
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::InterpreterUnavailable => f.write_str(INTERPRETER_UNAVAILABLE_MSG),
            Error::Panic(msg) => f.write_str(msg),
            err => fmt::Debug::fmt(err, f),
        }
    }
}

impl Error {
    fn panic(payload: Box<dyn Any + Send>) -> Self {
        let msg = payload
            .downcast_ref::<&str>()
            .copied()
            .or_else(|| payload.downcast_ref::<String>().map(String::as_str))
            .unwrap_or("unknown panic");
        Error::Panic(format!("Rust panic: {msg}").into_boxed_str())
    }
}

/// Attaches to Python from a non-Python thread.
///
/// `Python::attach` panics once the interpreter has shut down (see discussions/305), so
/// this uses `Python::try_attach` and returns [`Error::InterpreterUnavailable`] instead.
/// Any other panic in `f` is caught as [`Error::Panic`] to keep Tokio threads alive.
pub fn attach<F, R>(f: F) -> Result<R, Error>
where
    F: for<'py> FnOnce(Python<'py>) -> R,
{
    panic::catch_unwind(AssertUnwindSafe(|| Python::try_attach(f)))
        .map_err(Error::panic)?
        .ok_or(Error::InterpreterUnavailable)
}

impl From<header::InvalidHeaderName> for Error {
    fn from(err: header::InvalidHeaderName) -> Self {
        Error::InvalidHeaderName(err)
    }
}

impl From<header::InvalidHeaderValue> for Error {
    fn from(err: header::InvalidHeaderValue) -> Self {
        Error::InvalidHeaderValue(err)
    }
}

impl From<std::io::Error> for Error {
    fn from(err: std::io::Error) -> Self {
        Error::IO(err)
    }
}

impl From<wreq::Error> for Error {
    fn from(err: wreq::Error) -> Self {
        Error::Library(err)
    }
}

impl From<tokio::time::error::Elapsed> for Error {
    fn from(err: tokio::time::error::Elapsed) -> Self {
        Error::Timeout(err)
    }
}
