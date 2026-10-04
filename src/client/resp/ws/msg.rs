//! WebSocket Message Utilities
//!
//! This module provides the `Message` type for representing WebSocket messages,
//! including text, binary, ping, pong, and close frames. It offers constructors
//! for creating messages of various types, as well as methods and getters for
//! extracting message content (such as text, binary data, ping/pong payloads, and close reason).
//!
//! The `Message` type is used for sending and receiving WebSocket messages in a unified way.

use std::fmt::Debug;

use pyo3::{exceptions::PyValueError, prelude::*};
use wreq::ws::message::{self, CloseCode, CloseFrame, Utf8Bytes};

use crate::{
    buffer::PyBuffer,
    client::body::Json,
    error::Error,
    extractor::{BytesInput, StrInput},
};

/// An enum representing either a bytes message or a JSON message.
#[derive(FromPyObject)]
pub enum BytesLike {
    Bytes(BytesInput),
    Json(Json),
}

/// An enum representing either a text message or a JSON message.
#[derive(FromPyObject)]
pub enum TextLike {
    Text(StrInput),
    Json(Json),
}

/// A WebSocket message.
#[derive(Debug, Clone)]
#[pyclass(subclass, str, frozen, from_py_object)]
pub struct Message(pub message::Message);

#[pymethods]
impl Message {
    /// Returns the message data as a read-only memoryview.
    #[getter]
    pub fn data(&self) -> Option<PyBuffer> {
        let bytes = match &self.0 {
            message::Message::Text(text) => text.clone().into(),
            message::Message::Binary(bytes)
            | message::Message::Ping(bytes)
            | message::Message::Pong(bytes) => bytes.clone(),
            _ => return None,
        };
        Some(PyBuffer::from(bytes))
    }

    /// Returns the text content of the message if it is a text message.
    #[getter]
    pub fn text(&self) -> Option<&str> {
        if let message::Message::Text(text) = &self.0 {
            Some(text)
        } else {
            None
        }
    }

    /// Returns a read-only memoryview if this is a binary message.
    #[getter]
    pub fn binary(&self) -> Option<PyBuffer> {
        if let message::Message::Binary(data) = &self.0 {
            Some(PyBuffer::from(data))
        } else {
            None
        }
    }

    /// Returns a read-only memoryview if this is a ping message.
    #[getter]
    pub fn ping(&self) -> Option<PyBuffer> {
        if let message::Message::Ping(data) = &self.0 {
            Some(PyBuffer::from(data))
        } else {
            None
        }
    }

    /// Returns a read-only memoryview if this is a pong message.
    #[getter]
    pub fn pong(&self) -> Option<PyBuffer> {
        if let message::Message::Pong(data) = &self.0 {
            Some(PyBuffer::from(data))
        } else {
            None
        }
    }

    /// Returns the JSON representation of the message.
    #[getter]
    pub fn json(&self, py: Python) -> Option<Json> {
        py.detach(|| self.0.json::<Json>().ok())
    }

    /// Returns the close code and reason of the message if it is a close message.
    #[getter]
    pub fn close(&self) -> Option<(u16, Option<&str>)> {
        if let message::Message::Close(Some(s)) = &self.0 {
            Some((u16::from(s.code.clone()), Some(s.reason.as_str())))
        } else {
            None
        }
    }
}

#[pymethods]
impl Message {
    /// Creates a text message from a string, or from a JSON value serialized as text.
    #[staticmethod]
    #[pyo3(signature = (like))]
    pub fn from_text(like: TextLike) -> PyResult<Self> {
        match like {
            // A Python str is always valid UTF-8.
            TextLike::Text(text) => Utf8Bytes::try_from(text.0)
                .map(message::Message::text)
                .map(Self)
                .map_err(|err| PyValueError::new_err(err.to_string())),
            TextLike::Json(json) => message::Message::text_from_json(&json)
                .map(Self)
                .map_err(Error::Library)
                .map_err(Into::into),
        }
    }

    /// Creates a binary message from bytes, or from a JSON value serialized as binary.
    #[staticmethod]
    #[pyo3(signature = (like))]
    pub fn from_binary(like: BytesLike) -> PyResult<Self> {
        match like {
            BytesLike::Bytes(bytes) => Ok(Self(message::Message::binary(bytes.0))),
            BytesLike::Json(json) => message::Message::binary_from_json(&json)
                .map(Message)
                .map_err(Error::Library)
                .map_err(Into::into),
        }
    }

    /// Creates a new ping message.
    #[staticmethod]
    #[pyo3(signature = (data))]
    pub fn from_ping(data: BytesInput) -> Self {
        Self(message::Message::ping(data.0))
    }

    /// Creates a new pong message.
    #[staticmethod]
    #[pyo3(signature = (data))]
    pub fn from_pong(data: BytesInput) -> Self {
        Self(message::Message::pong(data.0))
    }

    /// Creates a new close message.
    #[staticmethod]
    #[pyo3(signature = (code, reason=None))]
    pub fn from_close(code: u16, reason: Option<StrInput>) -> Self {
        let reason = reason
            .map(|reason| reason.0)
            .and_then(|b| Utf8Bytes::try_from(b).ok())
            .unwrap_or_else(|| Utf8Bytes::from_static("Goodbye"));
        let msg = message::Message::close(CloseFrame {
            code: CloseCode::from(code),
            reason,
        });
        Self(msg)
    }
}

impl_print_str!(Display, Message);
