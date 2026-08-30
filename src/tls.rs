mod identity;
mod keylog;
mod store;

use pyo3::prelude::*;
use wreq::tls::compress::CertificateCompressor;
use wreq_util::emulate::compress;

pub use self::{identity::Identity, keylog::KeyLog, store::CertStore};
use crate::buffer::PyBuffer;

define_enum!(
    /// The TLS version.
    const,
    TlsVersion,
    wreq::tls::TlsVersion,
    TLS_1_0,
    TLS_1_1,
    TLS_1_2,
    TLS_1_3,
);

#[derive(Clone, FromPyObject)]
pub enum TlsVerify {
    Verification(bool),
    CertificatePath(std::path::PathBuf),
    CertificateStore(CertStore),
}

define_enum!(
    /// A TLS ALPN protocol.
    const,
    AlpnProtocol,
    wreq::tls::AlpnProtocol,
    HTTP1,
    HTTP2,
    HTTP3,
);

define_enum!(
    /// Application-layer protocol settings for HTTP/1.1 and HTTP/2.
    const,
    AlpsProtocol,
    wreq::tls::AlpsProtocol,
    HTTP1,
    HTTP2,
    HTTP3,
);

define_enum!(
    /// Key exchange groups (elliptic curves) for TLS 1.3.
    const,
    KeyShare,
    wreq::tls::KeyShare,
    P256,
    P384,
    P521,
    X25519,
    X25519_MLKEM768,
    X25519_KYBER768_DRAFT00,
    P256_KYBER768_DRAFT00,
    MLKEM1024,
    FFDHE2048,
    FFDHE3072,
);

define_enum!(
    // IANA assigned identifier of compression algorithm. See https://www.rfc-editor.org/rfc/rfc8879.html#name-compression-algorithms
    const,
    CertificateCompressionAlgorithm,
    wreq::tls::compress::CertificateCompressionAlgorithm,
    ZLIB,
    BROTLI,
    ZSTD,
);

define_enum!(
    /// A TLS extension type.
    const,
    ExtensionType,
    wreq::tls::ExtensionType,
    SERVER_NAME,
    STATUS_REQUEST,
    EC_POINT_FORMATS,
    SIGNATURE_ALGORITHMS,
    SRTP,
    APPLICATION_LAYER_PROTOCOL_NEGOTIATION,
    PADDING,
    EXTENDED_MASTER_SECRET,
    QUIC_TRANSPORT_PARAMETERS_LEGACY,
    QUIC_TRANSPORT_PARAMETERS_STANDARD,
    CERT_COMPRESSION,
    SESSION_TICKET,
    SUPPORTED_GROUPS,
    PRE_SHARED_KEY,
    EARLY_DATA,
    SUPPORTED_VERSIONS,
    COOKIE,
    PSK_KEY_EXCHANGE_MODES,
    CERTIFICATE_AUTHORITIES,
    SIGNATURE_ALGORITHMS_CERT,
    KEY_SHARE,
    RENEGOTIATE,
    DELEGATED_CREDENTIAL,
    APPLICATION_SETTINGS_OLD,
    APPLICATION_SETTINGS,
    ENCRYPTED_CLIENT_HELLO,
    CERTIFICATE_TIMESTAMP,
    NEXT_PROTO_NEG,
    CHANNEL_ID,
    RECORD_SIZE_LIMIT,
);

/// A builder for [`TlsOptions`].
#[derive(Default)]
struct Builder {
    /// Application-Layer Protocol Negotiation ([RFC 7301](https://datatracker.ietf.org/doc/html/rfc7301)).
    ///
    /// Specifies which application protocols (e.g., HTTP/2, HTTP/1.1) may be negotiated
    /// over a single TLS connection.
    alpn_protocols: Option<Vec<AlpnProtocol>>,

    /// Application-Layer Protocol Settings (ALPS).
    ///
    /// Enables exchanging application-layer settings during the handshake
    /// for protocols negotiated via ALPN.
    alps_protocols: Option<Vec<AlpsProtocol>>,

    /// Whether to use an alternative ALPS codepoint for compatibility.
    ///
    /// Useful when larger ALPS payloads are required.
    alps_use_new_codepoint: Option<bool>,

    /// Enables TLS Session Tickets ([RFC 5077](https://tools.ietf.org/html/rfc5077)).
    ///
    /// Allows session resumption without requiring server-side state.
    session_ticket: Option<bool>,

    /// Minimum TLS version allowed for the connection.
    min_tls_version: Option<TlsVersion>,

    /// Maximum TLS version allowed for the connection.
    max_tls_version: Option<TlsVersion>,

    /// Enables Pre-Shared Key (PSK) cipher suites ([RFC 4279](https://datatracker.ietf.org/doc/html/rfc4279)).
    ///
    /// Authentication relies on out-of-band pre-shared keys instead of certificates.
    pre_shared_key: Option<bool>,

    /// Controls whether to send a GREASE Encrypted ClientHello (ECH) extension
    /// when no supported ECH configuration is available.
    ///
    /// GREASE prevents protocol ossification by sending unknown extensions.
    enable_ech_grease: Option<bool>,

    /// Controls whether ClientHello extensions should be permuted.
    permute_extensions: Option<bool>,

    /// Controls whether GREASE extensions ([RFC 8701](https://datatracker.ietf.org/doc/html/rfc8701))
    /// are enabled in general.
    grease_enabled: Option<bool>,

    /// Enables OCSP stapling for the connection.
    enable_ocsp_stapling: Option<bool>,

    /// Enables Signed Certificate Timestamps (SCT).
    enable_signed_cert_timestamps: Option<bool>,

    /// Sets the maximum TLS record size.
    record_size_limit: Option<u16>,

    /// Whether to skip session tickets when using PSK.
    psk_skip_session_ticket: Option<bool>,

    /// Whether to set specific key shares for TLS 1.3 handshakes.
    key_shares: Option<Vec<KeyShare>>,

    /// Enables PSK with (EC)DHE key establishment (`psk_dhe_ke`).
    psk_dhe_ke: Option<bool>,

    /// Enables TLS renegotiation by sending the `renegotiation_info` extension.
    renegotiation: Option<bool>,

    /// Delegated Credentials ([RFC 9345](https://datatracker.ietf.org/doc/html/rfc9345)).
    ///
    /// Allows TLS 1.3 endpoints to use temporary delegated credentials
    /// for authentication with reduced long-term key exposure.
    delegated_credentials: Option<String>,

    /// List of supported elliptic curves.
    curves_list: Option<String>,

    /// List of supported signature algorithms.
    sigalgs_list: Option<String>,

    /// Cipher suite configuration string.
    ///
    /// Uses BoringSSL's mini-language to select, enable, and prioritize ciphers.
    cipher_list: Option<String>,

    /// Sets whether to preserve the TLS 1.3 cipher list as configured by [`Self::cipher_list`].
    preserve_tls13_cipher_list: Option<bool>,

    /// Supported certificate compression algorithms ([RFC 8879](https://datatracker.ietf.org/doc/html/rfc8879)).
    certificate_compression_algorithms: Option<Vec<CertificateCompressionAlgorithm>>,

    /// Supported TLS extensions, used for extension ordering/permutation.
    extension_permutation: Option<Vec<ExtensionType>>,

    /// Overrides AES hardware acceleration.
    aes_hw_override: Option<bool>,

    /// Overrides the random AES hardware acceleration.
    random_aes_hw_override: Option<bool>,
}

impl FromPyObject<'_, '_> for Builder {
    type Error = PyErr;

    fn extract(ob: Borrowed<PyAny>) -> PyResult<Self> {
        let mut params = Self::default();
        extract_option!(ob, params, alpn_protocols);
        extract_option!(ob, params, alps_protocols);
        extract_option!(ob, params, alps_use_new_codepoint);
        extract_option!(ob, params, session_ticket);
        extract_option!(ob, params, min_tls_version);
        extract_option!(ob, params, max_tls_version);
        extract_option!(ob, params, pre_shared_key);
        extract_option!(ob, params, enable_ech_grease);
        extract_option!(ob, params, permute_extensions);
        extract_option!(ob, params, grease_enabled);
        extract_option!(ob, params, enable_ocsp_stapling);
        extract_option!(ob, params, enable_signed_cert_timestamps);
        extract_option!(ob, params, record_size_limit);
        extract_option!(ob, params, psk_skip_session_ticket);
        extract_option!(ob, params, key_shares);
        extract_option!(ob, params, psk_dhe_ke);
        extract_option!(ob, params, renegotiation);
        extract_option!(ob, params, delegated_credentials);
        extract_option!(ob, params, curves_list);
        extract_option!(ob, params, cipher_list);
        extract_option!(ob, params, sigalgs_list);
        extract_option!(ob, params, certificate_compression_algorithms);
        extract_option!(ob, params, extension_permutation);
        extract_option!(ob, params, aes_hw_override);
        extract_option!(ob, params, preserve_tls13_cipher_list);
        extract_option!(ob, params, random_aes_hw_override);
        Ok(params)
    }
}

/// TLS connection configuration options.
///
/// This struct provides fine-grained control over the behavior of TLS
/// connections, including:
/// - **Protocol negotiation** (ALPN, ALPS, TLS versions)
/// - **Session management** (tickets, PSK, key shares)
/// - **Security & privacy** (OCSP, GREASE, ECH, delegated credentials)
/// - **Performance tuning** (record size, cipher preferences, hardware overrides)
///
/// All fields are optional or have defaults. See each field for details.
#[derive(Clone)]
#[pyclass(frozen, from_py_object)]
pub struct TlsOptions(pub wreq::tls::TlsOptions);

#[pymethods]
impl TlsOptions {
    #[new]
    #[pyo3(signature = (**kwds))]
    fn new(py: Python, kwds: Option<Builder>) -> Self {
        py.detach(|| {
            let mut builder = wreq::tls::TlsOptions::builder();

            if let Some(mut params) = kwds {
                apply_option!(
                    set_if_some_map,
                    builder,
                    params.alpn_protocols,
                    alpn_protocols,
                    |v: Vec<_>| v.into_iter().map(AlpnProtocol::into_ffi)
                );
                apply_option!(
                    set_if_some_map,
                    builder,
                    params.alps_protocols,
                    alps_protocols,
                    |v: Vec<_>| v.into_iter().map(AlpsProtocol::into_ffi)
                );
                apply_option!(
                    set_if_some,
                    builder,
                    params.alps_use_new_codepoint,
                    alps_use_new_codepoint
                );
                apply_option!(set_if_some, builder, params.session_ticket, session_ticket);
                apply_option!(
                    set_if_some_map,
                    builder,
                    params.min_tls_version,
                    min_tls_version,
                    TlsVersion::into_ffi
                );
                apply_option!(
                    set_if_some_map,
                    builder,
                    params.max_tls_version,
                    max_tls_version,
                    TlsVersion::into_ffi
                );
                apply_option!(set_if_some, builder, params.pre_shared_key, pre_shared_key);
                apply_option!(
                    set_if_some,
                    builder,
                    params.enable_ech_grease,
                    enable_ech_grease
                );
                apply_option!(
                    set_if_some,
                    builder,
                    params.permute_extensions,
                    permute_extensions
                );
                apply_option!(set_if_some, builder, params.grease_enabled, grease_enabled);
                apply_option!(
                    set_if_some,
                    builder,
                    params.enable_ocsp_stapling,
                    enable_ocsp_stapling
                );
                apply_option!(
                    set_if_some,
                    builder,
                    params.enable_signed_cert_timestamps,
                    enable_signed_cert_timestamps
                );
                apply_option!(
                    set_if_some,
                    builder,
                    params.record_size_limit,
                    record_size_limit
                );
                apply_option!(
                    set_if_some,
                    builder,
                    params.psk_skip_session_ticket,
                    psk_skip_session_ticket
                );
                apply_option!(
                    set_if_some_map,
                    builder,
                    params.key_shares,
                    key_shares,
                    |v: Vec<_>| v.into_iter().map(KeyShare::into_ffi).collect::<Vec<_>>()
                );
                apply_option!(set_if_some, builder, params.psk_dhe_ke, psk_dhe_ke);
                apply_option!(set_if_some, builder, params.renegotiation, renegotiation);
                apply_option!(
                    set_if_some,
                    builder,
                    params.delegated_credentials,
                    delegated_credentials
                );
                apply_option!(set_if_some, builder, params.curves_list, curves_list);
                apply_option!(set_if_some, builder, params.cipher_list, cipher_list);
                apply_option!(set_if_some, builder, params.sigalgs_list, sigalgs_list);
                apply_option!(
                    set_if_some_map,
                    builder,
                    params.certificate_compression_algorithms,
                    certificate_compressors,
                    |v: Vec<_>| v
                        .into_iter()
                        .map(|algs| {
                            match algs {
                                CertificateCompressionAlgorithm::ZLIB => {
                                    &compress::ZlibCompressor as _
                                }
                                CertificateCompressionAlgorithm::BROTLI => {
                                    &compress::BrotliCompressor as _
                                }
                                CertificateCompressionAlgorithm::ZSTD => {
                                    &compress::ZstdCompressor as _
                                }
                            }
                        })
                        .collect::<Vec<&'static dyn CertificateCompressor>>()
                );
                apply_option!(
                    set_if_some_map,
                    builder,
                    params.extension_permutation,
                    extension_permutation,
                    |v: Vec<_>| v
                        .into_iter()
                        .map(ExtensionType::into_ffi)
                        .collect::<Vec<_>>()
                );
                apply_option!(
                    set_if_some,
                    builder,
                    params.preserve_tls13_cipher_list,
                    preserve_tls13_cipher_list
                );
                apply_option!(
                    set_if_some,
                    builder,
                    params.aes_hw_override,
                    aes_hw_override
                );
                apply_option!(
                    set_if_some,
                    builder,
                    params.random_aes_hw_override,
                    random_aes_hw_override
                );
            }

            Self(builder.build())
        })
    }
}

/// Information about the TLS connection.
#[pyclass(frozen)]
pub struct TlsInfo(pub wreq::tls::TlsInfo);

#[pymethods]
impl TlsInfo {
    /// Get the DER encoded leaf certificate of the peer.
    #[inline]
    pub fn peer_certificate(&self) -> Option<PyBuffer> {
        self.0
            .peer_certificate()
            .map(ToOwned::to_owned)
            .map(PyBuffer::from)
    }
}
