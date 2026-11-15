use pyo3::{PyResult, pybacked::PyBackedStr, pyclass, pymethods};

use crate::{buffer::PyBuffer, error::Error};

#[derive(Clone)]
#[pyclass(from_py_object)]
pub struct CertStore(pub wreq::tls::trust::CertStore);

#[pymethods]
impl CertStore {
    /// Creates a new `CertStore`.
    #[new]
    #[pyo3(signature = (der_certs=None, pem_certs=None, default_paths=false))]
    pub fn new(
        der_certs: Option<Vec<PyBuffer>>,
        pem_certs: Option<Vec<PyBackedStr>>,
        default_paths: bool,
    ) -> PyResult<CertStore> {
        let mut store = wreq::tls::trust::CertStore::builder();

        // Add DER certificates if provided
        if let Some(der_certs) = der_certs {
            store = store.add_der_certs(&der_certs);
        }

        // Add PEM certificates if provided
        if let Some(pem_certs) = pem_certs {
            store = store.add_pem_certs(&pem_certs);
        }

        // Set default paths if specified
        if default_paths {
            store = store.set_default_paths();
        }

        store
            .build()
            .map(CertStore)
            .map_err(Error::Library)
            .map_err(Into::into)
    }

    /// Creates a new `CertStore` from a collection of DER-encoded certificates.
    #[staticmethod]
    #[pyo3(signature = (certs))]
    pub fn from_der_certs(certs: Vec<PyBuffer>) -> PyResult<CertStore> {
        wreq::tls::trust::CertStore::from_der_certs(&certs)
            .map(CertStore)
            .map_err(Error::Library)
            .map_err(Into::into)
    }

    /// Creates a new `CertStore` from a collection of PEM-encoded certificates.
    #[staticmethod]
    #[pyo3(signature = (certs))]
    pub fn from_pem_certs(certs: Vec<PyBackedStr>) -> PyResult<CertStore> {
        wreq::tls::trust::CertStore::from_pem_certs(&certs)
            .map(CertStore)
            .map_err(Error::Library)
            .map_err(Into::into)
    }

    /// Creates a new `CertStore` from a PEM-encoded certificate stack.
    #[staticmethod]
    #[pyo3(signature = (certs))]
    pub fn from_pem_stack(certs: PyBuffer) -> PyResult<CertStore> {
        wreq::tls::trust::CertStore::from_pem_stack(certs.as_ref())
            .map(CertStore)
            .map_err(Error::Library)
            .map_err(Into::into)
    }
}
