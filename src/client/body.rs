//! Types and utilities for representing HTTP request bodies.

mod form;
mod json;
pub mod multipart;
mod stream;

use pyo3::{FromPyObject, PyResult, prelude::*};

pub use self::{
    form::Form,
    json::Json,
    stream::{PyStream, Streamer},
};
use crate::extractor::{BytesInput, StrInput};

/// Represents the body of an HTTP request.
#[derive(FromPyObject)]
pub enum Body {
    Text(StrInput),
    Bytes(BytesInput),
    Form(Form),
    Json(Json),
    Stream(PyStream),
}

impl TryFrom<Body> for wreq::Body {
    type Error = PyErr;

    fn try_from(value: Body) -> PyResult<wreq::Body> {
        match value {
            Body::Form(form) => serde_urlencoded::to_string(form)
                .map(wreq::Body::from)
                .map_err(crate::Error::Form)
                .map_err(Into::into),
            Body::Json(json) => serde_json::to_vec(&json)
                .map_err(crate::Error::Json)
                .map(wreq::Body::from)
                .map_err(Into::into),
            Body::Text(s) => Ok(wreq::Body::from(s.0)),
            Body::Bytes(bytes) => Ok(wreq::Body::from(bytes.0)),
            Body::Stream(stream) => Ok(wreq::Body::wrap_stream(stream)),
        }
    }
}
