"""
HTTP/2 connection configuration.
"""

import datetime
from enum import Enum, auto
from typing import ClassVar, Self, TypedDict, NotRequired, Unpack, final

__all__ = [
    "StreamId",
    "StreamDependency",
    "Priority",
    "Priorities",
    "PseudoId",
    "PseudoOrder",
    "SettingId",
    "SettingsOrder",
    "Params",
    "Http2Options",
]


@final
class PseudoId(Enum):
    """
    Represents the order of HTTP/2 pseudo-header fields in the header block.

    HTTP/2 pseudo-header fields are a set of predefined header fields that start with ':'.
    The order of these fields in a header block is significant. This enum defines the
    possible pseudo-header fields and their standard order according to RFC 7540.
    """

    METHOD = auto()
    SCHEME = auto()
    AUTHORITY = auto()
    PATH = auto()
    PROTOCOL = auto()
    STATUS = auto()


@final
class SettingId(Enum):
    """
    An enum that lists all valid settings that can be sent in a SETTINGS frame.

    Each setting has a value that is a 32 bit unsigned integer (6.5.1.).

    See <https://datatracker.ietf.org/doc/html/rfc9113#name-defined-settings>.
    """

    HEADER_TABLE_SIZE = auto()
    """
    This setting allows the sender to inform the remote endpoint
    of the maximum size of the compression table used to decode field blocks,
    in units of octets. The encoder can select any size equal to or less than
    this value by using signaling specific to the compression format inside
    a field block (see [COMPRESSION]). The initial value is 4,096 octets.
    
    [COMPRESSION]: <https://datatracker.ietf.org/doc/html/rfc7541>
    """

    ENABLE_PUSH = auto()
    """
    Enables or disables server push.
    """

    MAX_CONCURRENT_STREAMS = auto()
    """
    Specifies the maximum number of concurrent streams.
    """

    INITIAL_WINDOW_SIZE = auto()
    """
    Sets the initial stream-level flow control window size.
    """

    MAX_FRAME_SIZE = auto()
    """
    Indicates the largest acceptable frame payload size.
    """

    MAX_HEADER_LIST_SIZE = auto()
    """
    Advises the peer of the max field section size.
    """

    ENABLE_CONNECT_PROTOCOL = auto()
    """
    Enables support for the Extended CONNECT protocol.
    """

    NO_RFC7540_PRIORITIES = auto()
    """
    Disable RFC 7540 Stream Priorities.
    [RFC 9218]: <https://www.rfc-editor.org/rfc/rfc9218.html#section-2.1>
    """


@final
class StreamId:
    """
    A stream identifier, as described in [Section 5.1.1] of RFC 7540.

    Streams are identified with an unsigned 31-bit integer. Streams
    initiated by a client MUST use odd-numbered stream identifiers; those
    initiated by the server MUST use even-numbered stream identifiers.  A
    stream identifier of zero (0x0) is used for connection control
    messages; the stream identifier of zero cannot be used to establish a
    new stream.

    [Section 5.1.1]: https://tools.ietf.org/html/rfc7540#section-5.1.1
    """

    ZERO: ClassVar[Self]
    """Stream ID 0."""

    MAX: ClassVar[Self]
    """The maximum allowed stream ID."""

    def __init__(self, value: int) -> None:
        """
        Create a new StreamId.
        """
        ...

    def __str__(self) -> str:
        """
        Return a string representation of the type.
        """
        ...


@final
class StreamDependency:
    """
    Represents a stream dependency in HTTP/2 priority frames.

    A stream dependency consists of three components:
    * A stream identifier that the stream depends on
    * A weight value between 0 and 255 (representing 1-256 in the protocol)
    * An exclusive flag indicating whether this is an exclusive dependency

    # Stream Dependencies

    In HTTP/2, stream dependencies form a dependency tree where each stream
    can depend on another stream. This creates a priority hierarchy that helps
    determine the relative order in which streams should be processed.
    """

    def __init__(
        self, dependency_id: StreamId, weight: int, is_exclusive: bool
    ) -> None:
        """
        Create a new StreamDependency.
        """
        ...

    def __str__(self) -> str:
        """
        Return a string representation of the type.
        """
        ...


@final
class Priority:
    """
    Represents an HTTP/2 PRIORITY frame (type=0x2).

    The PRIORITY frame specifies the sender-advised priority of a stream,
    as described in RFC 7540 Section 5.3. It can be sent in any stream state,
    including idle or closed streams.

    A PRIORITY frame consists of:
    * The stream identifier whose priority is being set
    * A StreamDependency object describing the dependency and weight

    [Section 5.3]: https://tools.ietf.org/html/rfc7540#section-5.3
    """

    def __init__(self, stream_id: StreamId, dependency: StreamDependency) -> None:
        """
        Create a new Priority frame description.
        """
        ...

    def __str__(self) -> str:
        """
        Return a string representation of the type.
        """
        ...


@final
class Priorities:
    """
    A collection of HTTP/2 PRIORITY frames.

    The Priorities class maintains an ordered list of Priority frames,
    which can be used to represent and manage the stream dependency tree
    in HTTP/2. This is useful for pre-configuring stream priorities or
    sending multiple PRIORITY frames at once during connection setup or
    stream reprioritization.
    """

    def __init__(self, *priority: Priority) -> None:
        """
        Create a new Priorities instance.
        """
        ...

    def __str__(self) -> str:
        """
        Return a string representation of the type.
        """
        ...


@final
class PseudoOrder:
    """
    Represents the order of HTTP/2 pseudo-header fields in the header block.

    The PseudoOrder class maintains a list of PseudoId values that define
    the order in which pseudo-header fields should appear in an HTTP/2
    HEADERS frame. This is important because the order of pseudo-headers
    is significant and must follow specific rules as defined in RFC 7540.
    """

    def __init__(self, *pseudo_id: PseudoId) -> None:
        """
        Create a new PseudoOrder instance.
        """
        ...

    def __str__(self) -> str:
        """
        Return a string representation of the type.
        """
        ...


@final
class SettingsOrder:
    """
    Represents the order of HTTP/2 settings parameters in the SETTINGS frame.

    The SettingsOrder class maintains a list of SettingId values that define
    the order in which settings parameters should appear in an HTTP/2
    SETTINGS frame. While the order of settings is not strictly enforced
    by the protocol, having a consistent order can help with readability
    and debugging.
    """

    def __init__(self, *setting_id: SettingId) -> None:
        """
        Create a new SettingsOrder instance.
        """
        ...

    def __str__(self) -> str:
        """
        Return a string representation of the type.
        """
        ...


class Params(TypedDict):
    """
    All parameters for HTTP/2 connections.
    """

    initial_window_size: NotRequired[int]
    """
    Initial window size for HTTP/2 streams.
    """

    initial_connection_window_size: NotRequired[int]
    """
    Initial connection-level window size.
    """

    initial_max_send_streams: NotRequired[int]
    """
    Initial maximum number of send streams.
    """

    initial_stream_id: NotRequired[int]
    """
    Initial stream ID for the connection.
    """

    adaptive_window: NotRequired[bool]
    """
    Whether to use adaptive flow control.
    """

    max_frame_size: NotRequired[int]
    """
    Maximum frame size to use for HTTP/2.
    """

    max_header_list_size: NotRequired[int]
    """
    Maximum size of the header list.
    """

    header_table_size: NotRequired[int]
    """
    Header table size for HPACK compression.
    """

    max_concurrent_streams: NotRequired[int]
    """
    Maximum concurrent streams from remote peer.
    """

    keep_alive_interval: NotRequired[datetime.timedelta]
    """
    Interval for HTTP/2 keep-alive ping frames.
    """

    keep_alive_timeout: NotRequired[datetime.timedelta]
    """
    Timeout for keep-alive ping acknowledgements.
    """

    keep_alive_while_idle: NotRequired[bool]
    """
    Whether keep-alive applies while idle.
    """

    enable_push: NotRequired[bool]
    """
    Whether to enable push promises.
    """

    enable_connect_protocol: NotRequired[bool]
    """
    Whether to enable the CONNECT protocol.
    """

    no_rfc7540_priorities: NotRequired[bool]
    """
    Whether to disable RFC 7540 Stream Priorities.
    """

    max_concurrent_reset_streams: NotRequired[int]
    """
    Max concurrent locally reset streams.
    """

    max_send_buf_size: NotRequired[int]
    """
    Maximum send buffer size for streams.
    """

    max_pending_accept_reset_streams: NotRequired[int]
    """
    Max pending accept reset streams.
    """

    headers_stream_dependency: NotRequired[StreamDependency]
    """
    Stream dependency for outgoing HEADERS.
    """

    headers_pseudo_order: NotRequired[PseudoOrder]
    """
    Order of pseudo-header fields in HEADERS.
    """

    settings_order: NotRequired[SettingsOrder]
    """
    Order of settings parameters in SETTINGS frame.
    """

    priorities: NotRequired[Priorities]
    """
    List of PRIORITY frames to send after connection.
    """


@final
class Http2Options:
    """
    Configuration for an HTTP/2 connection.

    This struct defines various parameters to fine-tune the behavior of an HTTP/2 connection,
    including stream management, window sizes, frame limits, and header config.
    """

    def __init__(self, **kwargs: Unpack[Params]) -> None:
        """
        Create a new Http2Options instance.
        """
        ...

    def __str__(self) -> str:
        """
        Return a string representation of the type.
        """
        ...
