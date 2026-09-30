use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};

use bytes::Bytes;
use pyo3::{
    FromPyObject,
    prelude::*,
    pybacked::{PyBackedBytes, PyBackedStr},
    types::{PyBytes, PyString},
};

/// A generic extractor for various types.
pub struct Extractor<T>(pub T);

/// Byte input with no hidden references to Python subclasses.
#[derive(Clone)]
pub struct BytesInput(pub Bytes);

/// UTF-8 input with no hidden references to Python subclasses.
#[derive(Clone)]
pub struct StrInput(pub Bytes);

// ===== impl BytesInput =====

impl FromPyObject<'_, '_> for BytesInput {
    type Error = PyErr;

    fn extract(ob: Borrowed<PyAny>) -> PyResult<Self> {
        let value = ob.extract::<PyBackedBytes>()?;
        // Subclasses can reference an exported view; Bytes owners are invisible to GC.
        let bytes = if ob.is_instance_of::<PyBytes>() && !ob.is_exact_instance_of::<PyBytes>() {
            Bytes::copy_from_slice(value.as_ref())
        } else {
            Bytes::from_owner(value)
        };
        Ok(Self(bytes))
    }
}

// ===== impl StrInput =====

impl FromPyObject<'_, '_> for StrInput {
    type Error = PyErr;

    fn extract(ob: Borrowed<PyAny>) -> PyResult<Self> {
        let value = ob.extract::<PyBackedStr>()?;
        let bytes = if ob.is_exact_instance_of::<PyString>() {
            Bytes::from_owner(value)
        } else {
            // Copy the actual UTF-8 payload, not a subclass's __str__ result.
            Bytes::copy_from_slice(value.as_bytes())
        };
        Ok(Self(bytes))
    }
}

// ===== impl Extractor =====

impl FromPyObject<'_, '_> for Extractor<(Option<Ipv4Addr>, Option<Ipv6Addr>)> {
    type Error = PyErr;

    fn extract(ob: Borrowed<PyAny>) -> PyResult<Self> {
        let (v4, v6) = ob.extract::<(Option<IpAddr>, Option<IpAddr>)>()?;
        Ok(Self((
            match v4 {
                Some(IpAddr::V4(addr)) => Some(addr),
                _ => None,
            },
            match v6 {
                Some(IpAddr::V6(addr)) => Some(addr),
                _ => None,
            },
        )))
    }
}
