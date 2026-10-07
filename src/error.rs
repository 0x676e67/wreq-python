use std::io;

use hickory_resolver::net::NetError;
use pyo3::{
    PyErr, Python,
    exceptions::{PyRuntimeError, PyStopAsyncIteration, PyStopIteration},
};
use tokio::time::error::Elapsed;
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

/// Map a library error to its exception. Causes found in the source chain come before
/// error kinds, so a body read that timed out raises `TimeoutError`, not `BodyError`.
fn library_error(error: wreq::Error) -> PyErr {
    macro_rules! classify {
        ($($variant:ident => $exception:ident),*) => {
            $(
                if error.$variant() {
                    return exceptions::$exception::new_err(format_library_error(
                        &error,
                        concat!(stringify!($variant), " error"),
                    ));
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
    exceptions::Error::new_err(format_library_error(&error, "error"))
}

/// Error sources can include PyErr, whose formatting attaches to Python.
fn format_library_error(error: &wreq::Error, label: &str) -> String {
    Python::try_attach(|_| format!("{label}: {error:?}"))
        .unwrap_or_else(|| format!("{label}: The Python interpreter is not available"))
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
            Error::WebSocketDisconnected => {
                exceptions::WebSocketError::new_err("The WebSocket has been disconnected")
            }
            Error::InvalidHeaderName(err) => {
                exceptions::BuilderError::new_err(format!("Invalid header name: {err:?}"))
            }
            Error::InvalidHeaderValue(err) => {
                exceptions::BuilderError::new_err(format!("Invalid header value: {err:?}"))
            }
            Error::Timeout(err) => {
                exceptions::TimeoutError::new_err(format!("Timeout error: {err:?}"))
            }
            // PyO3 raises the matching `OSError` subclass, such as `FileNotFoundError`.
            Error::IO(err) => err.into(),
            Error::Decode(err) => {
                exceptions::DecodingError::new_err(format!("Decode error: {err:?}"))
            }
            Error::Builder(err) => {
                exceptions::BuilderError::new_err(format!("Builder error: {err:?}"))
            }
            Error::Dns(err) => {
                exceptions::BuilderError::new_err(format!("DNS resolver error: {err:?}"))
            }
            Error::Json(err) => exceptions::BuilderError::new_err(format!("JSON error: {err:?}")),
            Error::Form(err) => exceptions::BuilderError::new_err(format!("Form error: {err:?}")),
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
            format_library_error(&error, "is_request error"),
            "is_request error: The Python interpreter is not available"
        );
        // Constructing and dropping the public exception must also avoid its source.
        drop(PyErr::from(Error::Library(error)));
    }
}
