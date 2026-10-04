use std::{fmt::Debug, time::Duration};

use pyo3::prelude::*;

define_enum!(
    /// Represents the order of HTTP/2 pseudo-header fields in the header block.
    ///
    /// HTTP/2 pseudo-header fields are a set of predefined header fields that start with ':'.
    /// The order of these fields in a header block is significant. This enum defines the
    /// possible pseudo-header fields and their standard order according to RFC 7540.
    const,
    PseudoId,
    wreq::http2::PseudoId,
    (METHOD, Method),
    (SCHEME, Scheme),
    (AUTHORITY, Authority),
    (PATH, Path),
    (PROTOCOL, Protocol),
    (STATUS, Status),
);

define_enum!(
    /// An enum that lists all valid settings that can be sent in a SETTINGS
    /// frame.
    ///
    /// Each setting has a value that is a 32 bit unsigned integer (6.5.1.).
    ///
    /// See <https://datatracker.ietf.org/doc/html/rfc9113#name-defined-settings>.
    const,
    SettingId,
    wreq::http2::SettingId,
    (HEADER_TABLE_SIZE, HeaderTableSize),
    (ENABLE_PUSH, EnablePush),
    (MAX_CONCURRENT_STREAMS, MaxConcurrentStreams),
    (INITIAL_WINDOW_SIZE, InitialWindowSize),
    (MAX_FRAME_SIZE, MaxFrameSize),
    (MAX_HEADER_LIST_SIZE, MaxHeaderListSize),
    (ENABLE_CONNECT_PROTOCOL, EnableConnectProtocol),
    (NO_RFC7540_PRIORITIES, NoRfc7540Priorities),
);

/// A stream identifier, as described in [Section 5.1.1] of RFC 7540.
///
/// Streams are identified with an unsigned 31-bit integer. Streams
/// initiated by a client MUST use odd-numbered stream identifiers; those
/// initiated by the server MUST use even-numbered stream identifiers.  A
/// stream identifier of zero (0x0) is used for connection control
/// messages; the stream identifier of zero cannot be used to establish a
/// new stream.
///
/// [Section 5.1.1]: https://tools.ietf.org/html/rfc7540#section-5.1.1
#[derive(Clone)]
#[pyclass(frozen, str, from_py_object)]
pub struct StreamId(wreq::http2::StreamId);

/// Represents a stream dependency in HTTP/2 priority frames.
///
/// A stream dependency consists of three components:
/// * A stream identifier that the stream depends on
/// * A weight value between 0 and 255 (representing 1-256 in the protocol)
/// * An exclusive flag indicating whether this is an exclusive dependency
///
/// # Stream Dependencies
///
/// In HTTP/2, stream dependencies form a dependency tree where each stream
/// can depend on another stream. This creates a priority hierarchy that helps
/// determine the relative order in which streams should be processed.
#[derive(Clone)]
#[pyclass(frozen, str, from_py_object)]
pub struct StreamDependency(wreq::http2::StreamDependency);

/// The PRIORITY frame (type=0x2) specifies the sender-advised priority
/// of a stream [Section 5.3].  It can be sent in any stream state,
/// including idle or closed streams.
/// [Section 5.3]: <https://tools.ietf.org/html/rfc7540#section-5.3>
#[derive(Clone)]
#[pyclass(frozen, str, from_py_object)]
pub struct Priority(wreq::http2::Priority);

/// A collection of HTTP/2 PRIORITY frames.
///
/// The `Priorities` struct maintains an ordered list of `Priority` frames,
/// which can be used to represent and manage the stream dependency tree
/// in HTTP/2. This is useful for pre-configuring stream priorities or
/// sending multiple PRIORITY frames at once during connection setup or
/// stream reprioritization.
#[derive(Clone)]
#[pyclass(frozen, str, from_py_object)]
pub struct Priorities(wreq::http2::Priorities);

/// Represents the order of HTTP/2 pseudo-header fields in a header block.
///
/// This structure maintains an ordered list of pseudo-header fields (such as `:method`, `:scheme`,
/// etc.) for use when encoding or decoding HTTP/2 header blocks. The order of pseudo-headers is
/// significant according to the HTTP/2 specification, and this type ensures that the correct order
/// is preserved and that no duplicates are present.
#[derive(Clone)]
#[pyclass(frozen, str, from_py_object)]
pub struct PseudoOrder(wreq::http2::PseudoOrder);

/// Represents the order of settings in a SETTINGS frame.
///
/// This structure maintains an ordered list of `SettingId` values for use when encoding or decoding
/// HTTP/2 SETTINGS frames. The order of settings can be important for protocol compliance, testing,
/// or interoperability. `SettingsOrder` ensures that the specified order is preserved and that no
/// duplicate settings are present.
#[derive(Clone)]
#[pyclass(frozen, str, from_py_object)]
pub struct SettingsOrder(wreq::http2::SettingsOrder);

/// A builder for [`Http2Options`].
#[derive(Default)]
struct Builder {
    /// The initial window size for HTTP/2 streams.
    initial_window_size: Option<u32>,

    /// The initial window size for HTTP/2 connection-level flow control.
    initial_connection_window_size: Option<u32>,

    /// The initial maximum number of locally initiated (send) streams.
    initial_max_send_streams: Option<usize>,

    /// The initial stream ID for the connection.
    initial_stream_id: Option<u32>,

    /// Whether to use adaptive flow control.
    adaptive_window: Option<bool>,

    /// The maximum frame size to use for HTTP/2.
    max_frame_size: Option<u32>,

    /// The maximum size of the header list.
    max_header_list_size: Option<u32>,

    /// The header table size for HPACK compression.
    header_table_size: Option<u32>,

    /// The maximum number of concurrent streams initiated by the remote peer.
    max_concurrent_streams: Option<u32>,

    /// The interval for HTTP/2 keep-alive ping frames.
    keep_alive_interval: Option<Duration>,

    /// The timeout for receiving an acknowledgement of the keep-alive ping.
    keep_alive_timeout: Option<Duration>,

    /// Whether HTTP/2 keep-alive should apply while the connection is idle.
    keep_alive_while_idle: Option<bool>,

    /// Whether to enable push promises.
    enable_push: Option<bool>,

    /// Whether to enable the CONNECT protocol.
    enable_connect_protocol: Option<bool>,

    /// Whether to disable RFC 7540 Stream Priorities.
    no_rfc7540_priorities: Option<bool>,

    /// The maximum number of concurrent locally reset streams.
    max_concurrent_reset_streams: Option<usize>,

    /// The maximum size of the send buffer for HTTP/2 streams.
    max_send_buf_size: Option<usize>,

    /// The maximum number of pending accept reset streams.
    max_pending_accept_reset_streams: Option<usize>,

    /// The stream dependency for the outgoing HEADERS frame.
    headers_stream_dependency: Option<StreamDependency>,

    /// The HTTP/2 pseudo-header field order for outgoing HEADERS frames.
    headers_pseudo_order: Option<PseudoOrder>,

    /// The order of settings parameters in the initial SETTINGS frame.
    settings_order: Option<SettingsOrder>,

    /// The list of PRIORITY frames to be sent after connection establishment.
    priorities: Option<Priorities>,
}

/// Configuration for an HTTP/2 connection.
///
/// This struct defines various parameters to fine-tune the behavior of an HTTP/2 connection,
/// including stream management, window sizes, frame limits, and header config.
#[derive(Clone)]
#[pyclass(frozen, str, from_py_object)]
pub struct Http2Options(pub wreq::http2::Http2Options);

// ===== impl StreamId =====

#[pymethods]
impl StreamId {
    /// Stream ID 0.
    #[classattr]
    const ZERO: Self = Self(wreq::http2::StreamId::ZERO);

    /// The maximum allowed stream ID.
    #[classattr]
    const MAX: Self = Self(wreq::http2::StreamId::MAX);

    /// Creates a new `StreamId`
    #[new]
    #[pyo3(signature = (value))]
    fn new(value: u32) -> Self {
        Self(wreq::http2::StreamId::from(value))
    }
}

impl_print_str!(Display, StreamId);

// ===== impl StreamDependency =====

#[pymethods]
impl StreamDependency {
    /// Creates a new `StreamDependency`.
    #[new]
    #[pyo3(signature = (dependency_id, weight, is_exclusive))]
    fn new(dependency_id: StreamId, weight: u8, is_exclusive: bool) -> Self {
        Self(wreq::http2::StreamDependency::new(
            dependency_id.0,
            weight,
            is_exclusive,
        ))
    }
}

impl_print_str!(Display, StreamDependency);

// ===== impl Priority =====

#[pymethods]
impl Priority {
    /// Creates a new `Priority`.
    #[new]
    #[pyo3(signature = (stream_id, dependency))]
    fn new(stream_id: StreamId, dependency: StreamDependency) -> Self {
        Self(wreq::http2::Priority::new(stream_id.0, dependency.0))
    }
}

impl_print_str!(Display, Priority);

// ===== impl Priorities =====

#[pymethods]
impl Priorities {
    /// Creates a `Priorities` collection from the given priorities.
    #[new]
    #[pyo3(signature = (*iter))]
    fn new(iter: Vec<Priority>) -> Self {
        Self(
            wreq::http2::Priorities::builder()
                .extend(iter.into_iter().map(|p| p.0))
                .build(),
        )
    }
}

impl_print_str!(Display, Priorities);

// ===== impl PseudoOrder =====

#[pymethods]
impl PseudoOrder {
    /// Creates a `PseudoOrder` from the given pseudo-header IDs.
    #[new]
    #[pyo3(signature = (*iter))]
    fn new(iter: Vec<PseudoId>) -> Self {
        Self(
            wreq::http2::PseudoOrder::builder()
                .extend(iter.into_iter().map(PseudoId::into_ffi))
                .build(),
        )
    }
}

impl_print_str!(Display, PseudoOrder);

// ===== impl SettingsOrder =====

#[pymethods]
impl SettingsOrder {
    /// Creates a `SettingsOrder` from the given setting IDs.
    #[new]
    #[pyo3(signature = (*iter))]
    fn new(iter: Vec<SettingId>) -> Self {
        Self(
            wreq::http2::SettingsOrder::builder()
                .extend(iter.into_iter().map(SettingId::into_ffi))
                .build(),
        )
    }
}

impl_print_str!(Display, SettingsOrder);

// ===== impl Builder =====

impl FromPyObject<'_, '_> for Builder {
    type Error = PyErr;

    fn extract(ob: Borrowed<PyAny>) -> PyResult<Self> {
        let mut params = Self::default();
        extract_option!(ob, params, initial_window_size);
        extract_option!(ob, params, initial_connection_window_size);
        extract_option!(ob, params, initial_max_send_streams);
        extract_option!(ob, params, initial_stream_id);
        extract_option!(ob, params, adaptive_window);
        extract_option!(ob, params, max_frame_size);
        extract_option!(ob, params, max_header_list_size);
        extract_option!(ob, params, header_table_size);
        extract_option!(ob, params, max_concurrent_streams);
        extract_option!(ob, params, keep_alive_interval);
        extract_option!(ob, params, keep_alive_timeout);
        extract_option!(ob, params, keep_alive_while_idle);
        extract_option!(ob, params, enable_push);
        extract_option!(ob, params, enable_connect_protocol);
        extract_option!(ob, params, no_rfc7540_priorities);
        extract_option!(ob, params, max_concurrent_reset_streams);
        extract_option!(ob, params, max_send_buf_size);
        extract_option!(ob, params, max_pending_accept_reset_streams);
        extract_option!(ob, params, headers_stream_dependency);
        extract_option!(ob, params, headers_pseudo_order);
        extract_option!(ob, params, settings_order);
        extract_option!(ob, params, priorities);
        Ok(params)
    }
}

// ===== impl Http2Options =====

#[pymethods]
impl Http2Options {
    #[new]
    #[pyo3(signature = (**kwds))]
    fn new(py: Python, kwds: Option<Builder>) -> Self {
        py.detach(|| {
            let mut builder = wreq::http2::Http2Options::builder();

            if let Some(mut params) = kwds {
                apply_option!(
                    set_if_some,
                    builder,
                    params.initial_window_size,
                    initial_window_size
                );
                apply_option!(
                    set_if_some,
                    builder,
                    params.initial_connection_window_size,
                    initial_connection_window_size
                );
                apply_option!(
                    set_if_some,
                    builder,
                    params.initial_max_send_streams,
                    initial_max_send_streams
                );
                apply_option!(
                    set_if_some,
                    builder,
                    params.initial_stream_id,
                    initial_stream_id
                );
                apply_option!(
                    set_if_some,
                    builder,
                    params.adaptive_window,
                    adaptive_window
                );
                apply_option!(set_if_some, builder, params.max_frame_size, max_frame_size);
                apply_option!(
                    set_if_some,
                    builder,
                    params.max_header_list_size,
                    max_header_list_size
                );
                apply_option!(
                    set_if_some,
                    builder,
                    params.header_table_size,
                    header_table_size
                );
                apply_option!(
                    set_if_some,
                    builder,
                    params.max_concurrent_streams,
                    max_concurrent_streams
                );
                apply_option!(set_if_some, builder, params.enable_push, enable_push);
                apply_option!(
                    set_if_some,
                    builder,
                    params.enable_connect_protocol,
                    enable_connect_protocol
                );
                apply_option!(
                    set_if_some,
                    builder,
                    params.no_rfc7540_priorities,
                    no_rfc7540_priorities
                );
                apply_option!(
                    set_if_some,
                    builder,
                    params.max_concurrent_reset_streams,
                    max_concurrent_reset_streams
                );
                apply_option!(
                    set_if_some,
                    builder,
                    params.max_send_buf_size,
                    max_send_buf_size
                );
                apply_option!(
                    set_if_some,
                    builder,
                    params.max_pending_accept_reset_streams,
                    max_pending_accept_reset_streams
                );
                apply_option!(
                    set_if_some_inner,
                    builder,
                    params.headers_stream_dependency,
                    headers_stream_dependency
                );
                apply_option!(
                    set_if_some_inner,
                    builder,
                    params.headers_pseudo_order,
                    headers_pseudo_order
                );
                apply_option!(
                    set_if_some_inner,
                    builder,
                    params.settings_order,
                    settings_order
                );
                apply_option!(set_if_some_inner, builder, params.priorities, priorities);
                apply_option!(
                    set_if_some,
                    builder,
                    params.keep_alive_interval,
                    keep_alive_interval
                );
                apply_option!(
                    set_if_some,
                    builder,
                    params.keep_alive_timeout,
                    keep_alive_timeout
                );
                apply_option!(
                    set_if_some,
                    builder,
                    params.keep_alive_while_idle,
                    keep_alive_while_idle
                );
            }

            Self(builder.build())
        })
    }
}

impl_print_str!(Debug, Http2Options);
