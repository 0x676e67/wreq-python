import datetime
from enum import Enum, auto
from ipaddress import IPv4Address, IPv6Address
from pathlib import Path
from typing import (
    Any,
    AsyncGenerator,
    Mapping,
    Generator,
    NotRequired,
    Sequence,
    Mapping,
    Tuple,
    TypedDict,
    Unpack,
    final,
)

from . import redirect
from . import emulation
from .cookie import *
from .dns import DnsOptions
from .emulation import *
from .header import *
from .http1 import Http1Options
from .http2 import Http2Options
from .proxy import *
from .redirect import History
from .tls import *


@final
class Method(Enum):
    r"""
    An HTTP method.
    """

    GET = auto()
    HEAD = auto()
    POST = auto()
    PUT = auto()
    DELETE = auto()
    OPTIONS = auto()
    TRACE = auto()
    PATCH = auto()


@final
class Version(Enum):
    r"""
    An HTTP version.
    """

    HTTP_09 = auto()
    HTTP_10 = auto()
    HTTP_11 = auto()
    HTTP_2 = auto()
    HTTP_3 = auto()


@final
class StatusCode:
    r"""
    HTTP status code.
    """

    def as_int(self) -> int:
        r"""
        Return the status code as an integer.
        """
        ...

    def is_informational(self) -> bool:
        r"""
        Check if status is within 100-199.
        """
        ...

    def is_success(self) -> bool:
        r"""
        Check if status is within 200-299.
        """
        ...

    def is_redirection(self) -> bool:
        r"""
        Check if status is within 300-399.
        """
        ...

    def is_client_error(self) -> bool:
        r"""
        Check if status is within 400-499.
        """
        ...

    def is_server_error(self) -> bool:
        r"""
        Check if status is within 500-599.
        """
        ...

    def __str__(self) -> str: ...
    def __richcmp__(self, other: Any, op: int) -> bool: ...


@final
class SocketAddr:
    r"""
    A IP socket address.
    """

    def __str__(self) -> str: ...
    def ip(self) -> IPv4Address | IPv6Address:
        r"""
        Returns the IP address of the socket address.
        """
        ...

    def port(self) -> int:
        r"""
        Returns the port number of the socket address.
        """
        ...


@final
class Multipart:
    r"""
    A multipart form for a request.
    """

    def __init__(self, *parts: "Part") -> None:
        r"""
        Creates a new multipart form.
        """
        ...


@final
class Part:
    r"""
    A part of a multipart form.
    """

    def __init__(
        self,
        name: str,
        value: (
            str
            | bytes
            | Path
            | Generator[bytes, str, None]
            | AsyncGenerator[bytes, str]
        ),
        filename: str | None = None,
        mime: str | None = None,
        length: int | None = None,
        headers: HeaderMap | None = None,
    ) -> None:
        r"""
        Creates a new part.

        # Arguments
        - `name` - The name of the part.
        - `value` - The value of the part, either text, bytes, a file path, or a async or sync stream.
        - `filename` - The filename of the part.
        - `mime` - The MIME type of the part.
        - `length` - The length of the part when value is a stream (e.g., for file uploads).
        - `headers` - The custom headers for the part.
        """
        ...


class Message:
    r"""
    A WebSocket message.
    """

    data: bytes | None
    r"""
    Returns the data of the message as bytes.
    """

    text: str | None
    r"""
    Returns the text content of the message if it is a text message.
    """

    binary: bytes | None
    r"""
    Returns the binary data of the message if it is a binary message.
    """

    ping: bytes | None
    r"""
    Returns the ping data of the message if it is a ping message.
    """

    pong: bytes | None
    r"""
    Returns the pong data of the message if it is a pong message.
    """

    close: Tuple[int, str | None] | None
    r"""
    Returns the close code and reason of the message if it is a close message.
    """

    json: Any
    r"""
    Returns the JSON representation of the message if it is a text message with JSON content.
    """

    @staticmethod
    def from_binary(data: bytes | Any) -> "Message":
        r"""
        Creates a new binary message.

        # Arguments

        * `data` - The binary data or any JSON-serializable data of the message.
        """
        ...

    @staticmethod
    def from_text(data: str | Any) -> "Message":
        r"""
        Creates a new text message.

        # Arguments

        * `data` - The text content or any JSON-serializable data of the message.
        """
        ...

    @staticmethod
    def from_ping(data: bytes) -> "Message":
        r"""
        Creates a new ping message.

        # Arguments

        * `data` - The ping data of the message.
        """
        ...

    @staticmethod
    def from_pong(data: bytes) -> "Message":
        r"""
        Creates a new pong message.

        # Arguments

        * `data` - The pong data of the message.
        """
        ...

    @staticmethod
    def from_close(code: int, reason: str | None = None) -> "Message":
        r"""
        Creates a new close message.

        # Arguments

        * `code` - The close code.
        * `reason` - An optional reason for closing.
        """
        ...

    def __str__(self) -> str: ...


class Streamer:
    r"""
    A stream response.
    An asynchronous iterator yielding data chunks (bytes) or HTTP trailers (HeaderMap) from the response stream.
    Used to stream response content and receive HTTP trailers if present.
    Implemented in the `stream` method of the `Response` class.
    Can be used in an asynchronous for loop in Python.

    When streaming a response, each iteration yields either a bytes object (for body data) or a HeaderMap (for HTTP trailers, if the server sends them).
    This allows you to access HTTP/1.1 or HTTP/2 trailers in addition to the main body.

    # Examples

    ```python
    import asyncio
    import wreq
    from wreq import Method, Emulation, HeaderMap

    async def main():
        resp = await wreq.get("https://example.com/stream-with-trailers")
        async with resp.stream() as streamer:
            async for chunk in streamer:
                if isinstance(chunk, bytes):
                    print("Chunk: ", chunk)
                elif isinstance(chunk, HeaderMap):
                    print("Trailers: ", chunk)
                await asyncio.sleep(0.1)

    if __name__ == "__main__":
        asyncio.run(main())
    ```
    """

    def __iter__(self) -> "Streamer": ...
    def __next__(self) -> bytes | HeaderMap: ...
    def __enter__(self) -> Any: ...
    def __exit__(self, _exc_type: Any, _exc_value: Any, _traceback: Any) -> None: ...
    async def __aiter__(self) -> "Streamer": ...
    async def __anext__(self) -> bytes | HeaderMap: ...
    async def __aenter__(self) -> Any: ...
    async def __aexit__(
        self, _exc_type: Any, _exc_value: Any, _traceback: Any
    ) -> None: ...


class Response:
    r"""
    A response from a request.

    # Examples

    ```python
    import asyncio
    import wreq

    async def main():
        response = await wreq.get("https://www.rust-lang.org")
        print("Status Code: ", response.status)
        print("Version: ", response.version)
        print("Response URL: ", response.url)
        print("Headers: ", response.headers)
        print("Content-Length: ", response.content_length)
        print("Encoding: ", response.encoding)
        print("Remote Address: ", response.remote_addr)

        text_content = await response.text()
        print("Text: ", text_content)

    if __name__ == "__main__":
        asyncio.run(main())
    ```
    """

    url: str
    r"""
    Get the URL of the response.
    """

    status: StatusCode
    r"""
    Get the status code of the response.
    """

    version: Version
    r"""
    Get the HTTP version of the response.
    """

    headers: HeaderMap
    r"""
    Get the headers of the response.
    """

    cookies: Sequence[Cookie]
    r"""
    Get the cookies of the response.
    """

    content_length: int | None
    r"""
    Get the content length of the response.
    """

    remote_addr: SocketAddr | None
    r"""
    Get the remote address of the response.
    """

    local_addr: SocketAddr | None
    r"""
    Get the local address of the response.
    """

    history: Sequence[History]
    r"""
    Get the redirect history of the Response.
    """

    tls_info: TlsInfo | None
    r"""
    Get the TLS information of the response.
    """

    def raise_for_status(self) -> None:
        r"""
        Turn a response into an error if the server returned an error.
        """

    def stream(self) -> Streamer:
        r"""
        Get the response into a `Streamer` of `bytes` from the body.
        """
        ...

    async def text(self, encoding: str | None = None) -> str:
        r"""
        Get the text content with the response encoding, defaulting to utf-8 when unspecified.
        """
        ...

    async def json(self) -> Any:
        r"""
        Get the JSON content of the response.
        """

    async def bytes(self) -> bytes:
        r"""
        Get the bytes content of the response.
        """
        ...

    async def close(self) -> None:
        r"""
        Close the response.

        This method closes the network connection regardless of whether connection pooling is
        enabled or not. It is recommended to use async context managers (`async with` statement)
        to properly manage response lifecycle instead of calling this method manually.
        """

    async def __aenter__(self) -> Any: ...
    async def __aexit__(
        self, _exc_type: Any, _exc_value: Any, _traceback: Any
    ) -> Any: ...
    def __str__(self) -> str: ...


class WebSocket:
    r"""
    A WebSocket response.
    """

    status: StatusCode
    r"""
    Get the status code of the response.
    """

    version: Version
    r"""
    Get the HTTP version of the response.
    """

    headers: HeaderMap
    r"""
    Get the headers of the response.
    """

    cookies: Sequence[Cookie]
    r"""
    Get the cookies of the response.
    """

    remote_addr: SocketAddr | None
    r"""
    Get the remote address of the response.
    """

    local_addr: SocketAddr | None
    r"""
    Get the local address of the response.
    """

    protocol: str | None
    r"""
    Get the WebSocket protocol.
    """

    async def recv(self, timeout: datetime.timedelta | None = None) -> Message | None:
        r"""
        Receive a message from the WebSocket.
        """

    async def send(self, message: Message) -> None:
        r"""
        Send a message to the WebSocket.
        """

    async def send_all(self, messages: Sequence[Message]) -> None:
        r"""
        Send multiple messages to the WebSocket.
        """

    async def close(
        self,
        code: int | None = None,
        reason: str | None = None,
    ) -> None:
        r"""
        Close the WebSocket connection.
        """

    def __aenter__(self) -> Any: ...
    def __aexit__(self, _exc_type: Any, _exc_value: Any, _traceback: Any) -> Any: ...
    def __str__(self) -> str: ...


class ClientConfig(TypedDict):
    emulation: NotRequired[emulation.Emulation | emulation.Profile]
    """Emulation config."""

    user_agent: NotRequired[str]
    """
    Sets the `User-Agent` header to be used by this client.
    """

    headers: NotRequired[Mapping[str, str] | HeaderMap]
    """
    Sets the default headers for every request.
    """

    orig_headers: NotRequired[Sequence[str] | OrigHeaderMap]
    """
    Sets the original headers for every request.
    """

    referer: NotRequired[bool]
    """
    Enable or disable automatic setting of the `Referer` header.
    """

    redirect: NotRequired[redirect.Policy]
    """
    Set a `redirect.Policy` for this client.
    """

    raise_for_status: NotRequired[bool]
    """
    Enable or disable automatic raising of exceptions for HTTP status codes.
    """

    cookie_store: NotRequired[bool]
    """
    Enable a persistent cookie store for the client.
    """

    cookie_provider: NotRequired[Jar]
    """
    Set the persistent cookie store for the client.
    
    Cookies received in responses will be passed to this store, and
    additional requests will query this store for cookies.
    
    By default, no cookie store is used.
    """

    # ========= Timeout options ========

    timeout: NotRequired[datetime.timedelta]
    """
    Enables a request timeout.
    
    The timeout is applied from when the request starts connecting until the
    response body has finished.
    
    Default is no timeout.
    """

    connect_timeout: NotRequired[datetime.timedelta]
    """
    Set a timeout for only the connect phase of a `Client`.
    """

    read_timeout: NotRequired[datetime.timedelta]
    """
    Set a timeout for only the read phase of a `Client`.
    """

    # ======== TCP options ========

    tcp_keepalive: NotRequired[datetime.timedelta]
    """
    Set that all sockets have `SO_KEEPALIVE` set with the supplied duration.
    
    Default is 15 seconds.
    """

    tcp_keepalive_interval: NotRequired[datetime.timedelta]
    """
    Set that all sockets have `SO_KEEPALIVE` set with the supplied interval.

    Default is 15 seconds.
    """

    tcp_keepalive_retries: NotRequired[int]
    """
    Set that all sockets have `SO_KEEPALIVE` set with the supplied retry count.
    
    Default is 3 retries.
    """

    tcp_user_timeout: NotRequired[datetime.timedelta]
    """
    Set that all sockets have `TCP_USER_TIMEOUT` set with the supplied duration.
    
    This option controls how long transmitted data may remain unacknowledged before
    the connection is force-closed.
    
    Default is 30 seconds.
    """

    tcp_nodelay: NotRequired[bool]
    """
    Set whether sockets have `TCP_NODELAY` enabled.
    
    Default is `True`.
    """

    tcp_reuse_address: NotRequired[bool]
    """
    Enable SO_REUSEADDR.
    """

    # ======== Connection pool options ========

    pool_idle_timeout: NotRequired[datetime.timedelta]
    """
    Set an optional timeout for idle sockets being kept-alive.
    """

    pool_max_idle_per_host: NotRequired[int]
    """
    Sets the maximum idle connection per host allowed in the pool.
    """

    pool_max_size: NotRequired[int]
    """
    Sets the maximum number of connections in the pool.
    """

    # ======== HTTP options ========

    http1_only: NotRequired[bool]
    """
    Only use HTTP/1.
    """

    http2_only: NotRequired[bool]
    """
    Only use HTTP/2.
    """

    https_only: NotRequired[bool]
    """
    Restrict the Client to be used with HTTPS only requests.
    """

    http1_options: NotRequired[Http1Options]
    """
    Sets the HTTP/1 options for the client.
    """

    http2_options: NotRequired[Http2Options]
    """
    Sets the HTTP/2 options for the client.
    """

    # ======== TLS options ========

    tls_verify: NotRequired[bool | Path | CertStore]
    """
    Sets whether to verify TLS certificates.
    """

    tls_verify_hostname: NotRequired[bool]
    """
    Configures the use of hostname verification when connecting.
    """

    tls_identity: NotRequired[Identity]
    """
    Represents a private key and X509 cert as a client certificate.
    """

    tls_keylog: NotRequired[KeyLog]
    """
    Key logging policy (environment or file).
    """

    tls_info: NotRequired[bool]
    """
    Add TLS information as `TlsInfo` extension to responses.
    """

    tls_min_version: NotRequired[TlsVersion]
    """
    Minimum TLS version.
    """

    tls_max_version: NotRequired[TlsVersion]
    """
    Maximum TLS version.
    """

    tls_options: NotRequired[TlsOptions]
    """
    Sets the TLS options.
    """

    # ======== Network options ========

    no_proxy: NotRequired[bool]
    """
    Clear all `proxies`, so `Client` will use no proxy anymore.

    This also disables the automatic usage of the "system" proxy.
    """

    proxies: NotRequired[Sequence[Proxy]]
    """
    The proxies to use for requests.
    """

    local_address: NotRequired[IPv4Address | IPv6Address]
    """
    Bind to a local IP Address.
    """

    local_addresses: NotRequired[Tuple[IPv4Address | None, IPv6Address | None]]
    """
    Bind to dual-stack local IP Addresses.
    """

    interface: NotRequired[str]
    """
    Bind connections only on the specified network interface.
    
    This option is only available on the following operating systems:
    
    - Android
    - Fuchsia
    - Linux
    - macOS and macOS-like systems (iOS, tvOS, watchOS and visionOS)
    - Solaris and illumos
    
    On Android, Linux, and Fuchsia, this uses the
    [`SO_BINDTODEVICE`][man-7-socket] socket option. On macOS and macOS-like
    systems, Solaris, and illumos, this instead uses the [`IP_BOUND_IF` and
     `IPV6_BOUND_IF`][man-7p-ip] socket options (as appropriate).
    
    Note that connections will fail if the provided interface name is not a
    network interface that currently exists when a connection is established.

    [man-7-socket]: https://man7.org/linux/man-pages/man7/socket.7.html
    [man-7p-ip]: https://docs.oracle.com/cd/E86824_01/html/E54777/ip-7p.html
    """

    # ========= DNS options =========

    dns_options: NotRequired[DnsOptions]

    # ========= Compression options =========

    gzip: NotRequired[bool]
    """
    Enable auto gzip decompression by checking the `Content-Encoding` response header.
    """

    brotli: NotRequired[bool]
    """
    Enable auto brotli decompression by checking the `Content-Encoding` response header.
    """

    deflate: NotRequired[bool]
    """
    Enable auto deflate decompression by checking the `Content-Encoding` response header.
    """
    zstd: NotRequired[bool]
    """
    Enable auto zstd decompression by checking the `Content-Encoding` response header.
    """


class Request(TypedDict):
    emulation: NotRequired[emulation.Emulation | emulation.Profile]
    """
    The Emulation settings for the request.
    """

    headers: NotRequired[Mapping[str, str] | HeaderMap]
    """
    The headers to use for the request.
    """

    orig_headers: NotRequired[Sequence[str] | OrigHeaderMap]
    """
    The original headers to use for the request.
    """

    default_headers: NotRequired[bool]
    """
    The option enables default headers.
    """

    cookies: NotRequired[str | Mapping[str, str]]
    """
    The cookies to use for the request.
    """

    proxy: NotRequired[Proxy]
    """
    The proxy to use for the request.
    """

    local_address: NotRequired[IPv4Address | IPv6Address]
    """
    Bind to a local IP Address.
    """

    local_addresses: NotRequired[Tuple[IPv4Address | None, IPv6Address | None]]
    """
    Bind to dual-stack local IP Addresses.
    """

    interface: NotRequired[str]
    """
    Bind connections only on the specified network interface.
    
    This option is only available on the following operating systems:
    
    - Android
    - Fuchsia
    - Linux
    - macOS and macOS-like systems (iOS, tvOS, watchOS and visionOS)
    - Solaris and illumos
    
    On Android, Linux, and Fuchsia, this uses the
    [`SO_BINDTODEVICE`][man-7-socket] socket option. On macOS and macOS-like
    systems, Solaris, and illumos, this instead uses the [`IP_BOUND_IF` and
     `IPV6_BOUND_IF`][man-7p-ip] socket options (as appropriate).
    
    Note that connections will fail if the provided interface name is not a
    network interface that currently exists when a connection is established.

    [man-7-socket]: https://man7.org/linux/man-pages/man7/socket.7.html
    [man-7p-ip]: https://docs.oracle.com/cd/E86824_01/html/E54777/ip-7p.html
    """

    timeout: NotRequired[datetime.timedelta]
    """
    The timeout to use for the request.
    """

    read_timeout: NotRequired[datetime.timedelta]
    """
    The read timeout to use for the request.
    """

    version: NotRequired[Version]
    """
    The HTTP version to use for the request.
    """

    redirect: NotRequired[redirect.Policy]
    """
    The redirect policy.
    """

    cookie_provider: NotRequired[Jar]
    """
    Set cookie provider for the request.
    """

    gzip: NotRequired[bool]
    """
    Sets gzip as an accepted encoding.
    """

    brotli: NotRequired[bool]
    """
    Sets brotli as an accepted encoding.
    """

    deflate: NotRequired[bool]
    """
    Sets deflate as an accepted encoding.
    """

    zstd: NotRequired[bool]
    """
    Sets zstd as an accepted encoding.
    """

    auth: NotRequired[str]
    """
    The authentication to use for the request.
    """

    bearer_auth: NotRequired[str]
    """
    The bearer authentication to use for the request.
    """

    basic_auth: NotRequired[Tuple[str, str | None]]
    """
    The basic authentication to use for the request.
    """

    query: NotRequired[
        Sequence[Tuple[str, str | int | float | bool]]
        | Mapping[str, str | int | float | bool]
    ]
    """
    The query parameters to use for the request.
    """

    form: NotRequired[
        Sequence[Tuple[str, str | int | float | bool]]
        | Mapping[str, str | int | float | bool]
    ]
    """
    The form parameters to use for the request.
    """

    json: NotRequired[Any]
    """
    The JSON body to use for the request.
    """

    body: NotRequired[
        str
        | bytes
        | Sequence[Tuple[str, str]]
        | Tuple[str, str | int | float | bool]
        | Mapping[str, str | int | float | bool]
        | Any
        | Generator[bytes, str, None]
        | AsyncGenerator[bytes, str]
    ]
    """
    The body to use for the request.
    """

    multipart: NotRequired[Multipart]
    """
    The multipart form to use for the request.
    """


class WebSocketRequest(TypedDict):
    emulation: NotRequired[emulation.Emulation | emulation.Profile]
    """
    The Emulation settings for the request.
    """

    proxy: NotRequired[Proxy]
    """
    The proxy to use for the request.
    """

    local_address: NotRequired[IPv4Address | IPv6Address]
    """
    Bind to a local IP Address.
    """

    local_addresses: NotRequired[Tuple[IPv4Address | None, IPv6Address | None]]
    """
    Bind to dual-stack local IP Addresses.
    """

    interface: NotRequired[str]
    """
    Bind to an interface by SO_BINDTODEVICE.
    """

    headers: NotRequired[Mapping[str, str] | HeaderMap]
    """
    The headers to use for the request.
    """

    orig_headers: NotRequired[Sequence[str] | OrigHeaderMap]
    """
    The original headers to use for the request.
    """

    default_headers: NotRequired[bool]
    """
    The option enables default headers.
    """

    cookies: NotRequired[str | Mapping[str, str]]
    """
    The cookies to use for the request.
    """

    protocols: NotRequired[Sequence[str]]
    """
    The protocols to use for the request.
    """

    version: NotRequired[Version]
    """
    The HTTP version to use for the request.
    """

    auth: NotRequired[str]
    """
    The authentication to use for the request.
    """

    bearer_auth: NotRequired[str]
    """
    The bearer authentication to use for the request.
    """

    basic_auth: NotRequired[Tuple[str, str | None]]
    """
    The basic authentication to use for the request.
    """

    query: NotRequired[
        Sequence[Tuple[str, str | int | float | bool]]
        | Mapping[str, str | int | float | bool]
    ]
    """
    The query parameters to use for the request.
    """

    read_buffer_size: NotRequired[int]
    """
    Read buffer capacity. This buffer is eagerly allocated and used for receiving messages.

    For high read load scenarios a larger buffer, e.g. 128 KiB, improves performance.

    For scenarios where you expect a lot of connections and don't need high read load
    performance a smaller buffer, e.g. 4 KiB, would be appropriate to lower total
    memory usage.

    The default value is 128 KiB.
    """

    write_buffer_size: NotRequired[int]
    """
    The target minimum size of the write buffer to reach before writing the data
    to the underlying stream. The default value is 128 KiB.

    If set to 0 each message will be eagerly written to the underlying stream.
    It is often more optimal to allow them to buffer a little, hence the default value.

    Note: flush() will always fully write the buffer regardless.
    """

    max_write_buffer_size: NotRequired[int]
    """
    The max size of the write buffer in bytes. Setting this can provide backpressure
    in the case the write buffer is filling up due to write errors.
    The default value is unlimited.

    Note: The write buffer only builds up past write_buffer_size when writes to the
    underlying stream are failing. So the write buffer can not fill up if you are not
    observing write errors even if not flushing.

    Note: Should always be at least write_buffer_size + 1 message and probably a little
    more depending on error handling strategy.
    """

    max_message_size: NotRequired[int]
    """
    The maximum size of an incoming message. None means no size limit.
    The default value is 64 MiB which should be reasonably big for all normal use-cases
    but small enough to prevent memory eating by a malicious user.
    """

    max_frame_size: NotRequired[int]
    """
    The maximum size of a single incoming message frame. None means no size limit.
    The limit is for frame payload NOT including the frame header.
    The default value is 16 MiB which should be reasonably big for all normal use-cases
    but small enough to prevent memory eating by a malicious user.
    """

    accept_unmasked_frames: NotRequired[bool]
    """
    When set to True, the server will accept and handle unmasked frames from the client.
    According to RFC 6455, the server must close the connection to the client in such cases,
    however it seems like there are some popular libraries that are sending unmasked frames,
    ignoring the RFC. By default this option is set to False, i.e. according to RFC6455.
    """


class Client:
    r"""
    A client for making HTTP requests.
    """

    cookie_jar: Jar | None
    r"""
    Get the cookie jar used by this client (if enabled/configured).

    Returns:
        - The provided `Jar` if the client was constructed with `cookie_provider=...`
        - The auto-created `Jar` if the client was constructed with `cookie_store=True`
    """

    def __init__(
        self,
        **kwargs: Unpack[ClientConfig],
    ) -> None:
        r"""
        Creates a new Client instance.

        Examples:

        ```python
        import asyncio
        import wreq

        async def main():
            client = wreq.Client(
                user_agent="Mozilla/5.0",
                timeout=10,
            )
            response = await client.get('https://httpbin.io/get')
            print(await response.text())

        asyncio.run(main())
        ```
        """
        ...

    def close(self) -> None:
        r"""
        Closes the client and any associated resources.

        After calling this method, the client should not be used to make further requests.

        Examples:

        ```python
        import asyncio
        import wreq

        async def main():
            client = wreq.Client()

            response = await client.get('https://httpbin.io/get')
            print(await response.text())

            client.close()

        asyncio.run(main())
        ```
        """
        ...

    async def request(
        self,
        method: Method,
        url: str,
        **kwargs: Unpack[Request],
    ) -> Response:
        r"""
        Sends a request with the given method and URL.

        # Examples

        ```python
        import wreq
        import asyncio
        from wreq import Method

        async def main():
            client = wreq.Client()
            response = await client.request(Method.GET, "https://httpbin.io/anything")
            print(await response.text())

        asyncio.run(main())
        ```
        """
        ...

    async def websocket(
        self,
        url: str,
        **kwargs: Unpack[WebSocketRequest],
    ) -> WebSocket:
        r"""
        Sends a WebSocket request.

        # Examples

        ```python
        import wreq
        import asyncio

        async def main():
            client = wreq.Client()
            ws = await client.websocket("wss://echo.websocket.org")
            await ws.send(wreq.Message.from_text("Hello, WebSocket!"))
            message = await ws.recv()
            print("Received:", message.data)
            await ws.close()

        asyncio.run(main())
        ```
        """
        ...

    async def trace(
        self,
        url: str,
        **kwargs: Unpack[Request],
    ) -> Response:
        r"""
        Sends a request with the given URL

        # Examples

        ```python
        import wreq
        import asyncio
        from wreq import Method

        async def main():
            client = wreq.Client()
            response = await client.trace("https://httpbin.io/anything")
            print(await response.text())

        asyncio.run(main())
        ```
        """
        ...

    async def options(
        self,
        url: str,
        **kwargs: Unpack[Request],
    ) -> Response:
        r"""
        Sends a request with the given URL

        # Examples

        ```python
        import wreq
        import asyncio
        from wreq import Method

        async def main():
            client = wreq.Client()
            response = await client.options("https://httpbin.io/anything")
            print(await response.text())

        asyncio.run(main())
        ```
        """
        ...

    async def patch(
        self,
        url: str,
        **kwargs: Unpack[Request],
    ) -> Response:
        r"""
        Sends a request with the given URL

        # Examples

        ```python
        import wreq
        import asyncio
        from wreq import Method

        async def main():
            client = wreq.Client()
            response = await client.patch("https://httpbin.io/anything", json={"key": "value"})
            print(await response.text())

        asyncio.run(main())
        ```
        """
        ...

    async def delete(
        self,
        url: str,
        **kwargs: Unpack[Request],
    ) -> Response:
        r"""
        Sends a request with the given URL

        # Examples

        ```python
        import wreq
        import asyncio
        from wreq import Method

        async def main():
            client = wreq.Client()
            response = await client.delete("https://httpbin.io/anything")
            print(await response.text())

        asyncio.run(main())
        ```
        """
        ...

    async def put(
        self,
        url: str,
        **kwargs: Unpack[Request],
    ) -> Response:
        r"""
        Sends a request with the given URL

        # Examples

        ```python
        import wreq
        import asyncio
        from wreq import Method

        async def main():
            client = wreq.Client()
            response = await client.put("https://httpbin.io/anything", json={"key": "value"})
            print(await response.text())

        asyncio.run(main())
        ```
        """
        ...

    async def post(
        self,
        url: str,
        **kwargs: Unpack[Request],
    ) -> Response:
        r"""
        Sends a request with the given URL

        # Examples

        ```python
        import wreq
        import asyncio
        from wreq import Method

        async def main():
            client = wreq.Client()
            response = await client.post("https://httpbin.io/anything", json={"key": "value"})
            print(await response.text())

        asyncio.run(main())
        ```
        """
        ...

    async def head(
        self,
        url: str,
        **kwargs: Unpack[Request],
    ) -> Response:
        r"""
        Sends a request with the given URL

        # Examples

        ```python
        import wreq
        import asyncio
        from wreq import Method

        async def main():
            client = wreq.Client()
            response = await client.head("https://httpbin.io/anything")
            print(response.status)

        asyncio.run(main())
        ```
        """
        ...

    async def get(
        self,
        url: str,
        **kwargs: Unpack[Request],
    ) -> Response:
        r"""
        Sends a request with the given URL

        # Examples

        ```python
        import wreq
        import asyncio
        from wreq import Method

        async def main():
            client = wreq.Client()
            response = await client.get("https://httpbin.io/anything")
            print(await response.text())

        asyncio.run(main())
        ```
        """
        ...

    async def __aenter__(self) -> Any: ...
    async def __aexit__(
        self, _exc_type: Any, _exc_value: Any, _traceback: Any
    ) -> Any: ...


async def delete(
    url: str,
    **kwargs: Unpack[Request],
) -> Response:
    r"""
    Shortcut method to quickly make a request.

    # Examples

    ```python
    import wreq
    import asyncio

    async def run():
        response = await wreq.delete("https://httpbin.io/anything")
        body = await response.text()
        print(body)

    asyncio.run(run())
    ```
    """
    ...


async def get(
    url: str,
    **kwargs: Unpack[Request],
) -> Response:
    r"""
    Shortcut method to quickly make a request.

    # Examples

    ```python
    import wreq
    import asyncio

    async def run():
        response = await wreq.get("https://httpbin.io/anything")
        body = await response.text()
        print(body)

    asyncio.run(run())
    ```
    """
    ...


async def head(
    url: str,
    **kwargs: Unpack[Request],
) -> Response:
    r"""
    Shortcut method to quickly make a request.

    # Examples

    ```python
    import wreq
    import asyncio

    async def run():
        response = await wreq.head("https://httpbin.io/anything")
        print(response.status)

    asyncio.run(run())
    ```
    """
    ...


async def options(
    url: str,
    **kwargs: Unpack[Request],
) -> Response:
    r"""
    Shortcut method to quickly make a request.

    # Examples

    ```python
    import wreq
    import asyncio

    async def run():
        response = await wreq.options("https://httpbin.io/anything")
        print(response.status)

    asyncio.run(run())
    ```
    """
    ...


async def patch(
    url: str,
    **kwargs: Unpack[Request],
) -> Response:
    r"""
    Shortcut method to quickly make a request.

    # Examples

    ```python
    import wreq
    import asyncio

    async def run():
        response = await wreq.patch("https://httpbin.io/anything")
        body = await response.text()
        print(body)

    asyncio.run(run())
    ```
    """
    ...


async def post(
    url: str,
    **kwargs: Unpack[Request],
) -> Response:
    r"""
    Shortcut method to quickly make a request.

    # Examples

    ```python
    import wreq
    import asyncio

    async def run():
        response = await wreq.post("https://httpbin.io/anything")
        body = await response.text()
        print(body)

    asyncio.run(run())
    ```
    """
    ...


async def put(
    url: str,
    **kwargs: Unpack[Request],
) -> Response:
    r"""
    Shortcut method to quickly make a request.

    # Examples

    ```python
    import wreq
    import asyncio

    async def run():
        response = await wreq.put("https://httpbin.io/anything")
        body = await response.text()
        print(body)

    asyncio.run(run())
    ```
    """
    ...


async def request(
    method: Method,
    url: str,
    **kwargs: Unpack[Request],
) -> Response:
    r"""
    Make a request with the given parameters.

    # Arguments

    * `method` - The method to use for the request.
    * `url` - The URL to send the request to.
    * `**kwargs` - Additional request parameters.

    # Examples

    ```python
    import wreq
    import asyncio
    from wreq import Method

    async def run():
        response = await wreq.request(Method.GET, "https://www.rust-lang.org")
        body = await response.text()
        print(body)

    asyncio.run(run())
    ```
    """
    ...


async def trace(
    url: str,
    **kwargs: Unpack[Request],
) -> Response:
    r"""
    Shortcut method to quickly make a request.

    # Examples

    ```python
    import wreq
    import asyncio

    async def run():
        response = await wreq.trace("https://httpbin.io/anything")
        print(response.status)

    asyncio.run(run())
    ```
    """
    ...


async def websocket(
    url: str,
    **kwargs: Unpack[WebSocketRequest],
) -> WebSocket:
    r"""
    Make a WebSocket connection with the given parameters.

    # Examples

    ```python
    import wreq
    import asyncio
    from wreq import Message

    async def run():
        ws = await wreq.websocket("wss://echo.websocket.org")
        await ws.send(Message.from_text("Hello, World!"))
        message = await ws.recv()
        print("Received:", message.data)
        await ws.close()

    asyncio.run(run())
    ```
    """
    ...
