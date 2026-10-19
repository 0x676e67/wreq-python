use std::{error::Error as StdError, fmt, io, iter};

use hickory_resolver::net::NetError;
use pyo3::{
    PyErrArguments,
    exceptions::{PyImportError, PyRuntimeError, PyStopAsyncIteration, PyStopIteration},
    intern,
    prelude::*,
    types::{PyFrozenSet, PyType},
};
use tokio::time::error::Elapsed;
use wreq::header;

use crate::http::StatusCode;

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

/// Exception types defined in `wreq/exceptions.py`, which also holds their hierarchy.
mod exceptions {
    use pyo3::import_exception;

    import_exception!(wreq.exceptions, Error);
    import_exception!(wreq.exceptions, BuilderError);
    import_exception!(wreq.exceptions, TlsError);
    import_exception!(wreq.exceptions, RequestError);
    import_exception!(wreq.exceptions, ConnectionError);
    import_exception!(wreq.exceptions, ProxyConnectionError);
    import_exception!(wreq.exceptions, ConnectionResetError);
    import_exception!(wreq.exceptions, TimeoutError);
    import_exception!(wreq.exceptions, BodyError);
    import_exception!(wreq.exceptions, DecodingError);
    import_exception!(wreq.exceptions, RedirectError);
    import_exception!(wreq.exceptions, StatusError);
    import_exception!(wreq.exceptions, WebSocketError);
    import_exception!(wreq.exceptions, UpgradeError);
}

/// Load the exception classes and check that each [`Kind`] reaches the `is_*` method
/// that reads it, so a broken install or a renamed method fails the import instead of
/// the first error.
pub fn init(py: Python<'_>) -> PyResult<()> {
    for kind in Kind::ALL {
        let method = format!("is_{}", kind.name());
        let error = Details::new(kind, String::new()).instantiate(py)?;
        if !error.bind(py).call_method0(method.as_str())?.is_truthy()? {
            return Err(PyImportError::new_err(format!(
                "wreq.exceptions.Error.{method}() does not read {:?}",
                kind.name()
            )));
        }
    }
    Ok(())
}

// ===== impl Kind =====

/// Defines [`Kind`] from a table of the name an `is_*` method of `wreq.exceptions.Error`
/// reads, the [`wreq::Error`] predicate behind it, and the exception class it selects.
macro_rules! define_kinds {
    ($($kind:ident($name:literal, $predicate:ident) $(=> $exception:ident)?),* $(,)?) => {
        /// A fact about a failure, one per `is_*` method of `wreq.exceptions.Error`.
        #[derive(Clone, Copy)]
        enum Kind {
            $($kind),*
        }

        impl Kind {
            /// Every kind, in the order that selects the exception class.
            const ALL: [Kind; [$($name),*].len()] = [$(Kind::$kind),*];

            /// The name the `is_*` method reads.
            const fn name(self) -> &'static str {
                match self {
                    $(Kind::$kind => $name),*
                }
            }

            /// Whether wreq reports this kind for `error`.
            fn matches(self, error: &wreq::Error) -> bool {
                match self {
                    $(Kind::$kind => error.$predicate()),*
                }
            }

            /// The exception class this kind selects, if any.
            fn exception(self, py: Python<'_>) -> Option<Bound<'_, PyType>> {
                match self {
                    $(Kind::$kind => define_kinds!(@exception py $($exception)?)),*
                }
            }
        }
    };
    (@exception $py:ident $exception:ident) => {
        Some($py.get_type::<exceptions::$exception>())
    };
    (@exception $py:ident) => {
        None
    };
}

// Causes found in the source chain come before error kinds, so a body read that timed
// out raises `TimeoutError`, not `BodyError`.
define_kinds! {
    Timeout("timeout", is_timeout) => TimeoutError,
    ProxyConnect("proxy_connect", is_proxy_connect) => ProxyConnectionError,
    ConnectionReset("connection_reset", is_connection_reset) => ConnectionResetError,
    Connect("connect", is_connect) => ConnectionError,
    Dns("dns", is_dns),
    Tls("tls", is_tls) => TlsError,
    Body("body", is_body) => BodyError,
    Decode("decode", is_decode) => DecodingError,
    Redirect("redirect", is_redirect) => RedirectError,
    Status("status", is_status) => StatusError,
    Upgrade("upgrade", is_upgrade) => UpgradeError,
    WebSocket("websocket", is_websocket) => WebSocketError,
    Builder("builder", is_builder) => BuilderError,
    Request("request", is_request) => RequestError,
}

const _: () = assert!(Kind::ALL.len() <= u16::BITS as usize);

impl Kind {
    const fn bit(self) -> u16 {
        1 << self as u16
    }
}

// ===== impl Kinds =====

/// A set of [`Kind`]s.
#[derive(Clone, Copy)]
struct Kinds(u16);

impl Kinds {
    /// Every kind wreq reports for `error`.
    fn of(error: &wreq::Error) -> Self {
        Kind::ALL
            .into_iter()
            .filter(|kind| kind.matches(error))
            .collect()
    }

    const fn is_empty(self) -> bool {
        self.0 == 0
    }

    /// The kinds in this set, in class selection order.
    fn iter(self) -> impl Iterator<Item = Kind> {
        Kind::ALL
            .into_iter()
            .filter(move |kind| self.0 & kind.bit() != 0)
    }
}

impl From<Kind> for Kinds {
    fn from(kind: Kind) -> Self {
        Kinds(kind.bit())
    }
}

impl<const N: usize> From<[Kind; N]> for Kinds {
    fn from(kinds: [Kind; N]) -> Self {
        kinds.into_iter().collect()
    }
}

impl FromIterator<Kind> for Kinds {
    fn from_iter<I: IntoIterator<Item = Kind>>(kinds: I) -> Self {
        Kinds(kinds.into_iter().fold(0, |bits, kind| bits | kind.bit()))
    }
}

// ===== impl Details =====

/// What a wreq exception carries. PyO3 builds it into the exception instance only when
/// raising it, which keeps these fields out of the exception's constructor.
struct Details {
    kinds: Kinds,
    message: String,
    url: Option<String>,
    status: Option<StatusCode>,
    cause: Option<PyErr>,
}

impl Details {
    /// Details for a failure detected here rather than by wreq.
    fn new(kinds: impl Into<Kinds>, message: String) -> Self {
        Details {
            kinds: kinds.into(),
            message,
            url: None,
            status: None,
            cause: None,
        }
    }

    /// Details for a library error. The URL may hold credentials, so it stays out of
    /// the message.
    fn library(error: wreq::Error) -> Self {
        let kinds = Kinds::of(&error);
        let url = error.uri().map(ToString::to_string);
        let status = error.status().map(StatusCode);
        let (message, cause) = describe(&error.without_uri());
        Details {
            kinds,
            message,
            url,
            status,
            cause,
        }
    }

    /// Create the exception of the first kind that selects a class.
    fn instantiate(self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        let class = self
            .kinds
            .iter()
            .find_map(|kind| kind.exception(py))
            .unwrap_or_else(|| py.get_type::<exceptions::Error>());
        let error = class.call1((self.message,))?;
        // The class supplies the defaults for anything left unset.
        if !self.kinds.is_empty() {
            let names = PyFrozenSet::new(py, self.kinds.iter().map(Kind::name))?;
            error.setattr(intern!(py, "_predicates"), names)?;
        }
        if let Some(url) = self.url {
            error.setattr(intern!(py, "url"), url)?;
        }
        if let Some(status) = self.status {
            error.setattr(intern!(py, "status"), status)?;
        }
        if let Some(cause) = self.cause {
            error.setattr(intern!(py, "__cause__"), cause.into_value(py))?;
        }
        Ok(error.unbind())
    }
}

impl PyErrArguments for Details {
    // PyO3 raises a returned exception instance as it is. Should building it fail, the
    // base `Error` is raised with that failure as its message.
    fn arguments(self, py: Python<'_>) -> Py<PyAny> {
        self.instantiate(py)
            .unwrap_or_else(|err| err.into_value(py).into_any())
    }
}

impl From<Details> for PyErr {
    fn from(details: Details) -> Self {
        PyErr::new::<exceptions::Error, _>(details)
    }
}

/// Format `error` with its source chain and take the first Python exception in the
/// chain as its cause. Both need the interpreter, so neither happens without it.
fn describe(error: &wreq::Error) -> (String, Option<PyErr>) {
    Python::try_attach(|py| {
        let cause = iter::successors(error.source(), |&err| err.source())
            .find_map(|err| err.downcast_ref::<PyErr>())
            .map(|err| err.clone_ref(py));
        (SourceChain(error).to_string(), cause)
    })
    .unwrap_or_else(|| ("The Python interpreter is not available".to_owned(), None))
}

// ===== impl SourceChain =====

/// Displays an error followed by every cause in its source chain. Many errors repeat
/// their cause in their own message, so a cause the previous message already ends with
/// is skipped, and one that starts with it adds only the rest.
struct SourceChain<'a>(&'a wreq::Error);

impl fmt::Display for SourceChain<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // wreq's own message already ends with its direct source.
        fmt::Display::fmt(self.0, f)?;
        let mut previous = self.0.source().map(ToString::to_string).unwrap_or_default();
        let mut source = self.0.source().and_then(StdError::source);
        while let Some(err) = source {
            let message = err.to_string();
            if !previous.ends_with(&message) {
                match message
                    .strip_prefix(&previous)
                    .filter(|_| !previous.is_empty())
                {
                    Some(rest) => f.write_str(rest)?,
                    None => write!(f, ": {message}")?,
                }
            }
            previous = message;
            source = err.source();
        }
        Ok(())
    }
}

// ===== impl Error =====

/// Unified error enum
#[derive(Debug)]
pub enum Error {
    Memory,
    StopIteration,
    StopAsyncIteration,
    WebSocketDisconnected,
    WebSocketTimeout(Elapsed),
    InvalidHeaderName(header::InvalidHeaderName),
    InvalidHeaderValue(header::InvalidHeaderValue),
    Dns(NetError),
    IO(io::Error),
    Json(serde_json::Error),
    Form(serde_urlencoded::ser::Error),
    Library(wreq::Error),
}

impl From<Error> for PyErr {
    fn from(err: Error) -> Self {
        match err {
            Error::Memory => PyRuntimeError::new_err(RACE_CONDITION_ERROR_MSG),
            Error::StopIteration => PyStopIteration::new_err("The iterator is exhausted"),
            Error::StopAsyncIteration => {
                PyStopAsyncIteration::new_err("The async iterator is exhausted")
            }
            Error::WebSocketDisconnected => Details::new(
                Kind::WebSocket,
                "The WebSocket has been disconnected".to_owned(),
            )
            .into(),
            Error::WebSocketTimeout(err) => Details::new(
                [Kind::Timeout, Kind::WebSocket],
                format!("WebSocket receive timed out: {err}"),
            )
            .into(),
            Error::InvalidHeaderName(err) => Details::new(Kind::Builder, err.to_string()).into(),
            Error::InvalidHeaderValue(err) => Details::new(Kind::Builder, err.to_string()).into(),
            Error::Dns(err) => {
                Details::new(Kind::Builder, format!("DNS resolver error: {err}")).into()
            }
            // PyO3 raises the matching `OSError` subclass, such as `FileNotFoundError`.
            Error::IO(err) => err.into(),
            Error::Json(err) => Details::new(Kind::Builder, format!("JSON error: {err}")).into(),
            Error::Form(err) => Details::new(Kind::Builder, format!("Form error: {err}")).into(),
            Error::Library(err) => Details::library(err).into(),
        }
    }
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

impl From<io::Error> for Error {
    fn from(err: io::Error) -> Self {
        Error::IO(err)
    }
}

impl From<wreq::Error> for Error {
    fn from(err: wreq::Error) -> Self {
        Error::Library(err)
    }
}

#[cfg(test)]
mod tests {
    use std::{fmt, process::Command};

    use futures_util::FutureExt;
    use http_body_util::BodyExt;

    use super::*;

    #[test]
    fn unavailable_interpreter_skips_error_sources() {
        const CHILD: &str = "WREQ_TEST_UNAVAILABLE_INTERPRETER";
        if std::env::var_os(CHILD).is_none() {
            // Other tests may initialize Python; isolate this unavailable-state check.
            let result = Command::new(std::env::current_exe().unwrap())
                .args([
                    "--exact",
                    "error::tests::unavailable_interpreter_skips_error_sources",
                    "--test-threads=1",
                ])
                .env(CHILD, "1")
                .output()
                .unwrap();
            assert!(
                result.status.success(),
                "stdout:\n{}\nstderr:\n{}",
                String::from_utf8_lossy(&result.stdout),
                String::from_utf8_lossy(&result.stderr)
            );
            return;
        }

        struct DebugBomb;

        impl fmt::Debug for DebugBomb {
            fn fmt(&self, _: &mut fmt::Formatter<'_>) -> fmt::Result {
                panic!("error source must not be formatted without Python");
            }
        }

        impl fmt::Display for DebugBomb {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                fmt::Debug::fmt(self, f)
            }
        }

        impl std::error::Error for DebugBomb {}

        assert!(Python::try_attach(|_| ()).is_none());
        let mut body =
            wreq::Body::wrap_stream(futures_util::stream::iter([Err::<bytes::Bytes, _>(
                DebugBomb,
            )]));
        let error = body.frame().now_or_never().unwrap().unwrap().unwrap_err();
        assert!(error.is_request());
        let (message, cause) = describe(&error);
        assert_eq!(message, "The Python interpreter is not available");
        assert!(cause.is_none());
        // Constructing and dropping the public exception must also avoid its source.
        drop(PyErr::from(Error::Library(error)));
    }
}
