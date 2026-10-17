use std::{error::Error as StdError, fmt, io};

use hickory_resolver::net::NetError;
use pyo3::{
    PyErr, PyTypeInfo, Python,
    exceptions::{PyRuntimeError, PyStopAsyncIteration, PyStopIteration},
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

/// Arguments of `wreq.exceptions.Error`: the message, the names of the matching
/// predicates, the request URL and the response status.
type ErrorArgs = (
    String,
    Vec<&'static str>,
    Option<String>,
    Option<StatusCode>,
);

/// A [`wreq::Error`] predicate and the name its `is_*` method in
/// `wreq.exceptions.Error` looks up.
type Predicate = (&'static str, fn(&wreq::Error) -> bool);

const PREDICATES: [Predicate; 14] = [
    ("builder", wreq::Error::is_builder),
    ("request", wreq::Error::is_request),
    ("connect", wreq::Error::is_connect),
    ("proxy_connect", wreq::Error::is_proxy_connect),
    ("connection_reset", wreq::Error::is_connection_reset),
    ("dns", wreq::Error::is_dns),
    ("timeout", wreq::Error::is_timeout),
    ("body", wreq::Error::is_body),
    ("tls", wreq::Error::is_tls),
    ("decode", wreq::Error::is_decode),
    ("redirect", wreq::Error::is_redirect),
    ("status", wreq::Error::is_status),
    ("upgrade", wreq::Error::is_upgrade),
    ("websocket", wreq::Error::is_websocket),
];

/// Map a library error to its exception, keeping every predicate it matches. Causes
/// found in the source chain come before error kinds, so a body read that timed out
/// raises `TimeoutError`, not `BodyError`.
fn library_error(error: wreq::Error) -> PyErr {
    let predicates = PREDICATES
        .iter()
        .filter(|(_, matches)| matches(&error))
        .map(|&(name, _)| name)
        .collect();
    // The URL may hold credentials, so it stays out of the message.
    let url = error.uri().map(ToString::to_string);
    let status = error.status().map(StatusCode);
    let error = error.without_uri();
    let args: ErrorArgs = (format_library_error(&error), predicates, url, status);

    macro_rules! classify {
        ($($variant:ident => $exception:ident),*) => {
            $(
                if error.$variant() {
                    return exceptions::$exception::new_err(args);
                }
            )*
        };
    }

    classify!(
        is_timeout => TimeoutError,
        is_proxy_connect => ProxyConnectionError,
        is_connection_reset => ConnectionResetError,
        is_connect => ConnectionError,
        is_tls => TlsError,
        is_body => BodyError,
        is_decode => DecodingError,
        is_redirect => RedirectError,
        is_status => StatusError,
        is_upgrade => UpgradeError,
        is_websocket => WebSocketError,
        is_builder => BuilderError,
        is_request => RequestError
    );
    exceptions::Error::new_err(args)
}

/// Raise `T` for a failure detected here rather than by wreq, matching the predicate
/// its class implies.
fn binding_error<T: PyTypeInfo>(predicate: &'static str, message: String) -> PyErr {
    PyErr::new::<T, ErrorArgs>((message, vec![predicate], None, None))
}

/// Error sources can include PyErr, whose formatting attaches to Python.
fn format_library_error(error: &wreq::Error) -> String {
    Python::try_attach(|_| SourceChain(error).to_string())
        .unwrap_or_else(|| "The Python interpreter is not available".to_owned())
}

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

/// Unified error enum
#[derive(Debug)]
pub enum Error {
    Memory,
    StopIteration,
    StopAsyncIteration,
    WebSocketDisconnected,
    InvalidHeaderName(header::InvalidHeaderName),
    InvalidHeaderValue(header::InvalidHeaderValue),
    Timeout(Elapsed),
    Builder(http::Error),
    Dns(NetError),
    IO(io::Error),
    Decode(cookie::ParseError),
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
            Error::WebSocketDisconnected => binding_error::<exceptions::WebSocketError>(
                "websocket",
                "The WebSocket has been disconnected".to_owned(),
            ),
            Error::InvalidHeaderName(err) => binding_error::<exceptions::BuilderError>(
                "builder",
                format!("Invalid header name: {err:?}"),
            ),
            Error::InvalidHeaderValue(err) => binding_error::<exceptions::BuilderError>(
                "builder",
                format!("Invalid header value: {err:?}"),
            ),
            Error::Timeout(err) => binding_error::<exceptions::TimeoutError>(
                "timeout",
                format!("Timeout error: {err:?}"),
            ),
            // PyO3 raises the matching `OSError` subclass, such as `FileNotFoundError`.
            Error::IO(err) => err.into(),
            Error::Decode(err) => binding_error::<exceptions::DecodingError>(
                "decode",
                format!("Decode error: {err:?}"),
            ),
            Error::Builder(err) => binding_error::<exceptions::BuilderError>(
                "builder",
                format!("Builder error: {err:?}"),
            ),
            Error::Dns(err) => binding_error::<exceptions::BuilderError>(
                "builder",
                format!("DNS resolver error: {err:?}"),
            ),
            Error::Json(err) => {
                binding_error::<exceptions::BuilderError>("builder", format!("JSON error: {err:?}"))
            }
            Error::Form(err) => {
                binding_error::<exceptions::BuilderError>("builder", format!("Form error: {err:?}"))
            }
            Error::Library(err) => library_error(err),
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

impl From<Elapsed> for Error {
    fn from(err: Elapsed) -> Self {
        Error::Timeout(err)
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
        assert_eq!(
            format_library_error(&error),
            "The Python interpreter is not available"
        );
        // Constructing and dropping the public exception must also avoid its source.
        drop(PyErr::from(Error::Library(error)));
    }
}
