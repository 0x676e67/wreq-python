//! Request bodies extracted from Python.

mod form;
mod json;
pub mod multipart;
mod stream;

use pyo3::{
    FromPyObject, PyResult, intern,
    prelude::*,
    types::{PyByteArray, PyBytes, PyDict, PyIterator, PyList, PyString, PyTuple},
};

pub use self::{
    form::Form,
    json::Json,
    stream::{Pulls, PyStream},
};
use crate::{
    error::Error,
    extractor::{Binary, Text},
};

/// A `body=` argument, matched by type before trying form, JSON and then a stream.
/// Mappings and pair sequences with scalar values are form-encoded and other containers
/// JSON-encoded, without the `Content-Type` that `form=` and `json=` set.
pub enum Body {
    Text(Text),
    Bytes(Binary),
    Form(Form),
    Json(Json),
    Stream(PyStream),
}

impl FromPyObject<'_, '_> for Body {
    type Error = PyErr;

    fn extract(ob: Borrowed<PyAny>) -> PyResult<Self> {
        // Match common bodies by type: each failed attempt builds an exception, and
        // normalizing one releases and reacquires the GIL.
        if ob.is_instance_of::<PyString>() {
            return ob.extract().map(Body::Text);
        }
        if ob.is_instance_of::<PyBytes>() || ob.is_instance_of::<PyByteArray>() {
            return ob.extract().map(Body::Bytes);
        }
        if ob.cast::<PyIterator>().is_ok() {
            return ob.extract().map(Body::Stream);
        }
        let container = ob.is_instance_of::<PyDict>()
            || ob.is_instance_of::<PyList>()
            || ob.is_instance_of::<PyTuple>();
        if !container && ob.hasattr(intern!(ob.py(), "asend"))? {
            return ob.extract().map(Body::Stream);
        }
        ob.extract()
            .map(Body::Form)
            .or_else(|_| ob.extract().map(Body::Json))
            .or_else(|_| ob.extract().map(Body::Stream))
    }
}

impl TryFrom<Body> for wreq::Body {
    type Error = PyErr;

    fn try_from(value: Body) -> PyResult<wreq::Body> {
        match value {
            Body::Form(form) => serde_urlencoded::to_string(form)
                .map(wreq::Body::from)
                .map_err(Error::Form)
                .map_err(Into::into),
            Body::Json(json) => serde_json::to_vec(&json)
                .map_err(Error::Json)
                .map(wreq::Body::from)
                .map_err(Into::into),
            Body::Text(s) => Ok(wreq::Body::from(s.0)),
            Body::Bytes(bytes) => Ok(wreq::Body::from(bytes.0)),
            Body::Stream(stream) => Ok(wreq::Body::wrap_stream(stream)),
        }
    }
}
