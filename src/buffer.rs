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

use std::os::raw::c_int;

use bytes::Bytes;
use pyo3::{exceptions::PyOverflowError, ffi, prelude::*, types::PyMemoryView};
use wreq::header::{HeaderCaseName, HeaderName, HeaderValue};

/// Exposes owned Rust bytes as a read-only Python memoryview without copying.
pub struct PyBuffer(BufferView);

#[pyclass(frozen, skip_from_py_object)]
struct BufferView(Bytes);

// ===== impl PyBuffer =====

impl<'a> IntoPyObject<'a> for PyBuffer {
    type Target = PyMemoryView;
    type Output = Bound<'a, Self::Target>;
    type Error = PyErr;

    #[inline(always)]
    fn into_pyobject(self, py: Python<'a>) -> Result<Self::Output, Self::Error> {
        let buffer = self.0.into_pyobject(py)?;
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
        PyBuffer(BufferView(value))
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

#[cfg(test)]
mod tests {
    use pyo3::buffer::PyBuffer as PythonBuffer;

    use super::*;

    #[test]
    fn memoryview_shares_owned_bytes() {
        Python::initialize();
        Python::attach(|py| {
            let bytes = Bytes::from(vec![0, 1, 255]);
            let ptr = bytes.as_ptr();
            let view = PyBuffer::from(bytes).into_pyobject(py).unwrap();
            let buffer = PythonBuffer::<u8>::get(view.as_any()).unwrap();

            assert_eq!(buffer.buf_ptr().cast_const().cast::<u8>(), ptr);
            assert!(buffer.readonly());
            assert_eq!(buffer.to_vec(py).unwrap(), [0, 1, 255]);
        });
    }
}
