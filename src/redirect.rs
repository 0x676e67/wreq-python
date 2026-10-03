use std::{fmt::Display, sync::Arc};

use pyo3::prelude::*;

use crate::{header::HeaderMap, http::StatusCode};

/// Represents the redirect policy for HTTP requests.
#[derive(Clone)]
#[pyclass(frozen, str, from_py_object)]
pub struct Policy(pub wreq::redirect::Policy);

/// A type that holds information on the next request and previous requests
/// in redirect chain.
#[pyclass]
pub struct Attempt {
    #[pyo3(get)]
    status: StatusCode,

    #[pyo3(get)]
    headers: HeaderMap,

    #[pyo3(get)]
    next: String,

    #[pyo3(get)]
    previous: Vec<String>,
}

/// An action to perform when a redirect status code is found.
#[derive(Clone)]
#[pyclass(frozen, str, from_py_object)]
pub struct Action {
    kind: ActionKind,
}

#[derive(Clone)]
enum ActionKind {
    Follow,
    Stop,
    Error(String),
}

/// An entry in the redirect history.
#[pyclass(subclass, str, frozen)]
pub struct History(pub wreq::redirect::HistoryEntry);

#[pymethods]
impl History {
    /// Get the status code of the redirect response.
    #[getter]
    fn status(&self) -> u16 {
        self.0.status.as_u16()
    }

    /// Get the URL of the redirect response.
    #[getter]
    fn url(&self) -> String {
        self.0.uri.to_string()
    }

    /// Get the previous URL before the redirect response.
    #[getter]
    fn previous(&self) -> String {
        self.0.previous.to_string()
    }

    /// Get the headers of the redirect response.
    #[getter]
    fn headers(&self) -> HeaderMap {
        HeaderMap(self.0.headers.clone())
    }
}

// ===== impl Policy =====

#[pymethods]
impl Policy {
    /// Create a [`Policy`] with a maximum number of redirects.
    ///
    /// An `Error` will be returned if the max is reached.
    #[staticmethod]
    #[pyo3(signature = (max=None))]
    pub fn limited(max: Option<usize>) -> Self {
        Self(max.map_or_else(
            wreq::redirect::Policy::default,
            wreq::redirect::Policy::limited,
        ))
    }

    /// Create a [`Policy`] that does not follow any redirect.
    #[staticmethod]
    pub fn none() -> Self {
        Self(wreq::redirect::Policy::none())
    }

    /// Create a custom `Policy` using the passed function.
    #[staticmethod]
    #[pyo3(signature = (callback))]
    pub fn custom(callback: Py<PyAny>) -> Self {
        let callback = Arc::new(callback);
        let polciy = wreq::redirect::Policy::custom(move |attempt| {
            let callback = callback.clone();
            attempt.pending(|attempt| async move {
                let args = Attempt::from(&attempt);
                let kind = tokio::task::spawn_blocking(move || {
                    Python::try_attach(|py| {
                        callback
                            .call1(py, (args,))
                            .and_then(|result| result.extract::<Action>(py).map_err(PyErr::from))
                            .map(|action| action.kind)
                            .unwrap_or_else(|err| ActionKind::Error(err.to_string()))
                    })
                    .unwrap_or_else(|| {
                        ActionKind::Error("The Python interpreter is not available".into())
                    })
                })
                .await;

                match kind {
                    Ok(ActionKind::Follow) => attempt.follow(),
                    Ok(ActionKind::Stop) => attempt.stop(),
                    Ok(ActionKind::Error(msg)) => attempt.error(msg),
                    Err(err) => attempt.error(err.to_string()),
                }
            })
        });

        Self(polciy)
    }
}

impl_print_str!(Debug, Policy);

// ===== impl Attempt =====

#[pymethods]
impl Attempt {
    /// Returns an action meaning the client should follow the next URI.
    pub fn follow(&self) -> Action {
        Action {
            kind: ActionKind::Follow,
        }
    }

    /// Returns an action meaning the client should not follow the next URI.
    ///
    /// The 30x response will be returned as the result.
    pub fn stop(&self) -> Action {
        Action {
            kind: ActionKind::Stop,
        }
    }

    /// Returns an action failing the redirect with an error.
    ///
    /// The error will be returned for the result of the sent request.
    pub fn error(&self, message: String) -> Action {
        Action {
            kind: ActionKind::Error(message),
        }
    }
}

impl From<&wreq::redirect::Attempt<'static, false>> for Attempt {
    fn from(attempt: &wreq::redirect::Attempt<'static, false>) -> Self {
        Attempt {
            status: StatusCode(attempt.status),
            headers: HeaderMap(attempt.headers.clone().into_owned()),
            next: attempt.uri.to_string(),
            previous: attempt.previous.iter().map(ToString::to_string).collect(),
        }
    }
}

impl Display for Attempt {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Attempt {{ status: {}, next: {}, previous: {:?} }}",
            self.status, self.next, self.previous
        )
    }
}

// ===== impl Action =====

impl Display for Action {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self.kind {
            ActionKind::Follow => write!(f, "Action::Follow"),
            ActionKind::Stop => write!(f, "Action::Stop"),
            ActionKind::Error(msg) => write!(f, "Action::Error({})", msg),
        }
    }
}

// ===== impl History =====

impl_print_str!(Debug, History);
