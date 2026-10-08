"""
HTTP Client Exceptions

Every exception wreq raises for a failed request, response, or WebSocket derives
from `Error`, so `except wreq.Error` catches them all. Misuse, such as reading a
consumed body or awaiting a coroutine twice, raises Python's builtin exceptions.

    Error
    ├── BuilderError
    ├── TlsError
    ├── RequestError
    │   ├── ConnectionError
    │   │   ├── ProxyConnectionError
    │   │   └── ConnectionResetError
    │   └── TimeoutError
    ├── BodyError
    ├── DecodingError
    ├── RedirectError
    ├── StatusError
    └── WebSocketError
        └── UpgradeError

`ConnectionError`, `ConnectionResetError` and `TimeoutError` also derive from the
builtins of the same name, so generic network handlers catch them too.
"""

import builtins

__all__ = [
    "Error",
    "BuilderError",
    "TlsError",
    "RequestError",
    "ConnectionError",
    "ProxyConnectionError",
    "ConnectionResetError",
    "TimeoutError",
    "BodyError",
    "DecodingError",
    "RedirectError",
    "StatusError",
    "WebSocketError",
    "UpgradeError",
]


class Error(Exception):
    r"""
    Base class for all wreq errors.
    """


# ========================================
# Configuration Errors
# ========================================


class BuilderError(Error):
    r"""
    A client, request, or one of their options is invalid.

    Raised for malformed URLs, header names or values, form or JSON bodies,
    proxies, and DNS resolver settings.
    """


class TlsError(Error):
    r"""
    TLS settings or material are invalid, such as an unparsable certificate,
    identity, or certificate store.

    A failed TLS handshake raises `ConnectionError`.
    """


# ========================================
# Transport Errors
# ========================================


class RequestError(Error):
    r"""
    The request failed in transit: while connecting, sending it, or waiting
    on the peer.

    Raised directly for transport failures without a more specific subclass,
    including errors raised by an upload stream.
    """


class ConnectionError(RequestError, builtins.ConnectionError):
    r"""
    The connection could not be established, for example on a DNS failure, a
    refused connection, or a failed TLS handshake.
    """


class ProxyConnectionError(ConnectionError):
    r"""
    The connection through the configured proxy could not be established.
    """


class ConnectionResetError(ConnectionError, builtins.ConnectionResetError):
    r"""
    The peer reset the connection.
    """


class TimeoutError(RequestError, builtins.TimeoutError):
    r"""
    A configured timeout elapsed while connecting, reading the response or its
    body, or receiving a WebSocket message.
    """


# ========================================
# Response Errors
# ========================================


class BodyError(Error):
    r"""
    Streaming a request or response body failed.
    """


class DecodingError(Error):
    r"""
    The response body could not be read in full or decoded as requested,
    such as an unknown charset, corrupt compression, or invalid JSON.
    """


class RedirectError(Error):
    r"""
    The redirect policy stopped the request, for example after too many
    redirects.
    """


class StatusError(Error):
    r"""
    The response has an error status (4xx or 5xx) while status checking is
    enabled.
    """


# ========================================
# WebSocket Errors
# ========================================


class WebSocketError(Error):
    r"""
    A WebSocket operation failed, or the connection is already closed.

    A connection reset raises `ConnectionResetError`, and a receive timeout
    raises `TimeoutError`; catch `Error` to handle every failure.
    """


class UpgradeError(WebSocketError):
    r"""
    The WebSocket handshake failed, for example on an unexpected status or a
    missing upgrade header.
    """
