use std::{sync::Arc, time::SystemTime};

use bytes::Bytes;
use cookie::{Cookie as RawCookie, Expiration, ParseError, time::Duration};
use pyo3::{prelude::*, pybacked::PyBackedStr, types::PyDict};
use wreq::header::{self, HeaderMap, HeaderValue};

use crate::error::Error;

define_enum!(
    /// The Cookie SameSite attribute.
    const,
    SameSite,
    cookie::SameSite,
    (Strict, Strict),
    (Lax, Lax),
    (Empty, None),
);

/// A helper enum to allow parsing either a `Cookie` or a cookie string.
#[derive(FromPyObject)]
pub enum PyCookie {
    Cookie(Cookie),
    String(PyBackedStr),
}

/// A single HTTP cookie.
#[derive(Clone)]
#[pyclass(subclass, str, frozen, from_py_object)]
pub struct Cookie(RawCookie<'static>);

/// A helper struct to allow parsing either a single cookie string or multiple cookies from a dict.
pub struct Cookies(pub Vec<HeaderValue>);

/// A good default `CookieStore` implementation.
///
/// This is the implementation used when simply calling `cookie_store(true)`.
/// This type is exposed to allow creating one and filling it with some
/// existing cookies more easily, before creating a `Client`.
#[derive(Clone, Default)]
#[pyclass(subclass, frozen, from_py_object)]
pub struct Jar(pub Arc<wreq::cookie::Jar>);

// ===== impl Cookie =====

#[pymethods]
impl Cookie {
    /// Create a new [`Cookie`].
    #[new]
    #[pyo3(signature = (
        name,
        value,
        domain = None,
        path = None,
        max_age = None,
        expires = None,
        http_only = None,
        secure = None,
        same_site = None
    ))]
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        name: String,
        value: String,
        domain: Option<String>,
        path: Option<String>,
        max_age: Option<std::time::Duration>,
        expires: Option<SystemTime>,
        http_only: Option<bool>,
        secure: Option<bool>,
        same_site: Option<SameSite>,
    ) -> Cookie {
        let mut cookie = RawCookie::new(name, value);

        if let Some(domain) = domain {
            cookie.set_domain(domain);
        }

        if let Some(path) = path {
            cookie.set_path(path);
        }

        if let Some(max_age) = max_age
            && let Ok(max_age) = Duration::try_from(max_age)
        {
            cookie.set_max_age(max_age);
        }

        if let Some(expires) = expires {
            cookie.set_expires(Expiration::DateTime(expires.into()));
        }

        cookie.set_http_only(http_only);
        cookie.set_secure(secure);
        cookie.set_same_site(same_site.map(|s| s.into_ffi()));

        Self(cookie)
    }

    /// The name of the cookie.
    #[getter]
    pub fn name(&self) -> &str {
        self.0.name()
    }

    /// The value of the cookie.
    #[getter]
    pub fn value(&self) -> &str {
        self.0.value()
    }

    /// Returns true if the 'HttpOnly' directive is enabled.
    #[getter]
    pub fn http_only(&self) -> bool {
        self.0.http_only().unwrap_or(false)
    }

    /// Returns true if the 'Secure' directive is enabled.
    #[getter]
    pub fn secure(&self) -> bool {
        self.0.secure().unwrap_or(false)
    }

    /// Returns true if  'SameSite' directive is 'Lax'.
    #[getter]
    pub fn same_site_lax(&self) -> bool {
        self.0.same_site() == Some(cookie::SameSite::Lax)
    }

    /// Returns true if  'SameSite' directive is 'Strict'.
    #[getter]
    pub fn same_site_strict(&self) -> bool {
        self.0.same_site() == Some(cookie::SameSite::Strict)
    }

    /// Returns the path directive of the cookie, if set.
    #[getter]
    pub fn path(&self) -> Option<&str> {
        self.0.path()
    }

    /// Returns the domain directive of the cookie, if set.
    #[getter]
    pub fn domain(&self) -> Option<&str> {
        self.0.domain()
    }

    /// Get the Max-Age information.
    #[getter]
    pub fn max_age(&self) -> Option<std::time::Duration> {
        self.0.max_age().and_then(|d| d.try_into().ok())
    }

    /// The cookie expiration time.
    #[getter]
    pub fn expires(&self) -> Option<SystemTime> {
        match self.0.expires() {
            Some(Expiration::DateTime(offset)) => Some(SystemTime::from(offset)),
            None | Some(Expiration::Session) => None,
        }
    }
}

impl Cookie {
    /// Parse cookies from a `HeaderMap`.
    pub fn extract_headers_cookies(headers: &HeaderMap) -> Vec<Cookie> {
        headers
            .get_all(header::SET_COOKIE)
            .iter()
            .map(Cookie::parse)
            .flat_map(Result::ok)
            .map(RawCookie::into_owned)
            .map(Cookie)
            .collect()
    }

    fn parse<'a>(value: &'a HeaderValue) -> Result<RawCookie<'a>, ParseError> {
        std::str::from_utf8(value.as_bytes())
            .map_err(cookie::ParseError::from)
            .and_then(RawCookie::parse)
    }
}

impl_print_str!(Display, Cookie);

// ===== impl Cookies =====

impl FromPyObject<'_, '_> for Cookies {
    type Error = PyErr;

    fn extract(ob: Borrowed<PyAny>) -> PyResult<Self> {
        if let Ok(cookie) = ob.extract::<PyBackedStr>() {
            return HeaderValue::from_maybe_shared(Bytes::from_owner(cookie))
                .map(|cookie| Cookies(vec![cookie]))
                .map_err(Error::from)
                .map_err(Into::into);
        }

        let dict = ob.cast::<PyDict>()?;
        dict.iter()
            .try_fold(Vec::with_capacity(dict.len()), |mut cookies, (k, v)| {
                let cookie = {
                    let key = k.extract::<PyBackedStr>()?;
                    let value = v.extract::<PyBackedStr>()?;
                    let mut cookie = String::with_capacity(key.len() + 1 + value.len());
                    cookie.push_str(key.as_ref());
                    cookie.push('=');
                    cookie.push_str(value.as_ref());
                    HeaderValue::from_maybe_shared(Bytes::from(cookie)).map_err(Error::from)?
                };

                cookies.push(cookie);
                Ok(cookies)
            })
            .map(Cookies)
    }
}

// ===== impl Jar =====

#[pymethods]
impl Jar {
    /// Create a new [`Jar`] with an empty cookie store.
    #[new]
    pub fn new() -> Self {
        Self(Arc::new(wreq::cookie::Jar::default()))
    }

    /// Get a cookie by name and URL.
    #[pyo3(signature = (name, url))]
    pub fn get(&self, py: Python, name: PyBackedStr, url: PyBackedStr) -> Option<Cookie> {
        py.detach(|| {
            self.0
                .get(&name, AsRef::<str>::as_ref(&url))
                .map(RawCookie::from)
                .map(Cookie)
        })
    }

    /// Get all cookies.
    pub fn get_all(&self, py: Python) -> Vec<Cookie> {
        py.detach(|| self.0.get_all().map(RawCookie::from).map(Cookie).collect())
    }

    /// Add a cookie to this jar.
    #[pyo3(signature = (cookie, url))]
    pub fn add(&self, py: Python, cookie: PyCookie, url: PyBackedStr) {
        py.detach(|| {
            let url = AsRef::<str>::as_ref(&url);
            match cookie {
                PyCookie::Cookie(cookie) => self.0.add(cookie.0, url),
                PyCookie::String(cookie_str) => self.0.add(cookie_str.as_ref(), url),
            }
        })
    }

    /// Remove a cookie from this jar by name and URL.
    #[pyo3(signature = (name, url))]
    pub fn remove(&self, py: Python, name: PyBackedStr, url: PyBackedStr) {
        py.detach(|| {
            self.0.remove(
                AsRef::<str>::as_ref(&name).to_owned(),
                AsRef::<str>::as_ref(&url),
            )
        })
    }

    /// Clear all cookies in this jar.
    pub fn clear(&self, py: Python) {
        py.detach(|| self.0.clear())
    }
}
