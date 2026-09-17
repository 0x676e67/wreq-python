use bytes::Bytes;
use pyo3::{prelude::*, pybacked::PyBackedStr};
use wreq::header::HeaderValue;

use crate::{error::Error, header::HeaderMap};

/// A builder for `Proxy`.
#[derive(Default)]
struct Builder {
    // Optional username for proxy authentication.
    username: Option<PyBackedStr>,

    // Optional password for proxy authentication.
    password: Option<PyBackedStr>,

    // Optional custom HTTP authentication header.
    custom_http_auth: Option<PyBackedStr>,

    /// Optional custom HTTP headers for the proxy.
    custom_http_headers: Option<HeaderMap>,

    // Optional exclusion list for the proxy.
    exclusion: Option<PyBackedStr>,
}

/// A proxy server for a request.
/// Supports HTTP, HTTPS, SOCKS4, SOCKS4a, SOCKS5, and SOCKS5h protocols.
#[derive(Clone)]
#[pyclass(subclass, frozen, str, from_py_object)]
pub struct Proxy(pub wreq::Proxy);

/// A helper struct to allow parsing either a single proxy or a sequence of proxies.
#[derive(Clone)]
pub struct Proxies(pub Vec<Proxy>);

// ===== impl Builder =====

impl FromPyObject<'_, '_> for Builder {
    type Error = PyErr;

    fn extract(ob: Borrowed<PyAny>) -> PyResult<Self> {
        let mut builder = Self::default();
        extract_option!(ob, builder, username);
        extract_option!(ob, builder, password);
        extract_option!(ob, builder, custom_http_auth);
        extract_option!(ob, builder, custom_http_headers);
        extract_option!(ob, builder, exclusion);
        Ok(builder)
    }
}

// ===== impl Proxy =====

#[pymethods]
impl Proxy {
    /// Creates a new HTTP proxy.
    ///
    /// This method sets up a proxy server for HTTP requests.
    #[staticmethod]
    #[pyo3(signature = (url, **kwds))]
    fn http(py: Python, url: &str, kwds: Option<Builder>) -> PyResult<Self> {
        create_proxy(py, wreq::Proxy::http, url, kwds)
    }

    /// Creates a new HTTPS proxy.
    ///
    /// This method sets up a proxy server for HTTPS requests.
    #[staticmethod]
    #[pyo3(signature = (url, **kwds))]
    fn https(py: Python, url: &str, kwds: Option<Builder>) -> PyResult<Self> {
        create_proxy(py, wreq::Proxy::https, url, kwds)
    }

    /// Creates a new proxy for all protocols.
    ///
    /// This method sets up a proxy server for all types of requests (HTTP, HTTPS, etc.).
    #[staticmethod]
    #[pyo3(signature = (url, **kwds))]
    fn all(py: Python, url: &str, kwds: Option<Builder>) -> PyResult<Self> {
        create_proxy(py, wreq::Proxy::all, url, kwds)
    }

    /// Creates a new UNIX domain socket proxy.
    #[staticmethod]
    #[pyo3(signature = (path, **kwds))]
    #[cfg_attr(not(unix), allow(unused))]
    fn unix(py: Python, path: &str, kwds: Option<Builder>) -> PyResult<Self> {
        #[cfg(not(unix))]
        {
            Err(pyo3::exceptions::PyRuntimeError::new_err(
                "UNIX domain socket proxies are not supported on this platform.",
            ))
        }

        #[cfg(unix)]
        create_proxy(py, wreq::Proxy::unix, path, kwds)
    }
}

impl_print_str!(Debug, Proxy);

// ===== impl Proxies =====

impl FromPyObject<'_, '_> for Proxies {
    type Error = PyErr;

    fn extract(ob: Borrowed<PyAny>) -> PyResult<Self> {
        if let Ok(proxy) = ob.extract::<Proxy>() {
            return Ok(Proxies(vec![proxy]));
        }

        ob.extract::<Vec<Proxy>>().map(Proxies)
    }
}

fn create_proxy<'py>(
    py: Python<'py>,
    proxy_fn: fn(&'py str) -> wreq::Result<wreq::Proxy>,
    url: &'py str,
    builder: Option<Builder>,
) -> PyResult<Proxy> {
    py.detach(|| {
        // Create base proxy using the provided constructor (http, https, all)
        let mut proxy = proxy_fn(url).map_err(Error::Library)?;

        if let Some(builder) = builder {
            // Convert the username and password to a basic auth header value.
            if let Some((username, password)) = builder.username.zip(builder.password) {
                proxy = proxy.basic_auth(username.as_ref(), password.as_ref());
            }

            // Convert the custom HTTP auth string to a header value.
            if let Some(Ok(custom_http_auth)) = builder
                .custom_http_auth
                .map(Bytes::from_owner)
                .map(HeaderValue::from_maybe_shared)
            {
                proxy = proxy.custom_http_auth(custom_http_auth);
            }

            // Convert the custom HTTP headers to a HeaderMap instance.
            if let Some(custom_http_headers) = builder.custom_http_headers {
                proxy = proxy.custom_http_headers(custom_http_headers.0);
            }

            // Convert the exclusion list string to a NoProxy instance.
            if let Some(exclusion) = builder.exclusion {
                proxy = proxy.no_proxy(wreq::NoProxy::from_string(exclusion.as_ref()));
            }
        }

        Ok(Proxy(proxy))
    })
}
