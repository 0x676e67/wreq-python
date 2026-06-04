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
use pyo3::{ffi, prelude::*};
use wreq::header::{HeaderCaseName, HeaderName, HeaderValue};

/// [`PyBuffer`] enables zero-copy conversion of Rust [`Bytes`] to Python bytes.
pub struct PyBuffer(BufferView);

#[pyclass(frozen, skip_from_py_object)]
struct BufferView(Bytes);

// ===== PyBuffer =====

impl<'a> IntoPyObject<'a> for PyBuffer {
    type Target = PyAny;
    type Output = Bound<'a, Self::Target>;
    type Error = PyErr;

    #[inline(always)]
    fn into_pyobject(self, py: Python<'a>) -> Result<Self::Output, Self::Error> {
        let buffer = self.0.into_pyobject(py)?;
        #[allow(unsafe_code)]
        unsafe {
            Bound::from_owned_ptr_or_err(py, ffi::PyBytes_FromObject(buffer.as_ptr()))
        }
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

// ===== BufferView =====

#[pymethods]
impl BufferView {
    #[allow(unsafe_code)]
    unsafe fn __getbuffer__(
        slf: PyRef<Self>,
        view: *mut ffi::Py_buffer,
        flags: c_int,
    ) -> PyResult<()> {
        let bytes = &slf.0;
        let ret = unsafe {
            // Fill the Py_buffer struct with information about the buffer
            ffi::PyBuffer_FillInfo(
                view,
                slf.as_ptr() as *mut _,
                bytes.as_ptr() as *mut _,
                bytes.len() as _,
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
