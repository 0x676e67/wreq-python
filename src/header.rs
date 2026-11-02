use pyo3::{
    prelude::*,
    pybacked::PyBackedStr,
    types::{PyBytes, PyDict, PyIterator, PyList},
};
use wreq::header::{self, HeaderName, HeaderValue};

use crate::{buffer::PyBuffer, error::Error, extractor::Text};

/// An HTTP header map whose names and values are exposed as `bytes`.
#[derive(Clone)]
#[pyclass(subclass, str)]
pub struct HeaderMap(pub header::HeaderMap);

/// An HTTP original header map whose iterator exposes `bytes`.
#[derive(Clone)]
#[pyclass(subclass, str)]
pub struct OrigHeaderMap(pub header::OrigHeaderMap);

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
                    .extract::<Text>()
                    .map(|value| value.0)
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

    /// Returns the value associated with the key as `bytes`.
    ///
    /// If there are multiple values associated with the key, then the first one
    /// is returned. Use `get_all` to get all values associated with a given
    /// key. Returns `default`, or `None`, if there are no values associated with the key.
    #[pyo3(signature = (key, default=None))]
    fn get<'py>(
        &self,
        py: Python<'py>,
        key: PyBackedStr,
        default: Option<PyBuffer>,
    ) -> Option<Bound<'py, PyBytes>> {
        // Header values are short, so a `bytes` copy is cheaper than a shared buffer.
        if let Some(value) = self.0.get::<&str>(key.as_ref()) {
            return Some(PyBytes::new(py, value.as_bytes()));
        }
        HeaderValue::from_maybe_shared(default?.0)
            .ok()
            .map(|value| PyBytes::new(py, value.as_bytes()))
    }

    /// Returns a list of `bytes` for the values associated with a key.
    #[pyo3(signature = (key))]
    fn get_all(&self, key: PyBackedStr) -> Vec<&[u8]> {
        self.0
            .get_all::<&str>(key.as_ref())
            .iter()
            .map(HeaderValue::as_bytes)
            .collect()
    }

    /// Insert a key-value pair into the header map.
    #[pyo3(signature = (key, value))]
    fn insert(&mut self, key: PyBackedStr, value: Text) {
        if let (Ok(name), Ok(value)) = (
            HeaderName::from_bytes(key.as_bytes()),
            HeaderValue::from_maybe_shared(value.0),
        ) {
            self.0.insert(name, value);
        }
    }

    /// Append a key-value pair to the header map.
    #[pyo3(signature = (key, value))]
    fn append(&mut self, key: PyBackedStr, value: Text) {
        if let (Ok(name), Ok(value)) = (
            HeaderName::from_bytes(key.as_bytes()),
            HeaderValue::from_maybe_shared(value.0),
        ) {
            self.0.append(name, value);
        }
    }

    /// Remove a key-value pair from the header map.
    #[pyo3(signature = (key))]
    fn remove(&mut self, key: PyBackedStr) {
        self.0.remove::<&str>(key.as_ref());
    }

    /// Returns true if the map contains a value for the specified key.
    #[pyo3(signature = (key))]
    fn contains_key(&self, key: PyBackedStr) -> bool {
        self.0.contains_key::<&str>(key.as_ref())
    }

    /// Returns a list of `bytes` for all keys.
    fn keys(&self) -> Vec<&[u8]> {
        self.0.keys().map(|name| name.as_str().as_bytes()).collect()
    }

    /// Returns a list of `bytes` for all values.
    fn values(&self) -> Vec<&[u8]> {
        self.0.values().map(HeaderValue::as_bytes).collect()
    }

    /// Returns the number of headers stored in the map.
    ///
    /// This number represents the total number of **values** stored in the map.
    /// This number can be greater than or equal to the number of **keys**
    /// stored given that a single key may have more than one associated value.
    fn len(&self) -> usize {
        self.0.len()
    }

    /// Returns the number of keys stored in the map.
    ///
    /// This number will be less than or equal to `len()` as each key may have
    /// more than one associated value.
    fn keys_len(&self) -> usize {
        self.0.keys_len()
    }

    /// Returns true if the map contains no elements.
    fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// Clears the map, removing all key-value pairs. Keeps the allocated memory for reuse.
    fn clear(&mut self) {
        self.0.clear();
    }
}

#[pymethods]
impl HeaderMap {
    fn __getitem__<'py>(&self, py: Python<'py>, key: PyBackedStr) -> Option<Bound<'py, PyBytes>> {
        self.get(py, key, None)
    }

    fn __setitem__(&mut self, key: PyBackedStr, value: Text) {
        self.insert(key, value);
    }

    fn __delitem__(&mut self, key: PyBackedStr) {
        self.remove(key);
    }

    fn __contains__(&self, key: PyBackedStr) -> bool {
        self.contains_key(key)
    }

    fn __len__(&self) -> usize {
        self.0.len()
    }

    fn __iter__<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyIterator>> {
        let items = self
            .0
            .iter()
            .map(|(k, v)| (k.as_str().as_bytes(), v.as_bytes()));
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
                        let value = value.extract::<Text>()?;
                        HeaderValue::from_maybe_shared(value.0).map_err(Error::from)?
                    };

                    headers.insert(name, value);
                    Ok(headers)
                },
            )
            .map(Self)
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
                let name = match name.extract::<Text>() {
                    Ok(name) => name.0,
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
    pub fn insert(&mut self, value: Text) -> bool {
        self.0.insert(value.0)
    }

    /// Extends the map with all entries from another [`OrigHeaderMap`], preserving order.
    pub fn extend(&mut self, iter: &Bound<'_, OrigHeaderMap>) {
        self.0.extend(iter.borrow().0.clone());
    }
}

#[pymethods]
impl OrigHeaderMap {
    fn __len__(&self) -> usize {
        self.0.len()
    }

    fn __iter__<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyIterator>> {
        let items = self
            .0
            .iter()
            .map(|(name, orig_name)| (name.as_str().as_bytes(), orig_name.as_ref()));
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
                        let name = name.extract::<Text>()?;
                        name.0
                    };
                    headers.insert(name);
                    Ok(headers)
                },
            )
            .map(Self)
    }
}
