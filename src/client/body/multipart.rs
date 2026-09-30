use std::path::PathBuf;

use pyo3::{prelude::*, types::PyTuple};
use wreq::{Body, multipart};

use crate::{
    client::body::PyStream,
    error::Error,
    extractor::{BytesInput, StrInput},
    header::HeaderMap,
};

/// A multipart form for a request.
#[pyclass(subclass)]
pub struct Multipart {
    pub form: Option<multipart::Form>,
    pub parts: Vec<Part>,
}

/// The data for a part value of a multipart form.
#[derive(FromPyObject)]
pub enum Value {
    Text(StrInput),
    Bytes(BytesInput),
    File(PathBuf),
    Stream(PyStream),
}

/// A part of a multipart form.
#[pyclass(subclass)]
pub struct Part {
    pub name: String,
    pub value: Option<Value>,
    pub filename: Option<String>,
    pub mime: Option<String>,
    pub length: Option<u64>,
    pub headers: Option<HeaderMap>,
}

// ===== impl Multipart =====

#[pymethods]
impl Multipart {
    /// Creates a new multipart.
    #[new]
    #[pyo3(signature = (*parts))]
    pub fn new(parts: &Bound<PyTuple>) -> PyResult<Multipart> {
        let mut new_parts = Vec::with_capacity(parts.len());
        for part in parts {
            let part = part.cast::<Part>()?;
            let mut part = part.borrow_mut();
            new_parts.push(part.try_clone()?);
        }

        Ok(Self {
            form: None,
            parts: new_parts,
        })
    }
}

impl Multipart {
    fn build_form(&mut self, py: Python) -> PyResult<multipart::Form> {
        let mut form = multipart::Form::new();
        for part in &mut self.parts {
            let (name, inner) = part.build_part(py)?;
            form = form.part(name, inner);
        }
        Ok(form)
    }
}

impl FromPyObject<'_, '_> for Multipart {
    type Error = PyErr;

    fn extract(ob: Borrowed<PyAny>) -> PyResult<Self> {
        let multipart = ob.cast::<Multipart>()?;
        let mut multipart = multipart.borrow_mut();
        let form = multipart.build_form(ob.py())?;

        Ok(Multipart {
            form: Some(form),
            parts: Vec::new(),
        })
    }
}

// ===== impl Value =====

impl Value {
    fn try_clone(&self) -> Option<Self> {
        match self {
            Value::Text(text) => {
                let text = text.clone();
                Some(Value::Text(text))
            }
            Value::Bytes(bytes) => {
                let bytes = bytes.clone();
                Some(Value::Bytes(bytes))
            }
            Value::File(path) => {
                let path = path.clone();
                Some(Value::File(path))
            }
            Value::Stream(_) => None,
        }
    }
}

// ===== impl Part =====

impl Part {
    fn with_value(&self, value: Value) -> Part {
        Part {
            name: self.name.clone(),
            value: Some(value),
            filename: self.filename.clone(),
            mime: self.mime.clone(),
            length: self.length,
            headers: self.headers.clone(),
        }
    }

    fn build_part(&mut self, py: Python) -> PyResult<(String, multipart::Part)> {
        let value = self
            .value
            .as_ref()
            .and_then(Value::try_clone)
            .or_else(|| self.value.take())
            .ok_or_else(|| Error::Memory)?;

        py.detach(move || {
            let mut inner = match value {
                Value::Text(text) => multipart::Part::stream(text.0),
                Value::Bytes(bytes) => multipart::Part::stream(bytes.0),
                Value::File(path) => crate::runtime::get().handle().block_on(async move {
                    multipart::Part::file(path).await.map_err(Error::from)
                })?,
                Value::Stream(stream) => {
                    let stream = Body::wrap_stream(stream);
                    match self.length {
                        Some(length) => multipart::Part::stream_with_length(stream, length),
                        None => multipart::Part::stream(stream),
                    }
                }
            };

            if let Some(filename) = self.filename.clone() {
                inner = inner.file_name(filename);
            }

            if let Some(ref mime) = self.mime {
                inner = inner.mime_str(mime).map_err(Error::Library)?;
            }

            if let Some(headers) = self.headers.clone() {
                inner = inner.headers(headers.0);
            }

            Ok((self.name.clone(), inner))
        })
    }

    fn try_clone(&mut self) -> PyResult<Part> {
        if let Some(part) = self
            .value
            .as_ref()
            .and_then(Value::try_clone)
            .map(|value| self.with_value(value))
        {
            return Ok(part);
        }

        self.value
            .take()
            .map(|value| self.with_value(value))
            .ok_or_else(|| Error::Memory)
            .map_err(Into::into)
    }
}

#[pymethods]
impl Part {
    /// Creates a new part.
    #[new]
    #[pyo3(signature = (
        name,
        value,
        filename = None,
        mime = None,
        length = None,
        headers = None
    ))]
    pub fn new(
        name: String,
        value: Value,
        filename: Option<String>,
        mime: Option<&str>,
        length: Option<u64>,
        headers: Option<HeaderMap>,
    ) -> Part {
        Part {
            name,
            value: Some(value),
            filename,
            mime: mime.map(ToOwned::to_owned),
            length,
            headers,
        }
    }
}
