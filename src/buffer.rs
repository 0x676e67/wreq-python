// Licensed to the Apache Software Foundation (ASF) under one
// or more contributor license agreements.  See the NOTICE file
// distributed with this work for additional information
// regarding copyright ownership.  The ASF licenses this file
// to you under the Apache License, Version 2.0 (the
// "License"); you may not use this file except in compliance
// with the License.  You may obtain a copy of the License at
//
//   http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing,
// software distributed under the License is distributed on an
// "AS IS" BASIS, WITHOUT WARRANTIES OR CONDITIONS OF ANY
// KIND, either express or implied.  See the License for the
// specific language governing permissions and limitations
// under the License.

use std::{ops::Range, os::raw::c_int};

use bytes::Bytes;
use pyo3::{
    buffer::PyUntypedBuffer,
    exceptions::{PyOverflowError, PyTypeError},
    ffi, intern,
    prelude::*,
    pybacked::PyBackedBytes,
    types::{PyByteArray, PyBytes, PyMemoryView},
};
use wreq::header::{HeaderCaseName, HeaderName, HeaderValue};

/// Bytes shared with Python in both directions: extracted from `bytes`, `bytearray` or
/// `memoryview` input and returned as a read-only memoryview.
///
/// Input is shared only when an exact `bytes` or one of our own memoryviews owns it. Mutable
/// exporters and subclasses are copied, so a `Bytes` never aliases writable memory or hides a
/// reference cycle from the GC.
#[derive(Clone)]
pub struct PyBuffer(pub Bytes);

/// The exporter behind each memoryview; views keep it, and so its bytes, alive.
#[pyclass(frozen)]
struct BufferView(Bytes);

// ===== impl PyBuffer =====

impl PyBuffer {
    /// Whether `ob` is a `bytes`, `bytearray` or `memoryview`, so callers can dispatch by type
    /// instead of building an error for each failed attempt.
    #[inline]
    pub fn is_bytes_like(ob: &Bound<PyAny>) -> bool {
        ob.is_instance_of::<PyBytes>()
            || ob.is_instance_of::<PyByteArray>()
            || ob.is_instance_of::<PyMemoryView>()
    }
}

impl FromPyObject<'_, '_> for PyBuffer {
    type Error = PyErr;

    fn extract(ob: Borrowed<PyAny>) -> PyResult<Self> {
        let bytes = if ob.is_exact_instance_of::<PyBytes>() || ob.is_instance_of::<PyByteArray>() {
            // Shares an exact bytes and copies a bytearray.
            Bytes::from_owner(ob.extract::<PyBackedBytes>()?)
        } else if let Ok(bytes) = ob.cast::<PyBytes>() {
            // Subclasses can reference an exported view; Bytes owners are invisible to GC.
            Bytes::copy_from_slice(bytes.as_bytes())
        } else if let Ok(view) = ob.cast::<PyMemoryView>() {
            view_bytes(&view)?
        } else {
            // Other exporters, such as numpy numbers or `array('d')`, count as bytes only in a
            // memoryview, so `body=` and `from_binary` keep encoding them as JSON.
            return Err(PyTypeError::new_err(format!(
                "'{}' object is not an instance of 'bytes | bytearray | memoryview'",
                ob.get_type().qualname()?
            )));
        };
        Ok(Self(bytes))
    }
}

impl<'a> IntoPyObject<'a> for PyBuffer {
    type Target = PyMemoryView;
    type Output = Bound<'a, Self::Target>;
    type Error = PyErr;

    #[inline(always)]
    fn into_pyobject(self, py: Python<'a>) -> Result<Self::Output, Self::Error> {
        let buffer = BufferView(self.0).into_pyobject(py)?;
        PyMemoryView::from(buffer.as_any())
    }
}

impl From<Vec<u8>> for PyBuffer {
    #[inline]
    fn from(value: Vec<u8>) -> Self {
        Self::from(Bytes::from(value))
    }
}

impl From<&Bytes> for PyBuffer {
    #[inline]
    fn from(value: &Bytes) -> Self {
        Self::from(value.clone())
    }
}

impl From<Bytes> for PyBuffer {
    #[inline]
    fn from(value: Bytes) -> Self {
        PyBuffer(value)
    }
}

impl From<HeaderName> for PyBuffer {
    #[inline]
    fn from(value: HeaderName) -> Self {
        Self::from(Bytes::from_owner(value))
    }
}

impl From<HeaderCaseName> for PyBuffer {
    fn from(value: HeaderCaseName) -> Self {
        Self::from(Bytes::from_owner(value))
    }
}

impl From<HeaderValue> for PyBuffer {
    #[inline]
    fn from(value: HeaderValue) -> Self {
        Self::from(Bytes::from_owner(value))
    }
}

// ===== impl BufferView =====

#[pymethods]
impl BufferView {
    /// # Safety
    /// `view` must be a writable `Py_buffer` supplied by the Python buffer protocol.
    #[allow(unsafe_code)]
    unsafe fn __getbuffer__(
        slf: PyRef<Self>,
        view: *mut ffi::Py_buffer,
        flags: c_int,
    ) -> PyResult<()> {
        let bytes = &slf.0;
        let len = ffi::Py_ssize_t::try_from(bytes.len())
            .map_err(|_| PyOverflowError::new_err("buffer length exceeds Python's maximum size"))?;
        // SAFETY: PyO3 supplies a valid output buffer. FillInfo retains the exporter,
        // which owns immutable Bytes for the lifetime of this read-only buffer.
        let ret = unsafe {
            ffi::PyBuffer_FillInfo(
                view,
                slf.as_ptr() as *mut _,
                bytes.as_ptr() as *mut _,
                len,
                1,
                flags,
            )
        };
        if ret == -1 {
            return Err(PyErr::fetch(slf.py()));
        }
        Ok(())
    }
}

/// Shares a view of an exact `bytes` or of our own memoryview, and copies any other view.
fn view_bytes(view: &Bound<PyMemoryView>) -> PyResult<Bytes> {
    let obj = view.getattr(intern!(view.py(), "obj"))?;
    if let Ok(exporter) = obj.cast::<BufferView>() {
        let bytes = &exporter.get().0;
        if let Some(range) = view_range(view, bytes) {
            return Ok(bytes.slice(range));
        }
    } else if let Ok(bytes) = obj.cast_into_exact::<PyBytes>()
        && let Some(range) = view_range(view, bytes.as_bytes())
    {
        return Ok(Bytes::from_owner(PyBackedBytes::from(bytes)).slice(range));
    }
    // `bytes()` copies any layout or item format as raw bytes.
    let copy = view
        .py()
        .get_type::<PyBytes>()
        .call1((view,))?
        .cast_into::<PyBytes>()?;
    Ok(Bytes::from_owner(PyBackedBytes::from(copy)))
}

/// The range of `data` that a C-contiguous `view` covers, if the view lies within it.
fn view_range(view: &Bound<PyMemoryView>, data: &[u8]) -> Option<Range<usize>> {
    let buffer = PyUntypedBuffer::get(view.as_any()).ok()?;
    if !buffer.is_c_contiguous() {
        return None;
    }
    let start = buffer.buf_ptr().addr().checked_sub(data.as_ptr().addr())?;
    let end = start.checked_add(buffer.len_bytes())?;
    (end <= data.len()).then_some(start..end)
}

#[cfg(test)]
mod tests {
    use pyo3::{
        buffer::PyBuffer as PythonBuffer,
        types::{PySlice, PyString},
    };

    use super::*;
    use crate::extractor::Text;

    #[test]
    fn buffers_share_immutable_bytes() {
        Python::initialize();
        Python::attach(|py| {
            let bytes = Bytes::from(vec![0, 1, 255]);
            let ptr = bytes.as_ptr();
            let view = PyBuffer::from(bytes).into_pyobject(py).unwrap();
            let buffer = PythonBuffer::<u8>::get(view.as_any()).unwrap();

            assert_eq!(buffer.buf_ptr().cast_const().cast::<u8>(), ptr);
            assert!(buffer.readonly());
            assert_eq!(buffer.to_vec(py).unwrap(), [0, 1, 255]);

            // Our views come back shared, sliced or not; a strided one is copied.
            let input = view.extract::<PyBuffer>().unwrap();
            assert_eq!(input.0.as_ptr(), ptr);
            let tail = view.get_item(PySlice::new(py, 1, 3, 1)).unwrap();
            let input = tail.extract::<PyBuffer>().unwrap();
            assert_eq!(input.0.as_ptr(), ptr.wrapping_add(1));
            assert_eq!(input.0[..], [1, 255]);
            let strided = view.get_item(PySlice::new(py, 0, 3, 2)).unwrap();
            assert_eq!(strided.extract::<PyBuffer>().unwrap().0[..], [0, 255]);

            let binary = PyBytes::new(py, b"builtin bytes");
            let input = binary.extract::<PyBuffer>().unwrap();
            assert_eq!(input.0.as_ptr(), binary.as_bytes().as_ptr());
            let view = PyBuffer::from(input.0).into_pyobject(py).unwrap();
            let buffer = PythonBuffer::<u8>::get(view.as_any()).unwrap();
            assert_eq!(
                buffer.buf_ptr().cast_const().cast::<u8>(),
                binary.as_bytes().as_ptr()
            );

            let tail = PyMemoryView::from(binary.as_any())
                .unwrap()
                .get_item(PySlice::new(py, 8, 13, 1))
                .unwrap();
            let input = tail.extract::<PyBuffer>().unwrap();
            assert_eq!(input.0.as_ptr(), binary.as_bytes()[8..].as_ptr());

            // A bytearray may change after extraction, so its views are copied.
            let array = PyByteArray::new(py, b"mutable");
            let input = PyMemoryView::from(array.as_any())
                .unwrap()
                .extract::<PyBuffer>()
                .unwrap();
            array.set_item(0, b'M').unwrap();
            assert_eq!(input.0[..], *b"mutable");

            let text = PyString::new(py, "builtin text");
            let input = text.extract::<Text>().unwrap();
            assert_eq!(input.0.as_ptr(), text.to_str().unwrap().as_ptr());
        });
    }
}
