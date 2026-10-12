from typing import Mapping, NotRequired, TypedDict, Unpack, final

from wreq.header import HeaderMap

__all__ = ["Proxy"]


class ProxyConfig(TypedDict):
    username: NotRequired[str]
    r"""Username for proxy authentication."""

    password: NotRequired[str]
    r"""Password for proxy authentication."""

    custom_http_auth: NotRequired[str]
    r"""Custom HTTP proxy authentication header value."""

    custom_http_headers: NotRequired[Mapping[str, str] | HeaderMap]
    r"""Custom HTTP proxy headers."""

    exclusion: NotRequired[str]
    r"""List of domains to exclude from proxying."""


@final
class Proxy:
    r"""
    A proxy server for a request.
    Supports HTTP, HTTPS, SOCKS4, SOCKS4a, SOCKS5, and SOCKS5h protocols.

    Unknown keyword arguments passed to its factory methods raise `TypeError`.
    """

    @staticmethod
    def http(url: str, **kwargs: Unpack[ProxyConfig]) -> "Proxy":
        r"""
        Creates a new HTTP proxy.

        This method sets up a proxy server for HTTP requests.

        # Examples

        ```python
        import wreq

        proxy = wreq.Proxy.http("http://proxy.example.com")
        ```
        """
        ...

    @staticmethod
    def https(url: str, **kwargs: Unpack[ProxyConfig]) -> "Proxy":
        r"""
        Creates a new HTTPS proxy.

        This method sets up a proxy server for HTTPS requests.

        # Examples

        ```python
        import wreq

        proxy = wreq.Proxy.https("https://proxy.example.com")
        ```
        """
        ...

    @staticmethod
    def all(url: str, **kwargs: Unpack[ProxyConfig]) -> "Proxy":
        r"""
        Creates a new proxy for all protocols.

        This method sets up a proxy server for all types of requests (HTTP, HTTPS, etc.).

        # Examples

        ```python
        import wreq

        proxy = wreq.Proxy.all("https://proxy.example.com")
        ```
        """
        ...

    @staticmethod
    def unix(path: str, **kwargs: Unpack[ProxyConfig]) -> "Proxy":
        r"""
        Creates a new UNIX socket proxy.

        This method sets up a proxy server using a UNIX domain socket.

        # Examples

        ```python
        import wreq

        proxy = wreq.Proxy.unix("/var/run/docker.sock")
        ```
        """
        ...

    def __str__(self) -> str: ...
