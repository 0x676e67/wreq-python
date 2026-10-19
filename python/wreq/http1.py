"""
HTTP/1 connection configuration.
"""

from typing import TypedDict, Unpack, NotRequired, final

__all__ = ["Http1Options", "Params"]


class Params(TypedDict):
    """
    All parameters for HTTP/1 connections.

    Unknown keyword arguments raise `TypeError` when constructing `Http1Options`.
    """

    http09_responses: NotRequired[bool]
    """
    Enable support for HTTP/0.9 responses.
    """

    writev: NotRequired[bool]
    """
    Whether to use vectored writes for HTTP/1 connections.
    """

    max_headers: NotRequired[int]
    """
    Maximum number of headers allowed in HTTP/1 responses.
    """

    read_buf_exact_size: NotRequired[int]
    """
    Exact size of the read buffer to use.
    """

    max_buf_size: NotRequired[int]
    """
    Maximum buffer size for HTTP/1 connections.
    """

    allow_spaces_after_header_name_in_responses: NotRequired[bool]
    """
    Allow spaces after header names.
    """

    ignore_invalid_headers_in_responses: NotRequired[bool]
    """
    Ignore invalid headers in responses.
    """

    allow_obsolete_multiline_headers_in_responses: NotRequired[bool]
    """
    Allow obsolete multiline headers.
    """


@final
class Http1Options:
    """
    HTTP/1 protocol options for customizing connection behavior.
    These options allow you to customize the behavior of HTTP/1 connections,
    such as enabling support for HTTP/0.9 responses, header case preservation, etc.
    """

    def __init__(self, **kwargs: Unpack[Params]) -> None:
        """
        Crate a new Http1Options instance.
        """
        ...

    def __str__(self) -> str:
        """
        Return a string representation of the type.
        """
        ...
