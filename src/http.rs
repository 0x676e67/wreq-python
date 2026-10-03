use pyo3::{class::basic::CompareOp, prelude::*, pyclass::PyClassGuardError};

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
#[pyclass(subclass, frozen, str, skip_from_py_object)]
pub struct StatusCode(pub wreq::StatusCode);

impl<'a, 'py> FromPyObject<'a, 'py> for StatusCode {
    type Error = PyClassGuardError<'a, 'py>;

    fn extract(ob: Borrowed<'a, 'py, PyAny>) -> Result<Self, Self::Error> {
        Ok(*ob.extract::<PyClassGuard<'_, Self>>()?)
    }
}

#[pymethods]
impl StatusCode {
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

    /// Rich comparison with integers.
    fn __richcmp__(&self, other: &Bound<PyAny>, op: CompareOp) -> PyResult<bool> {
        // Try to extract an integer from other
        let other_int: u16 = other.extract()?;
        let self_int = self.as_int();

        Ok(match op {
            CompareOp::Lt => self_int < other_int,
            CompareOp::Le => self_int <= other_int,
            CompareOp::Eq => self_int == other_int,
            CompareOp::Ne => self_int != other_int,
            CompareOp::Gt => self_int > other_int,
            CompareOp::Ge => self_int >= other_int,
        })
    }
}

impl_print_str!(Display, StatusCode);
