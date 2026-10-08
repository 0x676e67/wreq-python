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

The class names the main cause of a failure; the `is_*` methods of `Error` report
every detail wreq found, and `url` holds the request URL, which the message omits.
"""

import builtins
from typing import TYPE_CHECKING, Iterable

if TYPE_CHECKING:
    from .wreq import StatusCode

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

    The `is_*` methods mirror the predicates of the Rust `wreq::Error`. Several
    can match one failure: a connect timeout raises `TimeoutError` and matches
    both `is_timeout()` and `is_connect()`.

    Predicates depend on what wreq, its protocol libraries and the operating
    system report, so the ones a given failure matches may change between
    releases; the message text is unspecified as well. Choose handlers by
    exception class, and use predicates to refine them or for diagnostics.
    """

    url: "str | None"
    r"""
    The request URL, if known. The message leaves it out because it may hold
    credentials.
    """

    status: "StatusCode | None"
    r"""
    The response status of a `StatusError`, otherwise None.
    """

    def __init__(
        self,
        message: str = "",
        predicates: Iterable[str] = (),
        url: "str | None" = None,
        status: "StatusCode | None" = None,
    ) -> None:
        # Only the message goes to the base, so the `OSError` subclasses keep
        # `args == (message,)` instead of reading the rest as an errno.
        super().__init__(message)
        self._predicates = frozenset(predicates)
        self.url = url
        self.status = status

    def is_builder(self) -> bool:
        r"""
        Whether building the client, the request, or one of its options failed.
        """
        return "builder" in self._predicates

    def is_request(self) -> bool:
        r"""
        Whether sending the request or receiving its response failed.
        """
        return "request" in self._predicates

    def is_connect(self) -> bool:
        r"""
        Whether connecting to the destination failed, including the TLS handshake.
        """
        return "connect" in self._predicates

    def is_proxy_connect(self) -> bool:
        r"""
        Whether connecting through the proxy failed.
        """
        return "proxy_connect" in self._predicates

    def is_connection_reset(self) -> bool:
        r"""
        Whether the peer reset the connection.
        """
        return "connection_reset" in self._predicates

    def is_dns(self) -> bool:
        r"""
        Whether resolving the host name failed.
        """
        return "dns" in self._predicates

    def is_timeout(self) -> bool:
        r"""
        Whether a timeout elapsed.
        """
        return "timeout" in self._predicates

    def is_body(self) -> bool:
        r"""
        Whether streaming a request or response body failed.
        """
        return "body" in self._predicates

    def is_tls(self) -> bool:
        r"""
        Whether TLS settings or material are invalid; a failed handshake is
        `is_connect()`.
        """
        return "tls" in self._predicates

    def is_decode(self) -> bool:
        r"""
        Whether reading or decoding the response failed.
        """
        return "decode" in self._predicates

    def is_redirect(self) -> bool:
        r"""
        Whether the redirect policy stopped the request.
        """
        return "redirect" in self._predicates

    def is_status(self) -> bool:
        r"""
        Whether the response has an error status.
        """
        return "status" in self._predicates

    def is_upgrade(self) -> bool:
        r"""
        Whether upgrading the connection failed.
        """
        return "upgrade" in self._predicates

    def is_websocket(self) -> bool:
        r"""
        Whether a WebSocket operation failed.
        """
        return "websocket" in self._predicates


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
