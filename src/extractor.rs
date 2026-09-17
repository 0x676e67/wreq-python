use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};

use pyo3::{FromPyObject, prelude::*};

/// A generic extractor for various types.
#[derive(Clone)]
pub struct Extractor<T>(pub T);

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
