"""
TLS Utilities and Types

This module provides types and utilities for configuring TLS (Transport Layer Security) in HTTP clients.
"""

from enum import Enum, auto
from pathlib import Path
from typing import Sequence, NotRequired, TypedDict, Unpack, final

__all__ = [
    "TlsVersion",
    "Identity",
    "CertStore",
    "KeyLog",
    "AlpnProtocol",
    "AlpsProtocol",
    "CertificateCompressionAlgorithm",
    "ExtensionType",
    "TlsOptions",
    "TlsInfo",
    "chromium_root_store",
    "Params",
    "KeyShare",
]


@final
class TlsVersion(Enum):
    r"""
    The TLS version.
    """

    TLS_1_0 = auto()
    TLS_1_1 = auto()
    TLS_1_2 = auto()
    TLS_1_3 = auto()


@final
class AlpnProtocol(Enum):
    """
    A TLS ALPN protocol.
    """

    HTTP1 = auto()
    HTTP2 = auto()
    HTTP3 = auto()


@final
class AlpsProtocol(Enum):
    """
    Application-layer protocol settings for HTTP/1.1 and HTTP/2.
    """

    HTTP1 = auto()
    HTTP2 = auto()
    HTTP3 = auto()


@final
class KeyShare(Enum):
    """
    Key exchange groups (elliptic curves) for TLS 1.3.
    """

    P256 = auto()
    P384 = auto()
    P521 = auto()
    X25519 = auto()
    X25519_MLKEM768 = auto()
    X25519_KYBER768_DRAFT00 = auto()
    P256_KYBER768_DRAFT00 = auto()
    MLKEM1024 = auto()
    FFDHE2048 = auto()
    FFDHE3072 = auto()


@final
class CertificateCompressionAlgorithm(Enum):
    """
    IANA assigned identifier of compression algorithm.
    See https://www.rfc-editor.org/rfc/rfc8879.html#name-compression-algorithms
    """

    ZLIB = auto()
    BROTLI = auto()
    ZSTD = auto()


@final
class ExtensionType(Enum):
    """
    A TLS extension type.
    """

    SERVER_NAME = auto()
    STATUS_REQUEST = auto()
    EC_POINT_FORMATS = auto()
    SIGNATURE_ALGORITHMS = auto()
    SRTP = auto()
    APPLICATION_LAYER_PROTOCOL_NEGOTIATION = auto()
    PADDING = auto()
    EXTENDED_MASTER_SECRET = auto()
    QUIC_TRANSPORT_PARAMETERS_LEGACY = auto()
    QUIC_TRANSPORT_PARAMETERS_STANDARD = auto()
    CERT_COMPRESSION = auto()
    SESSION_TICKET = auto()
    SUPPORTED_GROUPS = auto()
    PRE_SHARED_KEY = auto()
    EARLY_DATA = auto()
    SUPPORTED_VERSIONS = auto()
    COOKIE = auto()
    PSK_KEY_EXCHANGE_MODES = auto()
    CERTIFICATE_AUTHORITIES = auto()
    SIGNATURE_ALGORITHMS_CERT = auto()
    KEY_SHARE = auto()
    RENEGOTIATE = auto()
    DELEGATED_CREDENTIAL = auto()
    APPLICATION_SETTINGS_OLD = auto()
    APPLICATION_SETTINGS = auto()
    ENCRYPTED_CLIENT_HELLO = auto()
    CERTIFICATE_TIMESTAMP = auto()
    NEXT_PROTO_NEG = auto()
    CHANNEL_ID = auto()
    RECORD_SIZE_LIMIT = auto()


@final
class Identity:
    """
    Represents a private key and X509 cert as a client certificate.
    """

    @staticmethod
    def from_pkcs12_der(buf: bytes, pass_: str) -> "Identity":
        """
        Parses a DER-formatted PKCS #12 archive, using the specified password to decrypt the key.

        The archive should contain a leaf certificate and its private key, as well any intermediate
        certificates that allow clients to build a chain to a trusted root.
        The chain certificates should be in order from the leaf certificate towards the root.

        PKCS #12 archives typically have the file extension `.p12` or `.pfx`, and can be created
        with the OpenSSL `pkcs12` tool:

            openssl pkcs12 -export -out identity.pfx -inkey key.pem -in cert.pem -certfile chain_certs.pem
        """
        ...

    @staticmethod
    def from_pkcs8_pem(buf: bytes, key: bytes) -> "Identity":
        """
        Parses a chain of PEM encoded X509 certificates, with the leaf certificate first.
        `key` is a PEM encoded PKCS #8 formatted private key for the leaf certificate.

        The certificate chain should contain any intermediate certificates that should be sent to
        clients to allow them to build a chain to a trusted root.

        A certificate chain here means a series of PEM encoded certificates concatenated together.
        """
        ...


@final
class CertStore:
    """
    Represents a certificate store for verifying TLS connections.
    """

    def __init__(
        self,
        der_certs: Sequence[bytes] | None = None,
        pem_certs: Sequence[str] | None = None,
        default_paths: bool | None = None,
    ) -> None:
        """
        Creates a new CertStore.

        Args:
            der_certs: Optional list of DER-encoded certificates (as bytes).
            pem_certs: Optional list of PEM-encoded certificates (as str).
            default_paths: If True, use system default certificate paths.
        """
        ...

    @staticmethod
    def from_der_certs(certs: Sequence[bytes]) -> "CertStore":
        """
        Creates a CertStore from a collection of DER-encoded certificates.

        Args:
            certs: List of DER-encoded certificates (as bytes).
        """
        ...

    @staticmethod
    def from_pem_certs(certs: Sequence[str]) -> "CertStore":
        """
        Creates a CertStore from a collection of PEM-encoded certificates.

        Args:
            certs: List of PEM-encoded certificates (as str).
        """
        ...

    @staticmethod
    def from_pem_stack(certs: bytes) -> "CertStore":
        """
        Creates a CertStore from a PEM-encoded certificate stack.

        Args:
            certs: PEM-encoded certificate stack (as bytes).
        """
        ...


@final
class KeyLog:
    """
    Specifies the intent for a (TLS) keylogger to be used in a client or server configuration.

    This type allows you to control how TLS session keys are logged for debugging or analysis.
    You can either use the default environment variable (SSLKEYLOGFILE) or specify a file path
    directly. This is useful for tools like Wireshark that can decrypt TLS traffic if provided
    with the correct session keys.

    Static Methods:
        environment() -> KeyLog
            Use the SSLKEYLOGFILE environment variable for key logging.
        file(path: Path) -> KeyLog
            Log keys to the specified file path.
    """

    @staticmethod
    def environment() -> "KeyLog":
        """
        Use the SSLKEYLOGFILE environment variable for key logging.
        """
        ...

    @staticmethod
    def file(path: Path | str) -> "KeyLog":
        """
        Log keys to the specified file path.

        Args:
            path: The file path to log TLS keys to.
        """
        ...


class Params(TypedDict):
    """
    All parameters for TLS connections.
    """

    alpn_protocols: NotRequired[Sequence[AlpnProtocol]]
    """
    Application-Layer Protocol Negotiation (RFC 7301).

    Specifies which application protocols (e.g., HTTP/2, HTTP/1.1) may be negotiated
    over a single TLS connection.
    """

    alps_protocols: NotRequired[Sequence[AlpsProtocol]]
    """
    Application-Layer Protocol Settings (ALPS).

    Enables exchanging application-layer settings during the handshake
    for protocols negotiated via ALPN.
    """

    alps_use_new_codepoint: NotRequired[bool]
    """
    Whether to use an alternative ALPS codepoint for compatibility.

    Useful when larger ALPS payloads are required.
    """

    session_ticket: NotRequired[bool]
    """
    Enables TLS Session Tickets (RFC 5077).

    Allows session resumption without requiring server-side state.
    """

    min_tls_version: NotRequired[TlsVersion]
    """
    Minimum TLS version allowed for the connection.
    """

    max_tls_version: NotRequired[TlsVersion]
    """
    Maximum TLS version allowed for the connection.
    """

    pre_shared_key: NotRequired[bool]
    """
    Enables Pre-Shared Key (PSK) cipher suites (RFC 4279).

    Authentication relies on out-of-band pre-shared keys instead of certificates.
    """

    enable_ech_grease: NotRequired[bool]
    """
    Controls whether to send a GREASE Encrypted ClientHello (ECH) extension
    when no supported ECH configuration is available.

    GREASE prevents protocol ossification by sending unknown extensions.
    """

    permute_extensions: NotRequired[bool]
    """
    Controls whether ClientHello extensions should be permuted.
    """

    grease_enabled: NotRequired[bool]
    """
    Controls whether GREASE extensions (RFC 8701) are enabled in general.
    """

    enable_ocsp_stapling: NotRequired[bool]
    """
    Enables OCSP stapling for the connection.
    """

    enable_signed_cert_timestamps: NotRequired[bool]
    """
    Enables Signed Certificate Timestamps (SCT).
    """

    record_size_limit: NotRequired[int]
    """
    Sets the maximum TLS record size.
    """

    psk_skip_session_ticket: NotRequired[bool]
    """
    Whether to skip session tickets when using PSK.
    """

    key_shares: NotRequired[Sequence[KeyShare]]
    """
    Whether to set specific key shares for TLS 1.3 handshakes.
    """

    psk_dhe_ke: NotRequired[bool]
    """
    Enables PSK with (EC)DHE key establishment (`psk_dhe_ke`).
    """

    renegotiation: NotRequired[bool]
    """
    Enables TLS renegotiation by sending the `renegotiation_info` extension.
    """

    delegated_credentials: NotRequired[str]
    """
    Delegated Credentials (RFC 9345).

    Allows TLS 1.3 endpoints to use temporary delegated credentials
    for authentication with reduced long-term key exposure.
    """

    curves_list: NotRequired[str]
    """
    List of supported elliptic curves.
    """

    sigalgs_list: NotRequired[str]
    """
    List of supported signature algorithms.
    """

    cipher_list: NotRequired[str]
    """
    Cipher suite configuration string.

    Uses BoringSSL's mini-language to select, enable, and prioritize ciphers.
    """

    preserve_tls13_cipher_list: NotRequired[bool]
    """
    Sets whether to preserve the TLS 1.3 cipher list as configured by cipher_list.

    By default, BoringSSL does not preserve the TLS 1.3 cipher list. When this option is disabled
    (the default), BoringSSL uses its internal default TLS 1.3 cipher suites in its default order,
    regardless of what is set via cipher_list.

    When enabled, this option ensures that the TLS 1.3 cipher suites explicitly set via
    cipher_list are retained in their original order, without being reordered or
    modified by BoringSSL's internal logic. This is useful for maintaining specific cipher suite
    priorities for TLS 1.3. Note that if cipher_list does not include any TLS 1.3
    cipher suites, BoringSSL will still fall back to its default TLS 1.3 cipher suites and order.
    """

    certificate_compression_algorithms: NotRequired[
        Sequence[CertificateCompressionAlgorithm]
    ]
    """
    Supported certificate compression algorithms (RFC 8879).
    """

    extension_permutation: NotRequired[Sequence[ExtensionType]]
    """
    Supported TLS extensions, used for extension ordering/permutation.
    """

    aes_hw_override: NotRequired[bool]
    """
    Overrides AES hardware acceleration.
    """

    random_aes_hw_override: NotRequired[bool]
    """
    Overrides the random AES hardware acceleration.
    """


@final
class TlsOptions:
    """
    TLS connection configuration options.

    This struct provides fine-grained control over the behavior of TLS
    connections, including:
     - **Protocol negotiation** (ALPN, ALPS, TLS versions)
     - **Session management** (tickets, PSK, key shares)
     - **Security & privacy** (OCSP, GREASE, ECH, delegated credentials)
     - **Performance tuning** (record size, cipher preferences, hardware overrides)

    All fields are optional or have defaults. See each field for details.
    """

    def __init__(self, **kwargs: Unpack[Params]) -> None:
        """
        Creates a new TlsOptions.
        """
        ...


@final
class TlsInfo:
    """
    Information about the established TLS connection.
    """

    def peer_certificate(self) -> bytes | None:
        """
        Get the DER encoded leaf certificate of the peer.
        """
        ...


def chromium_root_store() -> CertStore:
    """
    Chrome's root store from chromium-root-certs, for a client's `tls_verify`.

    Chrome 150+ requests specific trust anchors and a server may return a chain anchored to one
    of them, so the client needs Chrome's roots to verify it. Backed only when wreq was built with
    the `chromium-pki` feature; without it this function exists but raises RuntimeError.
    """
    ...
