use bytes::Bytes;
use pyo3::{
    prelude::*,
    pybacked::{PyBackedBytes, PyBackedStr},
    types::{PyDict, PyIterator, PyList},
};
use wreq::header::{self, HeaderName, HeaderValue};

use crate::{buffer::PyBuffer, error::Error};

/// A HTTP header map.
#[derive(Clone, Default)]
#[pyclass(subclass, str, skip_from_py_object)]
pub struct HeaderMap(pub header::HeaderMap);

/// A HTTP original header map.
#[derive(Clone)]
#[pyclass(subclass, str, skip_from_py_object)]
pub struct OrigHeaderMap(pub header::OrigHeaderMap);

/// An update to apply to a `HeaderMap`.
///
/// A `Client` replays one of these onto its own headers, so that every operation has a
/// single implementation whether it is applied to a plain map or to a client.
pub enum HeaderUpdate {
    Extend(HeaderMap),
    Insert(PyBackedStr, PyBackedStr),
    Append(PyBackedStr, PyBackedStr),
    Remove(PyBackedStr),
    Clear,
}

// ===== impl HeaderMap =====

#[pymethods]
impl HeaderMap {
    /// Creates a new `HeaderMap` from an optional dictionary.
    #[new]
    #[pyo3(signature = (dict=None, capacity=None))]
    fn new(dict: Option<&Bound<'_, PyDict>>, capacity: Option<usize>) -> HeaderMap {
        let mut headers = capacity
            .map(header::HeaderMap::with_capacity)
            .unwrap_or_default();

        // This section of memory might be retained by the Rust object,
        // and we want to prevent Python's garbage collector from managing it.
        if let Some(dict) = dict {
            for (name, value) in dict.iter() {
                let name = match name
                    .extract::<PyBackedStr>()
                    .map(|n| HeaderName::from_bytes(n.as_bytes()))
                {
                    Ok(Ok(n)) => n,
                    _ => continue,
                };

                let value = match value
                    .extract::<PyBackedStr>()
                    .map(Bytes::from_owner)
                    .map(HeaderValue::from_maybe_shared)
                {
                    Ok(Ok(v)) => v,
                    _ => continue,
                };

                headers.insert(name, value);
            }
        }

        HeaderMap(headers)
    }

    /// Returns a reference to the value associated with the key.
    ///
    /// If there are multiple values associated with the key, then the first one
    /// is returned. Use `get_all` to get all values associated with a given
    /// key. Returns `None` if there are no values associated with the key.
    #[pyo3(signature = (key, default=None))]
    fn get<'py>(
        &self,
        py: Python<'py>,
        key: PyBackedStr,
        default: Option<PyBackedBytes>,
    ) -> Option<PyBuffer> {
        py.detach(|| {
            self.0.get::<&str>(key.as_ref()).cloned().or_else(|| {
                match default
                    .map(Bytes::from_owner)
                    .map(HeaderValue::from_maybe_shared)
                {
                    Some(Ok(v)) => Some(v),
                    _ => None,
                }
            })
        })
        .map(PyBuffer::from)
    }

    /// Returns a view of all values associated with a key.
    #[pyo3(signature = (key))]
    fn get_all<'py>(&self, py: Python<'py>, key: PyBackedStr) -> Vec<PyBuffer> {
        py.detach(|| {
            self.0
                .get_all::<&str>(key.as_ref())
                .iter()
                .cloned()
                .map(PyBuffer::from)
                .collect()
        })
    }

    /// Insert a key-value pair into the header map.
    #[pyo3(signature = (key, value))]
    fn insert(&mut self, py: Python, key: PyBackedStr, value: PyBackedStr) {
        py.detach(|| HeaderUpdate::Insert(key, value).apply(self))
    }

    /// Append a key-value pair to the header map.
    #[pyo3(signature = (key, value))]
    fn append(&mut self, py: Python, key: PyBackedStr, value: PyBackedStr) {
        py.detach(|| HeaderUpdate::Append(key, value).apply(self))
    }

    /// Extend the header map with the given headers.
    ///
    /// The value of a key that is already present is replaced, and a key that is missing
    /// is added.
    #[pyo3(signature = (headers))]
    fn update(&mut self, py: Python, headers: HeaderMap) {
        py.detach(|| HeaderUpdate::Extend(headers).apply(self))
    }

    /// Remove a key-value pair from the header map.
    #[pyo3(signature = (key))]
    fn remove(&mut self, py: Python, key: PyBackedStr) {
        py.detach(|| HeaderUpdate::Remove(key).apply(self))
    }

    /// Returns true if the map contains a value for the specified key.
    #[pyo3(signature = (key))]
    fn contains_key(&self, py: Python, key: PyBackedStr) -> bool {
        py.detach(|| self.0.contains_key::<&str>(key.as_ref()))
    }

    /// An iterator visiting all keys.
    #[inline]
    fn keys<'py>(&self, py: Python<'py>) -> Vec<PyBuffer> {
        py.detach(|| {
            self.0
                .keys()
                .cloned()
                .map(PyBuffer::from)
                .collect::<Vec<_>>()
        })
    }

    ///  An iterator visiting all values.
    #[inline]
    fn values<'py>(&self, py: Python<'py>) -> Vec<PyBuffer> {
        py.detach(|| {
            self.0
                .values()
                .cloned()
                .map(PyBuffer::from)
                .collect::<Vec<_>>()
        })
    }

    /// Returns the number of headers stored in the map.
    ///
    /// This number represents the total number of **values** stored in the map.
    /// This number can be greater than or equal to the number of **keys**
    /// stored given that a single key may have more than one associated value.
    #[inline]
    fn len(&self) -> usize {
        self.0.len()
    }

    /// Returns the number of keys stored in the map.
    ///
    /// This number will be less than or equal to `len()` as each key may have
    /// more than one associated value.
    #[inline]
    fn keys_len(&self) -> usize {
        self.0.keys_len()
    }

    /// Returns true if the map contains no elements.
    #[inline]
    fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// Clears the map, removing all key-value pairs. Keeps the allocated memory for reuse.
    #[inline]
    fn clear(&mut self) {
        HeaderUpdate::Clear.apply(self)
    }
}

#[pymethods]
impl HeaderMap {
    #[inline]
    fn __getitem__<'py>(&self, py: Python<'py>, key: PyBackedStr) -> Option<PyBuffer> {
        self.get(py, key, None)
    }

    #[inline]
    fn __setitem__(&mut self, py: Python, key: PyBackedStr, value: PyBackedStr) {
        self.insert(py, key, value);
    }

    #[inline]
    fn __delitem__(&mut self, py: Python, key: PyBackedStr) {
        self.remove(py, key);
    }

    #[inline]
    fn __contains__(&self, py: Python, key: PyBackedStr) -> bool {
        self.contains_key(py, key)
    }

    #[inline]
    fn __len__(&self) -> usize {
        self.0.len()
    }

    fn __iter__<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyIterator>> {
        let items: Vec<_> = py.detach(|| {
            self.0
                .iter()
                .map(|(k, v)| (PyBuffer::from(k.clone()), PyBuffer::from(v.clone())))
                .collect()
        });
        let pylist = PyList::new(py, items)?;
        PyIterator::from_object(&pylist)
    }
}

impl_print_str!(Debug, HeaderMap);

impl FromPyObject<'_, '_> for HeaderMap {
    type Error = PyErr;

    fn extract(ob: Borrowed<PyAny>) -> PyResult<Self> {
        if let Ok(headers) = ob.cast::<HeaderMap>() {
            return Ok(Self(headers.borrow().0.clone()));
        }

        let dict = ob.cast::<PyDict>()?;
        dict.iter()
            .try_fold(
                header::HeaderMap::with_capacity(dict.len()),
                |mut headers, (name, value)| {
                    let name = {
                        let name = name.extract::<PyBackedStr>()?;
                        HeaderName::from_bytes(name.as_bytes()).map_err(Error::from)?
                    };

                    let value = {
                        let value = value.extract::<PyBackedStr>()?;
                        HeaderValue::from_maybe_shared(Bytes::from_owner(value))
                            .map_err(Error::from)?
                    };

                    headers.insert(name, value);
                    Ok(headers)
                },
            )
            .map(Self)
    }
}

// ===== impl HeaderUpdate =====

impl HeaderUpdate {
    /// Applies the update to the given headers, ignoring an invalid header name or value.
    pub fn apply(self, headers: &mut HeaderMap) {
        match self {
            HeaderUpdate::Extend(other) => headers.0.extend(other.0),
            HeaderUpdate::Insert(key, value) => {
                if let Some((name, value)) = header_pair(key, value) {
                    headers.0.insert(name, value);
                }
            }
            HeaderUpdate::Append(key, value) => {
                if let Some((name, value)) = header_pair(key, value) {
                    headers.0.append(name, value);
                }
            }
            HeaderUpdate::Remove(key) => {
                headers.0.remove::<&str>(key.as_ref());
            }
            HeaderUpdate::Clear => headers.0.clear(),
        }
    }
}

/// Converts a key-value pair into a header name and value, discarding an invalid one.
fn header_pair(key: PyBackedStr, value: PyBackedStr) -> Option<(HeaderName, HeaderValue)> {
    match (
        HeaderName::from_bytes(key.as_bytes()),
        HeaderValue::from_maybe_shared(Bytes::from_owner(value)),
    ) {
        (Ok(name), Ok(value)) => Some((name, value)),
        _ => None,
    }
}

// ===== impl OrigHeaderMap =====

#[pymethods]
impl OrigHeaderMap {
    /// Creates a new `OrigHeaderMap` from an optional list of header names.
    #[new]
    #[pyo3(signature = (init=None, capacity=None))]
    fn new(init: Option<&Bound<'_, PyList>>, capacity: Option<usize>) -> OrigHeaderMap {
        let mut headers = capacity
            .map(header::OrigHeaderMap::with_capacity)
            .unwrap_or_default();

        // This section of memory might be retained by the Rust object,
        // and we want to prevent Python's garbage collector from managing it.
        if let Some(init) = init {
            for name in init.iter() {
                let name = match name.extract::<PyBackedStr>() {
                    Ok(n) => Bytes::from_owner(n),
                    _ => continue,
                };

                headers.insert(name);
            }
        }

        OrigHeaderMap(headers)
    }

    /// Insert a new header name into the collection.
    ///
    /// If the map did not previously have this key present, then `false` is
    /// returned.
    ///
    /// If the map did have this key present, the new value is pushed to the end
    /// of the list of values currently associated with the key. The key is not
    /// updated, though; this matters for types that can be `==` without being
    /// identical.
    #[inline]
    pub fn insert(&mut self, value: PyBackedStr) -> bool {
        self.0.insert(Bytes::from_owner(value))
    }

    /// Extends the map with all entries from another [`OrigHeaderMap`], preserving order.
    #[inline]
    pub fn extend(&mut self, iter: &Bound<'_, OrigHeaderMap>) {
        self.0.extend(iter.borrow().0.clone());
    }
}

#[pymethods]
impl OrigHeaderMap {
    #[inline]
    fn __len__(&self) -> usize {
        self.0.len()
    }

    fn __iter__<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyIterator>> {
        let items: Vec<_> = py.detach(|| {
            self.0
                .iter()
                .map(|(name, orig_name)| {
                    let name = PyBuffer::from(name.clone());
                    let orig_name = PyBuffer::from(orig_name.clone());
                    (name, orig_name)
                })
                .collect()
        });
        let pylist = PyList::new(py, items)?;
        PyIterator::from_object(&pylist)
    }
}

impl_print_str!(Debug, OrigHeaderMap);

impl FromPyObject<'_, '_> for OrigHeaderMap {
    type Error = PyErr;

    fn extract(ob: Borrowed<PyAny>) -> PyResult<Self> {
        if let Ok(headers) = ob.cast::<OrigHeaderMap>() {
            return Ok(Self(headers.borrow().0.clone()));
        }

        let list = ob.cast::<PyList>()?;
        list.iter()
            .try_fold(
                header::OrigHeaderMap::with_capacity(list.len()),
                |mut headers, name| {
                    let name = {
                        let name = name.extract::<PyBackedStr>()?;
                        Bytes::from_owner(name)
                    };
                    headers.insert(name);
                    Ok(headers)
                },
            )
            .map(Self)
    }
}
