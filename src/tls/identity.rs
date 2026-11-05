use pyo3::{PyResult, pybacked::PyBackedStr, pyclass, pymethods};

use crate::{buffer::PyBuffer, error::Error};

/// Represents a private key and X509 cert as a client certificate.
#[derive(Clone)]
#[pyclass(from_py_object)]
pub struct Identity(pub wreq::tls::trust::Identity);

#[pymethods]
impl Identity {
    /// Parses a DER-formatted PKCS #12 archive, using the specified password to decrypt the key.
    ///
    /// The archive should contain a leaf certificate and its private key, as well any intermediate
    /// certificates that allow clients to build a chain to a trusted root.
    /// The chain certificates should be in order from the leaf certificate towards the root.
    ///
    /// PKCS #12 archives typically have the file extension `.p12` or `.pfx`, and can be created
    /// with the OpenSSL `pkcs12` tool:
    ///
    /// ```bash
    /// openssl pkcs12 -export -out identity.pfx -inkey key.pem -in cert.pem -certfile chain_certs.pem
    /// ```
    #[staticmethod]
    #[pyo3(signature = (buf, pass))]
    pub fn from_pkcs12_der(buf: PyBuffer, pass: PyBackedStr) -> PyResult<Identity> {
        wreq::tls::trust::Identity::from_pkcs12_der(buf.as_ref(), pass.as_ref())
            .map(Identity)
            .map_err(Error::Library)
            .map_err(Into::into)
    }

    /// Parses a chain of PEM encoded X509 certificates, with the leaf certificate first.
    /// `key` is a PEM encoded PKCS #8 formatted private key for the leaf certificate.
    ///
    /// The certificate chain should contain any intermediate certificates that should be sent to
    /// clients to allow them to build a chain to a trusted root.
    ///
    /// A certificate chain here means a series of PEM encoded certificates concatenated together.
    #[staticmethod]
    #[pyo3(signature = (buf, key))]
    pub fn from_pkcs8_pem(buf: PyBuffer, key: PyBuffer) -> PyResult<Identity> {
        wreq::tls::trust::Identity::from_pkcs8_pem(buf.as_ref(), key.as_ref())
            .map(Identity)
            .map_err(Error::Library)
            .map_err(Into::into)
    }
}
