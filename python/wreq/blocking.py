import datetime
from typing import (
    Any,
    Sequence,
    Unpack,
)

from . import (
    ClientConfig,
    Message,
    Method,
    Request,
    SocketAddr,
    StatusCode,
    Streamer,
    Version,
    WebSocketRequest,
)
from .cookie import Cookie, Jar
from .header import HeaderMap
from .redirect import History
from .runtime import Runtime
from .tls import TlsInfo


class Response:
    r"""
    A blocking response from a request.
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
        Stream the body as read-only memoryviews, with HeaderMap frames for trailers.
        """
        ...

    def text(self, encoding: str | None = None) -> str:
        r"""
        Get the text content with the response encoding, defaulting to utf-8 when unspecified.
        """
        ...

    def json(self) -> Any:
        r"""
        Get the JSON content of the response.
        """

    def bytes(self) -> memoryview:
        r"""
        Read the body as a read-only memoryview without copying it into Python bytes.
        The view remains valid after the response is closed or deleted.
        Use bytes(view) or view.tobytes() for a copy; view.release() releases this view.
        """
        ...

    def close(self) -> None:
        r"""
        Discard the retained body and mark its connection as non-reusable.
        This does not guarantee an immediate socket shutdown or interrupt an active read.
        Do not close concurrently with a body read. A body transferred to a Streamer
        is managed separately; previously returned memoryviews remain valid.
        `with` instead releases the body and keeps a fully read connection reusable.
        """

    def __enter__(self) -> Any: ...
    def __exit__(self, _exc_type: Any, _exc_value: Any, _traceback: Any) -> None:
        r"""
        Release the body without forbidding reuse: a fully read connection returns
        to the pool, while an unread HTTP/1 body drains or closes its connection.
        """

    def __str__(self) -> str: ...


class WebSocket:
    r"""
    A blocking WebSocket response.
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

    def recv(self, timeout: datetime.timedelta | None = None) -> Message | None:
        r"""
        Receive a message from the WebSocket.
        """

    def send(self, message: Message) -> None:
        r"""
        Send a message to the WebSocket.

        # Arguments

        * `message` - The message to send.
        """

    def send_all(self, messages: Sequence[Message]) -> None:
        r"""
        Send multiple messages to the WebSocket.

        # Arguments

        * `messages` - The sequence of messages to send.
        """

    def close(
        self,
        code: int | None = None,
        reason: str | None = None,
    ) -> None:
        r"""
        Close the WebSocket connection.

        # Arguments

        * `code` - An optional close code.
        * `reason` - An optional reason for closing.
        """

    def __enter__(self) -> Any: ...
    def __exit__(self, _exc_type: Any, _exc_value: Any, _traceback: Any) -> None:
        r"""
        Close the WebSocket connection without a close code or reason, unless already closed.
        """

    def __str__(self) -> str: ...


class Client:
    r"""
    A blocking client for making HTTP requests.
    """

    runtime: Runtime
    """Read-only shared runtime used by this client and its responses."""

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
        Creates a new blocking Client instance.

        # Examples

        ```python
        import asyncio
        import wreq

        client = wreq.blocking.Client(
            user_agent="Mozilla/5.0",
            timeout=10,
        )
        response = client.get('https://httpbin.io/get')
        print(response.text())
        ```
        """
        ...

    def close(self) -> None:
        r"""
        Cancels pending requests and rejects new ones with `asyncio.CancelledError`.
        Existing responses, WebSockets and the shared runtime remain usable.

        Examples:

        ```python
        import asyncio
        import wreq

        client = wreq.blocking.Client()

        response = client.get('https://httpbin.io/get')
        print(response.text())

        client.close()
        ```
        """
        ...

    def request(
        self,
        method: Method,
        url: str,
        **kwargs: Unpack[Request],
    ) -> "Response":
        r"""
        Sends a request with the given method and URL.

        # Examples

        ```python
        import wreq
        import wreq.blocking
        from wreq import Method

        client = wreq.blocking.Client()
        response = client.request(Method.GET, "https://httpbin.io/anything")
        ```
        """
        ...

    def websocket(self, url: str, **kwargs: Unpack[WebSocketRequest]) -> "WebSocket":
        r"""
        Sends a WebSocket request.

        # Examples

        ```python
        import wreq
        import wreq.blocking

        client = wreq.blocking.Client()
        ws = client.websocket("wss://echo.websocket.org")
        ws.send(wreq.Message.from_text("Hello, WebSocket!"))
        message = ws.recv()
        print("Received:", message.data)
        ws.close()
        ```
        """
        ...

    def trace(
        self,
        url: str,
        **kwargs: Unpack[Request],
    ) -> "Response":
        r"""
        Sends a request with the given URL.

        # Examples

        ```python
        import wreq
        import wreq.blocking
        from wreq import Method

        client = wreq.blocking.Client()
        response = client.trace("https://httpbin.io/anything")
        print(response.text())
        ```
        """
        ...

    def options(
        self,
        url: str,
        **kwargs: Unpack[Request],
    ) -> "Response":
        r"""
        Sends a request with the given URL.

        # Examples

        ```python
        import wreq
        import wreq.blocking
        from wreq import Method

        client = wreq.blocking.Client()
        response = client.options("https://httpbin.io/anything")
        print(response.text())
        ```
        """
        ...

    def head(
        self,
        url: str,
        **kwargs: Unpack[Request],
    ) -> "Response":
        r"""
        Sends a request with the given URL.

        # Examples

        ```python
        import wreq
        import wreq.blocking
        from wreq import Method

        client = wreq.blocking.Client()
        response = client.head("https://httpbin.io/anything")
        print(response.text())
        ```
        """
        ...

    def delete(
        self,
        url: str,
        **kwargs: Unpack[Request],
    ) -> "Response":
        r"""
        Sends a request with the given URL.

        # Examples

        ```python
        import wreq
        import wreq.blocking
        from wreq import Method

        client = wreq.blocking.Client()
        response = client.delete("https://httpbin.io/anything")
        print(response.text())
        ```
        """
        ...

    def patch(
        self,
        url: str,
        **kwargs: Unpack[Request],
    ) -> "Response":
        r"""
        Sends a request with the given URL.

        # Examples

        ```python
        import wreq
        import wreq.blocking
        from wreq import Method

        client = wreq.blocking.Client()
        response = client.patch("https://httpbin.io/anything", json={"key": "value"})
        print(response.text())
        ```
        """
        ...

    def put(
        self,
        url: str,
        **kwargs: Unpack[Request],
    ) -> "Response":
        r"""
        Sends a request with the given URL.

        # Examples

        ```python
        import wreq
        import wreq.blocking
        from wreq import Method

        client = wreq.blocking.Client()
        response = client.put("https://httpbin.io/anything", json={"key": "value"})
        print(response.text())
        ```
        """
        ...

    def post(
        self,
        url: str,
        **kwargs: Unpack[Request],
    ) -> "Response":
        r"""
        Sends a request with the given URL.

        # Examples

        ```python
        import wreq
        import wreq.blocking
        from wreq import Method

        client = wreq.blocking.Client()
        response = client.post("https://httpbin.io/anything", json={"key": "value"})
        print(response.text())
        ```
        """
        ...

    def get(
        self,
        url: str,
        **kwargs: Unpack[Request],
    ) -> "Response":
        r"""
        Sends a request with the given URL.

        # Examples

        ```python
        import wreq
        import wreq.blocking
        from wreq import Method

        client = wreq.blocking.Client()
        response = client.get("https://httpbin.io/anything")
        print(response.text())
        ```
        """
        ...

    def __enter__(self) -> Any: ...
    def __exit__(self, _exc_type: Any, _exc_value: Any, _traceback: Any) -> None:
        r"""
        Close the client like `close()`: cancel pending requests and reject new ones.
        """


def delete(
    url: str,
    **kwargs: Unpack[Request],
) -> "Response":
    r"""
    Shortcut method to quickly make a request.

    # Examples

    ```python
    import wreq
    import wreq.blocking

    response = wreq.blocking.delete("https://httpbin.io/anything")
    print(response.text())
    ```
    """
    ...


def get(
    url: str,
    **kwargs: Unpack[Request],
) -> "Response":
    r"""
    Shortcut method to quickly make a request.

    # Examples

    ```python
    import wreq
    import wreq.blocking

    response = wreq.blocking.get("https://httpbin.io/anything")
    print(response.text())
    ```
    """
    ...


def head(
    url: str,
    **kwargs: Unpack[Request],
) -> "Response":
    r"""
    Shortcut method to quickly make a request.

    # Examples

    ```python
    import wreq
    import wreq.blocking

    response = wreq.blocking.head("https://httpbin.io/anything")
    print(response.status)
    ```
    """
    ...


def options(
    url: str,
    **kwargs: Unpack[Request],
) -> "Response":
    r"""
    Shortcut method to quickly make a request.

    # Examples

    ```python
    import wreq
    import wreq.blocking

    response = wreq.blocking.options("https://httpbin.io/anything")
    print(response.status)
    ```
    """
    ...


def patch(
    url: str,
    **kwargs: Unpack[Request],
) -> "Response":
    r"""
    Shortcut method to quickly make a request.

    # Examples

    ```python
    import wreq
    import wreq.blocking

    response = wreq.blocking.patch("https://httpbin.io/anything", json={"key": "value"})
    print(response.text())
    ```
    """
    ...


def post(
    url: str,
    **kwargs: Unpack[Request],
) -> "Response":
    r"""
    Shortcut method to quickly make a request.

    # Examples

    ```python
    import wreq
    import wreq.blocking

    response = wreq.blocking.post("https://httpbin.io/anything", json={"key": "value"})
    print(response.text())
    ```
    """
    ...


def put(
    url: str,
    **kwargs: Unpack[Request],
) -> "Response":
    r"""
    Shortcut method to quickly make a request.

    # Examples

    ```python
    import wreq
    import wreq.blocking

    response = wreq.blocking.put("https://httpbin.io/anything", json={"key": "value"})
    print(response.text())
    ```
    """
    ...


def request(
    method: Method,
    url: str,
    **kwargs: Unpack[Request],
) -> "Response":
    r"""
    Make a request with the given parameters.

    # Arguments

    * `method` - The method to use for the request.
    * `url` - The URL to send the request to.
    * `**kwargs` - Additional request parameters.

    # Examples

    ```python
    import wreq
    import wreq.blocking
    from wreq import Method

    response = wreq.blocking.request(Method.GET, "https://www.rust-lang.org")
    print(response.text())
    ```
    """
    ...


def trace(
    url: str,
    **kwargs: Unpack[Request],
) -> "Response":
    r"""
    Shortcut method to quickly make a request.

    # Examples

    ```python
    import wreq
    import wreq.blocking

    response = wreq.blocking.trace("https://httpbin.io/anything")
    print(response.status)
    ```
    """
    ...


def websocket(
    url: str,
    **kwargs: Unpack[WebSocketRequest],
) -> "WebSocket":
    r"""
    Make a WebSocket connection with the given parameters.

    # Examples

    ```python
    import wreq
    import wreq.blocking

    ws = wreq.blocking.websocket("wss://echo.websocket.org")
    ws.send(wreq.Message.from_text("Hello, World!"))
    message = ws.recv()
    print("Received:", message.data)
    ws.close()
    ```
    """
    ...
