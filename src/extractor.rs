use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};

use bytes::Bytes;
use pyo3::{
    FromPyObject,
    prelude::*,
    pybacked::{PyBackedBytes, PyBackedStr},
    types::{PyBytes, PyString},
};

/// Extracts a foreign type with custom rules, such as an `(IPv4, IPv6)` pair that drops the
/// wrong family.
pub struct Extractor<T>(pub T);

/// Data of a `bytes` or `bytearray`. Exact `bytes` are shared without copying; subclasses
/// are copied so a cycle through the `Bytes` owner cannot hide from the GC.
#[derive(Clone)]
pub struct Binary(pub Bytes);

/// UTF-8 data of a `str`, shared for an exact `str` and copied for a subclass.
#[derive(Clone)]
pub struct Text(pub Bytes);

// ===== impl Binary =====

impl FromPyObject<'_, '_> for Binary {
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

// ===== impl Text =====

impl FromPyObject<'_, '_> for Text {
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
