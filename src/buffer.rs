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

use std::{
    ffi::CString,
    ops::{Deref, Range},
    os::raw::c_int,
    sync::atomic::{AtomicIsize, AtomicUsize, Ordering},
};

use bytes::{Buf, Bytes};
use memchr::memmem;
use pyo3::{
    IntoPyObjectExt, PyTypeCheck, PyTypeInfo,
    buffer::PyUntypedBuffer,
    exceptions::{
        PyDeprecationWarning, PyIndexError, PyMemoryError, PyOverflowError, PyTypeError,
        PyValueError,
    },
    ffi, intern,
    prelude::*,
    pybacked::PyBackedBytes,
    pyclass::CompareOp,
    sync::PyOnceLock,
    types::{
        PyBool, PyByteArray, PyBytes, PyInt, PyMemoryView, PySlice, PyString, PyTuple, PyType,
        slice::PySliceIndices,
    },
};

/// Bytes shared with Python in both directions: extracted from `bytes`, `bytearray`,
/// `memoryview` or `wreq.Bytes` input and returned as an immutable `wreq.Bytes`.
///
/// Input is shared only when an exact `bytes`, a `wreq.Bytes` or a memoryview of either owns
/// it. Mutable exporters and subclasses are copied, so a `Bytes` never aliases writable memory
/// or hides a reference cycle from the GC.
#[derive(Clone)]
pub struct PyBuffer(pub Bytes);

/// Immutable binary data shared without copying. It exports a read-only buffer and compares
/// and hashes like `bytes`; `bytes(data)` makes a copy.
// Its owners are exact `bytes` or Rust data, so it cannot hide a reference cycle, and it
// allows no subclasses, so an exact type test finds every instance.
#[pyclass(name = "Bytes", module = "wreq", frozen, sequence, weakref)]
pub struct SharedBytes {
    bytes: Bytes,
    /// `hash(bytes(self))` once computed, otherwise [`UNHASHED`].
    hash: AtomicIsize,
}

/// Iterates the ints of a `wreq.Bytes`; threads sharing it each take distinct bytes.
#[pyclass(name = "BytesIterator", module = "wreq", frozen)]
struct BytesIter {
    bytes: Bytes,
    next: AtomicUsize,
}

/// A `start` or `end` argument. Ints beyond `isize` clamp, as CPython clamps slice indices.
#[derive(Clone, Copy)]
struct SliceIndex(isize);

/// Bytes-like data read in place, or extracted when it cannot be.
enum Operand<'a> {
    Borrowed(&'a [u8]),
    Owned(Bytes),
}

/// What `find`, `count` and `in` look for: a subsequence or a single byte.
enum Needle<'a> {
    Byte(u8),
    Seq(Operand<'a>),
}

/// Python hashes are never `-1`, which marks an uncomputed hash.
const UNHASHED: isize = -1;

// ===== impl PyBuffer =====

impl PyBuffer {
    /// Extracts `bytes`, `bytearray`, `memoryview` or `wreq.Bytes` data in one pass over their
    /// types, or returns `None` for any other type without building an error, so callers can
    /// fall back.
    pub fn extract_bytes_like(ob: &Bound<PyAny>) -> Option<PyResult<Self>> {
        let bytes = if let Some(bytes) = as_exact::<PyBytes>(ob) {
            let owner = PyBackedBytes::from(bytes.clone());
            share(0..owner.len(), owner)
        } else if let Some(shared) = as_exact::<SharedBytes>(ob) {
            // A counted clone, without a round trip through the buffer protocol.
            shared.get().bytes.clone()
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
                "'{}' object is not an instance of 'bytes | bytearray | memoryview | Bytes'",
                ob.get_type().qualname()?
            ))),
        }
    }
}

impl<'a> IntoPyObject<'a> for PyBuffer {
    type Target = SharedBytes;
    type Output = Bound<'a, Self::Target>;
    type Error = PyErr;

    #[inline(always)]
    fn into_pyobject(self, py: Python<'a>) -> Result<Self::Output, Self::Error> {
        Bound::new(py, SharedBytes::from(self.0))
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

impl AsRef<[u8]> for PyBuffer {
    #[inline]
    fn as_ref(&self) -> &[u8] {
        &self.0
    }
}

// ===== impl SharedBytes =====

impl From<Bytes> for SharedBytes {
    #[inline]
    fn from(bytes: Bytes) -> Self {
        Self {
            bytes,
            hash: AtomicIsize::new(UNHASHED),
        }
    }
}

impl SharedBytes {
    /// The first index of `sub` within `[start:end]`.
    fn position(
        &self,
        sub: &Bound<PyAny>,
        start: Option<SliceIndex>,
        end: Option<SliceIndex>,
    ) -> PyResult<Option<usize>> {
        // `sub` is checked even when the window is out of range, as `bytes` checks it.
        let needle = needle(sub, false)?;
        Ok(window(&self.bytes, start, end)
            .and_then(|(offset, data)| Some(offset.saturating_add(needle.find(data)?))))
    }

    /// Whether `test` holds within `[start:end]` for one bytes-like `affix` or any item of a
    /// tuple of them.
    fn test_affix(
        &self,
        method: &str,
        affix: &Bound<PyAny>,
        start: Option<SliceIndex>,
        end: Option<SliceIndex>,
        test: fn(&[u8], &[u8]) -> bool,
    ) -> PyResult<bool> {
        // A window out of range matches nothing, though every affix is still checked.
        let data = window(&self.bytes, start, end).map(|(_, data)| data);
        let matches = |affix: &[u8]| data.is_some_and(|data| test(data, affix));
        let Ok(affixes) = affix.cast::<PyTuple>() else {
            return match operand(affix) {
                Some(affix) => Ok(matches(&affix?)),
                None => Err(PyTypeError::new_err(format!(
                    "{method} first arg must be bytes or a tuple of bytes, not {}",
                    affix.get_type().qualname()?
                ))),
            };
        };
        for affix in affixes {
            if matches(&operand(&affix).ok_or_else(|| not_bytes_like(&affix))??) {
                return Ok(true);
            }
        }
        Ok(false)
    }
}

#[pymethods]
impl SharedBytes {
    /// Shares an exact `bytes`, a `Bytes` or a contiguous view of either, keeping its whole
    /// source alive, and copies any other buffer; a `Bytes` comes back as the same object.
    #[new]
    #[pyo3(signature = (data = None, /))]
    fn new<'py>(py: Python<'py>, data: Option<&Bound<'py, PyAny>>) -> PyResult<Bound<'py, Self>> {
        let Some(data) = data else {
            return Bound::new(py, Self::from(Bytes::new()));
        };
        if let Some(shared) = as_exact::<Self>(data) {
            return Ok(shared.clone());
        }
        let bytes = match PyBuffer::extract_bytes_like(data) {
            Some(bytes) => bytes?.0,
            None => buffer_like(data).ok_or_else(|| not_bytes_like(data))??,
        };
        Bound::new(py, Self::from(bytes))
    }

    /// # Safety
    /// `view` must be a writable `Py_buffer` supplied by the Python buffer protocol.
    #[allow(unsafe_code)]
    unsafe fn __getbuffer__(
        slf: &Bound<'_, Self>,
        view: *mut ffi::Py_buffer,
        flags: c_int,
    ) -> PyResult<()> {
        let bytes = &slf.get().bytes;
        let len = py_len(bytes)?;
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

    fn __len__(&self) -> usize {
        self.bytes.len()
    }

    /// An int for an index; a shared `Bytes` for a contiguous slice, a copy for a strided one.
    fn __getitem__<'py>(
        slf: &Bound<'py, Self>,
        key: &Bound<'py, PyAny>,
    ) -> PyResult<Bound<'py, PyAny>> {
        let py = slf.py();
        let bytes = &slf.get().bytes;
        let Ok(slice) = key.cast::<PySlice>() else {
            let Some(index) = as_index(key)? else {
                return Err(PyTypeError::new_err(format!(
                    "byte indices must be integers or slices, not {}",
                    key.get_type().qualname()?
                )));
            };
            let index = index.extract::<isize>().map_err(|err| {
                if err.is_instance_of::<PyOverflowError>(py) {
                    PyIndexError::new_err("cannot fit 'int' into an index-sized integer")
                } else {
                    err
                }
            })?;
            let index = if index < 0 {
                bytes.len().checked_sub(index.unsigned_abs())
            } else {
                Some(index.unsigned_abs())
            };
            return index
                .and_then(|index| bytes.get(index))
                .ok_or_else(|| PyIndexError::new_err("index out of range"))?
                .into_bound_py_any(py);
        };
        let PySliceIndices {
            start,
            step,
            slicelength,
            ..
        } = slice.indices(py_len(bytes)?)?;
        // `start` is only negative for an empty slice with a negative step.
        let start = usize::try_from(start).unwrap_or_default();
        let sliced = if step == 1 {
            let end = start.saturating_add(slicelength).min(bytes.len());
            bytes.slice(start.min(end)..end)
        } else {
            let mut picked = Vec::new();
            picked
                .try_reserve_exact(slicelength)
                .map_err(|_| PyMemoryError::new_err(()))?;
            let step_by = step.unsigned_abs().max(1);
            if step > 0 {
                let tail = bytes.get(start..).unwrap_or_default();
                picked.extend(tail.iter().step_by(step_by).take(slicelength));
            } else {
                let head = bytes.get(..=start).unwrap_or_default();
                if step == -1 {
                    // Reversing a copied run beats stepping byte by byte.
                    let run = head.len().saturating_sub(slicelength);
                    picked.extend_from_slice(head.get(run..).unwrap_or_default());
                    picked.reverse();
                } else {
                    picked.extend(head.iter().rev().step_by(step_by).take(slicelength));
                }
            }
            Bytes::from(picked)
        };
        Bound::new(py, Self::from(sliced)).map(Bound::into_any)
    }

    fn __iter__(&self) -> BytesIter {
        BytesIter {
            bytes: self.bytes.clone(),
            next: AtomicUsize::new(0),
        }
    }

    fn __contains__(&self, item: &Bound<PyAny>) -> PyResult<bool> {
        Ok(needle(item, true)?.find(&self.bytes).is_some())
    }

    /// The lowest index of `sub` (bytes-like or a byte value) within `[start:end]`, or `-1`.
    #[pyo3(signature = (sub, start = None, end = None, /))]
    fn find(
        &self,
        sub: &Bound<PyAny>,
        start: Option<SliceIndex>,
        end: Option<SliceIndex>,
    ) -> PyResult<isize> {
        Ok(self
            .position(sub, start, end)?
            .and_then(|index| isize::try_from(index).ok())
            .unwrap_or(-1))
    }

    /// Like `find`, but raises `ValueError` when `sub` is not found.
    #[pyo3(signature = (sub, start = None, end = None, /))]
    fn index(
        &self,
        sub: &Bound<PyAny>,
        start: Option<SliceIndex>,
        end: Option<SliceIndex>,
    ) -> PyResult<usize> {
        self.position(sub, start, end)?
            .ok_or_else(|| PyValueError::new_err("subsection not found"))
    }

    /// The number of non-overlapping occurrences of `sub` within `[start:end]`.
    #[pyo3(signature = (sub, start = None, end = None, /))]
    fn count(
        &self,
        sub: &Bound<PyAny>,
        start: Option<SliceIndex>,
        end: Option<SliceIndex>,
    ) -> PyResult<usize> {
        let needle = needle(sub, false)?;
        Ok(window(&self.bytes, start, end).map_or(0, |(_, data)| needle.count(data)))
    }

    /// Compares with `bytes` or `Bytes`; other types reflect, so `bytearray` and `memoryview`
    /// compare through their own methods over this buffer.
    fn __richcmp__(&self, other: &Bound<PyAny>, op: CompareOp) -> Py<PyAny> {
        let py = other.py();
        let other = if let Some(other) = as_exact::<SharedBytes>(other) {
            &other.get().bytes[..]
        } else if let Some(other) = as_instance::<PyBytes>(other) {
            other.as_bytes()
        } else {
            return py.NotImplemented();
        };
        let ours = &self.bytes[..];
        let result = match op {
            CompareOp::Eq => ours == other,
            CompareOp::Ne => ours != other,
            _ => op.matches(ours.cmp(other)),
        };
        PyBool::new(py, result).to_owned().into_any().unbind()
    }

    /// Equals `hash(bytes(self))`, so `Bytes` and equal `bytes` are interchangeable keys.
    #[allow(unsafe_code)]
    fn __hash__(&self, py: Python) -> PyResult<isize> {
        let hash = self.hash.load(Ordering::Relaxed);
        if hash != UNHASHED {
            return Ok(hash);
        }
        let len = py_len(&self.bytes)?;
        // SAFETY: the pointer is valid for `len` immutable bytes while `self` is borrowed. Under
        // the limited API the shim hashes a temporary `bytes` copy; otherwise it reads in place.
        let hash = unsafe { ffi::compat::Py_HashBuffer(self.bytes.as_ptr().cast(), len) };
        if hash == UNHASHED {
            return Err(PyErr::fetch(py));
        }
        // Racing threads store the same value.
        self.hash.store(hash, Ordering::Relaxed);
        Ok(hash)
    }

    fn __bytes__<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyBytes>> {
        to_bytes(py, &self.bytes)
    }

    fn __repr__<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        // The copy drops first, and `str.format` raises `MemoryError` where `format!` aborts.
        let repr = to_bytes(py, &self.bytes)?.repr()?;
        intern!(py, "Bytes({})").call_method1(intern!(py, "format"), (repr,))
    }

    /// Pickles as a `bytes` copy, which unpickling shares again.
    fn __reduce__<'py>(
        slf: &Bound<'py, Self>,
    ) -> PyResult<(Bound<'py, PyType>, (Bound<'py, PyBytes>,))> {
        Ok((slf.get_type(), (to_bytes(slf.py(), &slf.get().bytes)?,)))
    }

    /// Concatenates with any buffer into a new `Bytes`.
    fn __add__<'py>(
        slf: &Bound<'py, Self>,
        other: &Bound<'py, PyAny>,
    ) -> PyResult<Bound<'py, PyAny>> {
        let py = slf.py();
        let Some(tail) = operand(other) else {
            return Ok(py.NotImplemented().into_bound(py));
        };
        let (head, tail) = (&slf.get().bytes, tail?);
        let joined = if tail.is_empty() {
            head.clone()
        } else {
            let mut joined = Vec::new();
            head.len()
                .checked_add(tail.len())
                .and_then(|len| joined.try_reserve_exact(len).ok())
                .ok_or_else(|| PyMemoryError::new_err(()))?;
            joined.extend_from_slice(head);
            joined.extend_from_slice(&tail);
            Bytes::from(joined)
        };
        Bound::new(py, Self::from(joined)).map(Bound::into_any)
    }

    #[pyo3(signature = (encoding = "utf-8", errors = "strict"))]
    fn decode<'py>(
        slf: &Bound<'py, Self>,
        encoding: &str,
        errors: &str,
    ) -> PyResult<Bound<'py, PyString>> {
        if errors == "strict" && matches!(encoding, "utf-8" | "utf8" | "UTF-8" | "UTF8") {
            // CPython validates UTF-8 once while building the string.
            return PyString::from_bytes(slf.py(), &slf.get().bytes);
        }
        // Other codecs decode this read-only buffer in place.
        let encoding = CString::new(encoding)?;
        let errors = CString::new(errors)?;
        PyString::from_encoded_object(slf.as_any(), Some(&encoding), Some(&errors))
    }

    #[pyo3(signature = (sep = None, bytes_per_sep = 1))]
    fn hex<'py>(
        slf: &Bound<'py, Self>,
        sep: Option<&Bound<'py, PyAny>>,
        bytes_per_sep: isize,
    ) -> PyResult<Bound<'py, PyAny>> {
        // `memoryview.hex` reads this buffer in place.
        let view = PyMemoryView::from(slf.as_any())?;
        let hex = intern!(slf.py(), "hex");
        match sep {
            Some(sep) => view.call_method1(hex, (sep, bytes_per_sep)),
            None => view.call_method0(hex),
        }
    }

    /// A `bytes` copy, as `memoryview.tobytes()` returns; cheaper than `bytes(data)`.
    fn tobytes<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyBytes>> {
        to_bytes(py, &self.bytes)
    }

    #[pyo3(signature = (prefix, start = None, end = None, /))]
    fn startswith(
        &self,
        prefix: &Bound<PyAny>,
        start: Option<SliceIndex>,
        end: Option<SliceIndex>,
    ) -> PyResult<bool> {
        self.test_affix("startswith", prefix, start, end, <[u8]>::starts_with)
    }

    #[pyo3(signature = (suffix, start = None, end = None, /))]
    fn endswith(
        &self,
        suffix: &Bound<PyAny>,
        start: Option<SliceIndex>,
        end: Option<SliceIndex>,
    ) -> PyResult<bool> {
        self.test_affix("endswith", suffix, start, end, <[u8]>::ends_with)
    }

    /// Immutable, so a copy is the same object.
    fn __copy__<'py>(slf: &Bound<'py, Self>) -> Bound<'py, Self> {
        slf.clone()
    }

    fn __deepcopy__<'py>(slf: &Bound<'py, Self>, _memo: &Bound<'py, PyAny>) -> Bound<'py, Self> {
        slf.clone()
    }

    /// Kept for code written against `memoryview`; the data is freed when the last reference
    /// goes away.
    fn release(&self, py: Python) -> PyResult<()> {
        PyErr::warn(
            py,
            &py.get_type::<PyDeprecationWarning>(),
            c"Bytes.release() does nothing; drop the reference instead",
            1,
        )
    }

    fn __enter__<'py>(slf: &Bound<'py, Self>) -> PyResult<Bound<'py, Self>> {
        let py = slf.py();
        PyErr::warn(
            py,
            &py.get_type::<PyDeprecationWarning>(),
            c"Bytes is not a context manager; use `with memoryview(data) as view:`",
            1,
        )?;
        Ok(slf.clone())
    }

    #[pyo3(signature = (*_args))]
    fn __exit__(&self, _args: &Bound<PyTuple>) {}
}

// ===== impl BytesIter =====

#[pymethods]
impl BytesIter {
    fn __iter__(slf: PyRef<Self>) -> PyRef<Self> {
        slf
    }

    fn __next__(&self) -> Option<u8> {
        let len = self.bytes.len();
        self.next
            .try_update(Ordering::Relaxed, Ordering::Relaxed, |next| {
                (next < len).then_some(next + 1)
            })
            .ok()
            .and_then(|next| self.bytes.get(next).copied())
    }

    fn __length_hint__(&self) -> usize {
        self.bytes
            .len()
            .saturating_sub(self.next.load(Ordering::Relaxed))
    }
}

// ===== impl SliceIndex =====

impl FromPyObject<'_, '_> for SliceIndex {
    type Error = PyErr;

    fn extract(ob: Borrowed<PyAny>) -> PyResult<Self> {
        let Some(index) = as_index(&ob)? else {
            return Err(PyTypeError::new_err(
                "slice indices must be integers or None or have an __index__ method",
            ));
        };
        match index.extract::<isize>() {
            Ok(index) => Ok(Self(index)),
            Err(err) if err.is_instance_of::<PyOverflowError>(ob.py()) => {
                Ok(Self(if index.lt(0)? { isize::MIN } else { isize::MAX }))
            }
            Err(err) => Err(err),
        }
    }
}

// ===== impl Operand =====

impl Deref for Operand<'_> {
    type Target = [u8];

    #[inline]
    fn deref(&self) -> &[u8] {
        match self {
            Operand::Borrowed(data) => data,
            Operand::Owned(bytes) => bytes,
        }
    }
}

// ===== impl Needle =====

impl<'a> From<Operand<'a>> for Needle<'a> {
    /// A one-byte subsequence searches as that byte, without building a substring finder.
    fn from(seq: Operand<'a>) -> Self {
        if let [byte] = *seq {
            return Needle::Byte(byte);
        }
        Needle::Seq(seq)
    }
}

impl Needle<'_> {
    fn find(&self, haystack: &[u8]) -> Option<usize> {
        match self {
            Needle::Byte(byte) => memchr::memchr(*byte, haystack),
            Needle::Seq(seq) => memmem::find(haystack, seq),
        }
    }

    fn count(&self, haystack: &[u8]) -> usize {
        match self {
            Needle::Byte(byte) => haystack.iter().filter(|&item| item == byte).count(),
            Needle::Seq(seq) => memmem::find_iter(haystack, &**seq).count(),
        }
    }
}

/// The length of `data` as a `Py_ssize_t`.
#[inline]
fn py_len(data: &[u8]) -> PyResult<ffi::Py_ssize_t> {
    ffi::Py_ssize_t::try_from(data.len())
        .map_err(|_| PyOverflowError::new_err("buffer length exceeds Python's maximum size"))
}

/// A `bytes` copy of `data` that raises `MemoryError` where `PyBytes::new` would panic.
#[allow(unsafe_code)]
fn to_bytes<'py>(py: Python<'py>, data: &[u8]) -> PyResult<Bound<'py, PyBytes>> {
    let len = py_len(data)?;
    // SAFETY: `data` is valid for `len` bytes, which CPython copies; NULL comes with an error.
    let copy = unsafe {
        Bound::from_owned_ptr_or_err(
            py,
            ffi::PyBytes_FromStringAndSize(data.as_ptr().cast(), len),
        )?
    };
    Ok(copy.cast_into_exact()?)
}

/// Shares a view of an exact `bytes` or of a `wreq.Bytes`, and copies any other view.
fn view_bytes(view: &Bound<PyMemoryView>) -> PyResult<Bytes> {
    let obj = view.getattr(intern!(view.py(), "obj"))?;
    if let Some(exporter) = as_exact::<SharedBytes>(&obj) {
        let bytes = &exporter.get().bytes;
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

/// `ob` as `bytes`, `bytearray`, `memoryview` or `Bytes` data, read in place where it can be,
/// or `None` for any other type.
fn bytes_like<'a>(ob: &'a Bound<PyAny>) -> Option<PyResult<Operand<'a>>> {
    if let Some(shared) = as_exact::<SharedBytes>(ob) {
        return Some(Ok(Operand::Borrowed(&shared.get().bytes)));
    }
    if let Some(bytes) = as_instance::<PyBytes>(ob) {
        return Some(Ok(Operand::Borrowed(bytes.as_bytes())));
    }
    PyBuffer::extract_bytes_like(ob).map(|bytes| bytes.map(|bytes| Operand::Owned(bytes.0)))
}

/// A raw copy of any other buffer exporter, such as an `array` or numpy data, or `None` for an
/// object without a buffer.
fn buffer_like(ob: &Bound<PyAny>) -> Option<PyResult<Bytes>> {
    match PyMemoryView::from(ob) {
        Ok(view) => Some(view_bytes(&view)),
        Err(err) if err.is_instance_of::<PyTypeError>(ob.py()) => None,
        Err(err) => Some(Err(err)),
    }
}

/// `ob` as any buffer, as `bytes` methods take their operands.
fn operand<'a>(ob: &'a Bound<PyAny>) -> Option<PyResult<Operand<'a>>> {
    bytes_like(ob).or_else(|| buffer_like(ob).map(|bytes| bytes.map(Operand::Owned)))
}

/// The `TypeError` for an operand that exports no buffer.
fn not_bytes_like(ob: &Bound<PyAny>) -> PyErr {
    match ob.get_type().qualname() {
        Ok(name) => PyTypeError::new_err(format!("a bytes-like object is required, not '{name}'")),
        Err(err) => err,
    }
}

/// `sub` as a buffer or a byte value given as an int or `__index__` object. `bytes.__contains__`
/// takes an exporter with `__index__`, such as a numpy int, as a byte (`prefer_byte`); the
/// other methods take its buffer.
fn needle<'a>(sub: &'a Bound<PyAny>, prefer_byte: bool) -> PyResult<Needle<'a>> {
    if let Some(int) = as_instance::<PyInt>(sub) {
        return byte(int).map(Needle::Byte);
    }
    if let Some(seq) = bytes_like(sub) {
        return seq.map(Needle::from);
    }
    if prefer_byte && let Some(int) = as_index(sub)? {
        return byte(&int).map(Needle::Byte);
    }
    if let Some(seq) = buffer_like(sub) {
        return seq.map(|bytes| Needle::from(Operand::Owned(bytes)));
    }
    if prefer_byte {
        return Err(not_bytes_like(sub));
    }
    match as_index(sub)? {
        Some(int) => byte(&int).map(Needle::Byte),
        None => Err(PyTypeError::new_err(format!(
            "argument should be integer or bytes-like object, not '{}'",
            sub.get_type().qualname()?
        ))),
    }
}

/// The byte `int` stands for, with the error `bytes` raises outside `range(0, 256)`.
fn byte(int: &Bound<PyInt>) -> PyResult<u8> {
    int.extract::<u8>().map_err(|err| {
        if err.is_instance_of::<PyOverflowError>(int.py()) {
            PyValueError::new_err("byte must be in range(0, 256)")
        } else {
            err
        }
    })
}

/// `ob` as an int, through `operator.index` unless it is one, or `None` when its type has no
/// `__index__`. Errors raised by `__index__` propagate.
fn as_index<'py>(ob: &Bound<'py, PyAny>) -> PyResult<Option<Bound<'py, PyInt>>> {
    static INDEX: PyOnceLock<Py<PyAny>> = PyOnceLock::new();
    if let Some(int) = as_instance::<PyInt>(ob) {
        return Ok(Some(int.clone()));
    }
    let py = ob.py();
    if !ob.get_type().hasattr(intern!(py, "__index__"))? {
        return Ok(None);
    }
    let int = INDEX.import(py, "operator", "index")?.call1((ob,))?;
    Ok(Some(int.cast_into::<PyInt>()?))
}

/// The `[start:end]` window of `data` and its offset, clamped as `bytes` methods clamp them, or
/// `None` when `start` lies past the end or after `end`.
fn window(
    data: &[u8],
    start: Option<SliceIndex>,
    end: Option<SliceIndex>,
) -> Option<(usize, &[u8])> {
    let len = data.len();
    let clamp = |SliceIndex(index): SliceIndex| {
        if index < 0 {
            len.saturating_sub(index.unsigned_abs())
        } else {
            index.unsigned_abs().min(len)
        }
    };
    let start = start.unwrap_or(SliceIndex(0));
    if start.0 > 0 && start.0.unsigned_abs() > len {
        return None;
    }
    let start = clamp(start);
    let end = end.map_or(len, clamp);
    data.get(start..end).map(|data| (start, data))
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
    use pyo3::buffer::PyBuffer as PythonBuffer;

    use super::*;
    use crate::extractor::Text;

    #[test]
    fn buffers_share_immutable_bytes() {
        Python::initialize();
        Python::attach(|py| {
            let bytes = Bytes::from(vec![0, 1, 255]);
            let ptr = bytes.as_ptr();
            let shared = PyBuffer::from(bytes).into_pyobject(py).unwrap();
            let buffer = PythonBuffer::<u8>::get(shared.as_any()).unwrap();

            assert_eq!(buffer.buf_ptr().cast_const().cast::<u8>(), ptr);
            assert!(buffer.readonly());
            assert_eq!(buffer.to_vec(py).unwrap(), [0, 1, 255]);

            // Our Bytes and their views come back shared, sliced or not; a strided one is
            // copied.
            let input = shared.extract::<PyBuffer>().unwrap();
            assert_eq!(input.0.as_ptr(), ptr);
            let view = PyMemoryView::from(shared.as_any()).unwrap();
            assert_eq!(view.extract::<PyBuffer>().unwrap().0.as_ptr(), ptr);
            let tail = shared.get_item(PySlice::new(py, 1, 3, 1)).unwrap();
            let input = tail.extract::<PyBuffer>().unwrap();
            assert_eq!(input.0.as_ptr(), ptr.wrapping_add(1));
            assert_eq!(input.0[..], [1, 255]);
            let strided = shared.get_item(PySlice::new(py, 0, 3, 2)).unwrap();
            assert_eq!(strided.extract::<PyBuffer>().unwrap().0[..], [0, 255]);

            let binary = PyBytes::new(py, b"builtin bytes");
            let input = binary.extract::<PyBuffer>().unwrap();
            assert_eq!(input.0.as_ptr(), binary.as_bytes().as_ptr());
            let shared = PyBuffer::from(input.0).into_pyobject(py).unwrap();
            let buffer = PythonBuffer::<u8>::get(shared.as_any()).unwrap();
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
