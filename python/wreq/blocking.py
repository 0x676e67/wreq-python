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
from .proxy import Proxy
from .redirect import History
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

    def text(self, encoding: str | None = None) -> str:
        r"""
        Get the text content with the response encoding, defaulting to utf-8 when unspecified.
        """
        ...

    def json(self) -> Any:
        r"""
        Get the JSON content of the response.
        """

    def bytes(self) -> bytes:
        r"""
        Get the bytes content of the response.
        """
        ...

    def close(self) -> None:
        r"""
        Close the response.

        This method closes the network connection regardless of whether connection pooling is
        enabled or not. It is recommended to use context managers (`with` statement) to properly
        manage response lifecycle instead of calling this method manually.
        """

    def __enter__(self) -> Any: ...
    def __exit__(self, _exc_type: Any, _exc_value: Any, _traceback: Any) -> None: ...
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
    def __exit__(self, _exc_type: Any, _exc_value: Any, _traceback: Any) -> None: ...
    def __str__(self) -> str: ...


class Client:
    r"""
    A blocking client for making HTTP requests.
    """

    cookie_jar: Jar | None
    r"""
    Get the cookie jar used by this client (if enabled/configured).

    Returns:
        - The provided `Jar` if the client was constructed with `cookie_provider=...`
        - The auto-created `Jar` if the client was constructed with `cookie_store=True`
    """

    proxies: Proxy | Sequence[Proxy] | None
    r"""
    Get or set the proxies used by this client.

    A single `Proxy` or a sequence of them can be assigned; reading the attribute
    always gives back a list, or `None` when the client has no proxies.

    Assigning a new value rebuilds the client with the given proxies, keeping the rest
    of its configuration and its cookie jar. Requests made afterwards go through the
    new proxies, while the ones already in flight keep using the previous ones.
    Assigning `None` restores the default behaviour of using the system proxies.

    Note that a client created with `no_proxy=True` never uses a proxy, exactly like
    when both options are given to the constructor.

    Examples:

    ```python
    import wreq

    client = wreq.blocking.Client(proxies=[wreq.Proxy.all("http://proxy.example.com:8080")])
    client.proxies = wreq.Proxy.all("http://other.example.com:8080")
    ```
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
        Closes the client and any associated resources.

        After calling this method, the client should not be used to make further requests.

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
    def __exit__(self, _exc_type: Any, _exc_value: Any, _traceback: Any) -> None: ...


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
