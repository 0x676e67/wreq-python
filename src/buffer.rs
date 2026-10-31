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

use bytes::{Buf, Bytes};
use pyo3::{
    PyTypeCheck, PyTypeInfo,
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
    /// Extracts `bytes`, `bytearray` or `memoryview` data in one pass over their types, or
    /// returns `None` for any other type without building an error, so callers can fall back.
    pub fn extract_bytes_like(ob: &Bound<PyAny>) -> Option<PyResult<Self>> {
        let bytes = if let Some(bytes) = as_exact::<PyBytes>(ob) {
            let owner = PyBackedBytes::from(bytes.clone());
            share(0..owner.len(), owner)
        } else if let Some(view) = as_exact::<PyMemoryView>(ob) {
            return Some(view_bytes(view).map(Self));
        } else if let Some(array) = as_instance::<PyByteArray>(ob) {
            // A bytearray may change after extraction, so it is copied once.
            Bytes::from(array.to_vec())
        } else {
            // Subclasses can reference an exported view; Bytes owners are invisible to GC.
            Bytes::copy_from_slice(as_instance::<PyBytes>(ob)?.as_bytes())
        };
        Some(Ok(Self(bytes)))
    }
}

impl FromPyObject<'_, '_> for PyBuffer {
    type Error = PyErr;

    fn extract(ob: Borrowed<PyAny>) -> PyResult<Self> {
        match Self::extract_bytes_like(&ob) {
            Some(bytes) => bytes,
            // Other exporters, such as numpy numbers or `array('d')`, count as bytes only in a
            // memoryview, so `body=` and `from_binary` keep encoding them as JSON.
            None => Err(PyTypeError::new_err(format!(
                "'{}' object is not an instance of 'bytes | bytearray | memoryview'",
                ob.get_type().qualname()?
            ))),
        }
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
        slf: &Bound<'_, Self>,
        view: *mut ffi::Py_buffer,
        flags: c_int,
    ) -> PyResult<()> {
        let bytes = &slf.get().0;
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
    // BufferView allows no subclasses, so an exact test covers it.
    if let Some(exporter) = as_exact::<BufferView>(&obj) {
        let bytes = &exporter.get().0;
        if let Some(range) = view_range(view, bytes) {
            // A counted clone of the exporter's Bytes, whatever its vtable.
            return Ok(bytes.slice(range));
        }
    } else if obj.is_exact_instance_of::<PyBytes>()
        && let Ok(bytes) = obj.cast_into_exact::<PyBytes>()
    {
        let owner = PyBackedBytes::from(bytes);
        if let Some(range) = view_range(view, &owner) {
            return Ok(share(range, owner));
        }
    }
    // `bytes()` copies any layout or item format as raw bytes.
    let copy = view
        .py()
        .get_type::<PyBytes>()
        .call1((view,))?
        .cast_into::<PyBytes>()?;
    let owner = PyBackedBytes::from(copy);
    Ok(share(0..owner.len(), owner))
}

/// Shares the `range` of the exact `bytes` behind `owner`, which must contain it. Empty data
/// needs no owner, and the owned `Bytes` narrows in place instead of through a counted clone.
#[inline]
fn share(range: Range<usize>, owner: PyBackedBytes) -> Bytes {
    if range.is_empty() {
        return Bytes::new();
    }
    let mut bytes = Bytes::from_owner(owner);
    bytes.truncate(range.end);
    bytes.advance(range.start);
    bytes
}

/// The range of `data` that a C-contiguous `view` covers, if the view lies within it.
fn view_range(view: &Bound<PyMemoryView>, data: &[u8]) -> Option<Range<usize>> {
    let buffer = PyUntypedBuffer::get(view.as_any()).ok()?;
    let contiguous = buffer.is_c_contiguous();
    let (ptr, len) = (buffer.buf_ptr().addr(), buffer.len_bytes());
    // Released while attached, so `Drop` need not attach to the interpreter.
    buffer.release(view.py());
    if !contiguous {
        return None;
    }
    let start = ptr.checked_sub(data.as_ptr().addr())?;
    let end = start.checked_add(len)?;
    (end <= data.len()).then_some(start..end)
}

/// `ob` as exactly a `T`; other types return `None` without building a cast error.
#[inline]
fn as_exact<'a, 'py, T: PyTypeInfo>(ob: &'a Bound<'py, PyAny>) -> Option<&'a Bound<'py, T>> {
    // The cast repeats the checked type test and cannot fail.
    if ob.is_exact_instance_of::<T>() {
        ob.cast_exact().ok()
    } else {
        None
    }
}

/// `ob` as a `T` or a subclass; other types return `None` without building a cast error.
#[inline]
fn as_instance<'a, 'py, T: PyTypeCheck>(ob: &'a Bound<'py, PyAny>) -> Option<&'a Bound<'py, T>> {
    if ob.is_instance_of::<T>() {
        ob.cast().ok()
    } else {
        None
    }
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

            // Views of an exact bytes share it, whole or narrowed; empty data needs no owner.
            let view = PyMemoryView::from(binary.as_any()).unwrap();
            let input = view.extract::<PyBuffer>().unwrap();
            assert_eq!(input.0.as_ptr(), binary.as_bytes().as_ptr());
            assert_eq!(input.0[..], *b"builtin bytes");
            let word = view.get_item(PySlice::new(py, 8, 12, 1)).unwrap();
            let input = word.extract::<PyBuffer>().unwrap();
            assert_eq!(input.0.as_ptr(), binary.as_bytes()[8..].as_ptr());
            assert_eq!(input.0[..], *b"byte");
            let empty = view.get_item(PySlice::new(py, 3, 3, 1)).unwrap();
            let input = empty.extract::<PyBuffer>().unwrap();
            assert_eq!(input.0.as_ptr(), Bytes::new().as_ptr());
            let input = PyBytes::new(py, b"").extract::<PyBuffer>().unwrap();
            assert_eq!(input.0.as_ptr(), Bytes::new().as_ptr());

            // A bytearray may change after extraction, so its views are copied.
            let array = PyByteArray::new(py, b"mutable");
            let input = PyMemoryView::from(array.as_any())
                .unwrap()
                .extract::<PyBuffer>()
                .unwrap();
            array.set_item(0, b'M').unwrap();
            assert_eq!(input.0[..], *b"mutable");

            // A bytes subclass can reference an exported view, so it is copied as well.
            let module =
                PyModule::from_code(py, c"class Sub(bytes): pass", c"sub.py", c"sub").unwrap();
            let sub = module.getattr("Sub").unwrap().call1((&binary,)).unwrap();
            let input = sub.extract::<PyBuffer>().unwrap();
            let data = sub.cast::<PyBytes>().unwrap().as_bytes();
            assert_ne!(input.0.as_ptr(), data.as_ptr());
            assert_eq!(input.0[..], *data);

            let text = PyString::new(py, "builtin text");
            let input = text.extract::<Text>().unwrap();
            assert_eq!(input.0.as_ptr(), text.to_str().unwrap().as_ptr());
        });
    }
}
