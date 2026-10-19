use pyo3::{
    class::basic::CompareOp,
    exceptions::PyValueError,
    intern,
    prelude::*,
    pyclass::PyClassGuardError,
    types::{PyBool, PyInt, PyType},
};

define_enum!(
    /// An HTTP version.
    const,
    Version,
    wreq::Version,
    HTTP_09,
    HTTP_10,
    HTTP_11,
    HTTP_2,
    HTTP_3,
);

define_enum!(
    /// An HTTP method.
    Method,
    wreq::Method,
    GET,
    HEAD,
    POST,
    PUT,
    DELETE,
    OPTIONS,
    TRACE,
    PATCH,
);

/// HTTP status code.
#[derive(Clone, Copy)]
#[pyclass(subclass, frozen, str, skip_from_py_object, module = "wreq")]
pub struct StatusCode(pub wreq::StatusCode);

impl<'a, 'py> FromPyObject<'a, 'py> for StatusCode {
    type Error = PyClassGuardError<'a, 'py>;

    fn extract(ob: Borrowed<'a, 'py, PyAny>) -> Result<Self, Self::Error> {
        Ok(*ob.extract::<PyClassGuard<'_, Self>>()?)
    }
}

#[pymethods]
impl StatusCode {
    /// Create a status code from an integer in 100-999.
    #[new]
    fn new(code: &Bound<'_, PyInt>) -> PyResult<Self> {
        code.extract::<u16>()
            .ok()
            .and_then(|code| wreq::StatusCode::from_u16(code).ok())
            .map(Self)
            .ok_or_else(|| PyValueError::new_err(format!("invalid status code: {code}")))
    }

    fn __reduce__<'py>(
        slf: &Bound<'py, Self>,
    ) -> (Bound<'py, PyType>, (u16,), Option<Bound<'py, PyAny>>) {
        // Instances of a Python subclass keep their attributes in `__dict__`.
        let state = slf.getattr(intern!(slf.py(), "__dict__")).ok();
        (slf.get_type(), (slf.get().as_int(),), state)
    }

    /// Return the status code as an integer.
    pub const fn as_int(&self) -> u16 {
        self.0.as_u16()
    }

    /// Check if status is within 100-199.
    pub fn is_informational(&self) -> bool {
        self.0.is_informational()
    }

    /// Check if status is within 200-299.
    pub fn is_success(&self) -> bool {
        self.0.is_success()
    }

    /// Check if status is within 300-399.
    pub fn is_redirection(&self) -> bool {
        self.0.is_redirection()
    }

    /// Check if status is within 400-499.
    pub fn is_client_error(&self) -> bool {
        self.0.is_client_error()
    }

    /// Check if status is within 500-599.
    pub fn is_server_error(&self) -> bool {
        self.0.is_server_error()
    }

    /// Compare with another status code or an integer.
    fn __richcmp__(&self, other: &Bound<'_, PyAny>, op: CompareOp) -> Py<PyAny> {
        let py = other.py();
        let other = match other.extract::<StatusCode>() {
            Ok(code) => code.as_int(),
            Err(_) => match other.extract::<u16>() {
                Ok(code) => code,
                Err(_) => return py.NotImplemented(),
            },
        };
        PyBool::new(py, op.matches(self.as_int().cmp(&other)))
            .to_owned()
            .into_any()
            .unbind()
    }

    // Equal to the integer of the same value, so it hashes like one.
    fn __hash__(&self) -> u64 {
        u64::from(self.as_int())
    }
}

impl_print_str!(Display, StatusCode);
