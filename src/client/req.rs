use std::{
    net::{IpAddr, Ipv4Addr, Ipv6Addr},
    time::Duration,
};

use futures_util::TryFutureExt;
use http::header::COOKIE;
use pyo3::{PyResult, prelude::*, pybacked::PyBackedStr};

use crate::{
    client::{
        Client,
        body::{Body, Form, Json, multipart::Multipart},
        query::Query,
        resp::{Response, WebSocket},
    },
    cookie::{Cookies, Jar},
    emulate::EmulationLike,
    error::Error,
    extractor::Extractor,
    header::{HeaderMap, OrigHeaderMap},
    http::{Method, Version},
    proxy::Proxy,
    redirect,
};

/// The parameters for a request.
#[derive(Default)]
#[non_exhaustive]
pub struct Request {
    /// The Emulation settings for the request.
    emulation: Option<EmulationLike>,

    /// The proxy to use for the request.
    proxy: Option<Proxy>,

    /// Bind to a local IP Address.
    local_address: Option<IpAddr>,

    /// Bind to local IP Addresses (IPv4, IPv6).
    local_addresses: Option<Extractor<(Option<Ipv4Addr>, Option<Ipv6Addr>)>>,

    /// Bind to an interface by `SO_BINDTODEVICE`.
    interface: Option<String>,

    /// The timeout to use for the request.
    timeout: Option<Duration>,

    /// The read timeout to use for the request.
    read_timeout: Option<Duration>,

    /// The HTTP version to use for the request.
    version: Option<Version>,

    /// The headers to use for the request.
    headers: Option<HeaderMap>,

    /// The original headers to use for the request.
    orig_headers: Option<OrigHeaderMap>,

    /// The option enables default headers.
    default_headers: Option<bool>,

    /// The cookies to use for the request.
    cookies: Option<Cookies>,

    /// The redirect policy to use for the request.
    redirect: Option<redirect::Policy>,

    /// The cookie provider to use for the request.
    cookie_provider: Option<Jar>,

    /// Sets gzip as an accepted encoding.
    gzip: Option<bool>,

    /// Sets brotli as an accepted encoding.
    brotli: Option<bool>,

    /// Sets deflate as an accepted encoding.
    deflate: Option<bool>,

    /// Sets zstd as an accepted encoding.
    zstd: Option<bool>,

    /// The authentication to use for the request.
    auth: Option<PyBackedStr>,

    /// The bearer authentication to use for the request.
    bearer_auth: Option<PyBackedStr>,

    /// The basic authentication to use for the request.
    basic_auth: Option<(PyBackedStr, Option<PyBackedStr>)>,

    /// The query parameters to use for the request.
    query: Option<Query>,

    /// The form parameters to use for the request.
    form: Option<Form>,

    /// The JSON body to use for the request.
    json: Option<Json>,

    /// The multipart form to use for the request.
    multipart: Option<Multipart>,

    /// The body to use for the request.
    body: Option<Body>,
}

/// The parameters for a WebSocket request.
#[derive(Default)]
#[non_exhaustive]
pub struct WebSocketRequest {
    /// The Emulation settings for the request.
    emulation: Option<EmulationLike>,

    /// The proxy to use for the request.
    proxy: Option<Proxy>,

    /// Bind to a local IP Address.
    local_address: Option<IpAddr>,

    /// Bind to local IP Addresses (IPv4, IPv6).
    local_addresses: Option<Extractor<(Option<Ipv4Addr>, Option<Ipv6Addr>)>>,

    /// Bind to an interface by `SO_BINDTODEVICE`.
    interface: Option<String>,

    /// The headers to use for the request.
    headers: Option<HeaderMap>,

    /// The original headers to use for the request.
    orig_headers: Option<OrigHeaderMap>,

    /// The option enables default headers.
    default_headers: Option<bool>,

    /// The cookies to use for the request.
    cookies: Option<Cookies>,

    /// The protocols to use for the request.
    protocols: Option<Vec<String>>,

    /// The HTTP version to use for the request.
    version: Option<Version>,

    /// The authentication to use for the request.
    auth: Option<PyBackedStr>,

    /// The bearer authentication to use for the request.
    bearer_auth: Option<PyBackedStr>,

    /// The basic authentication to use for the request.
    basic_auth: Option<(PyBackedStr, Option<PyBackedStr>)>,

    /// The query parameters to use for the request.
    query: Option<Query>,

    /// Read buffer capacity. This buffer is eagerly allocated and used for receiving
    /// messages.
    ///
    /// For high read load scenarios a larger buffer, e.g. 128 KiB, improves performance.
    ///
    /// For scenarios where you expect a lot of connections and don't need high read load
    /// performance a smaller buffer, e.g. 4 KiB, would be appropriate to lower total
    /// memory usage.
    ///
    /// The default value is 128 KiB.
    read_buffer_size: Option<usize>,

    /// The target minimum size of the write buffer to reach before writing the data
    /// to the underlying stream.
    /// The default value is 128 KiB.
    ///
    /// If set to `0` each message will be eagerly written to the underlying stream.
    /// It is often more optimal to allow them to buffer a little, hence the default value.
    ///
    /// Note: [`flush`](WebSocket::flush) will always fully write the buffer regardless.
    write_buffer_size: Option<usize>,

    /// The max size of the write buffer in bytes. Setting this can provide backpressure
    /// in the case the write buffer is filling up due to write errors.
    /// The default value is unlimited.
    ///
    /// Note: The write buffer only builds up past [`write_buffer_size`](Self::write_buffer_size)
    /// when writes to the underlying stream are failing. So the **write buffer can not
    /// fill up if you are not observing write errors even if not flushing**.
    ///
    /// Note: Should always be at least [`write_buffer_size + 1 message`](Self::write_buffer_size)
    /// and probably a little more depending on error handling strategy.
    max_write_buffer_size: Option<usize>,

    /// The maximum size of an incoming message. `None` means no size limit. The default value is
    /// 64 MiB which should be reasonably big for all normal use-cases but small enough to
    /// prevent memory eating by a malicious user.
    max_message_size: Option<usize>,

    /// The maximum size of a single incoming message frame. `None` means no size limit. The limit
    /// is for frame payload NOT including the frame header. The default value is 16 MiB which
    /// should be reasonably big for all normal use-cases but small enough to prevent memory
    /// eating by a malicious user.
    max_frame_size: Option<usize>,

    /// When set to `true`, the server will accept and handle unmasked frames
    /// from the client. According to the RFC 6455, the server must close the
    /// connection to the client in such cases, however it seems like there are
    /// some popular libraries that are sending unmasked frames, ignoring the RFC.
    /// By default this option is set to `false`, i.e. according to RFC 6455.
    accept_unmasked_frames: Option<bool>,
}

// ===== impl Request =====

impl FromPyObject<'_, '_> for Request {
    type Error = PyErr;

    fn extract(ob: Borrowed<PyAny>) -> PyResult<Request> {
        let mut request = Self::default();
        extract_option!(ob, request, emulation);
        extract_option!(ob, request, proxy);
        extract_option!(ob, request, local_address);
        extract_option!(ob, request, local_addresses);
        extract_option!(ob, request, interface);

        extract_option!(ob, request, timeout);
        extract_option!(ob, request, read_timeout);

        extract_option!(ob, request, version);
        extract_option!(ob, request, headers);
        extract_option!(ob, request, orig_headers);
        extract_option!(ob, request, default_headers);
        extract_option!(ob, request, cookies);
        extract_option!(ob, request, redirect);
        extract_option!(ob, request, cookie_provider);
        extract_option!(ob, request, auth);
        extract_option!(ob, request, bearer_auth);
        extract_option!(ob, request, basic_auth);
        extract_option!(ob, request, query);
        extract_option!(ob, request, form);
        extract_option!(ob, request, json);
        extract_option!(ob, request, body);
        extract_option!(ob, request, multipart);

        extract_option!(ob, request, gzip);
        extract_option!(ob, request, brotli);
        extract_option!(ob, request, deflate);
        extract_option!(ob, request, zstd);

        Ok(request)
    }
}

// ===== impl WebSocketRequest =====

impl FromPyObject<'_, '_> for WebSocketRequest {
    type Error = PyErr;

    fn extract(ob: Borrowed<PyAny>) -> PyResult<Self> {
        let mut params = Self::default();
        extract_option!(ob, params, emulation);
        extract_option!(ob, params, proxy);
        extract_option!(ob, params, local_address);
        extract_option!(ob, params, local_addresses);
        extract_option!(ob, params, interface);

        extract_option!(ob, params, version);
        extract_option!(ob, params, headers);
        extract_option!(ob, params, orig_headers);
        extract_option!(ob, params, default_headers);
        extract_option!(ob, params, cookies);
        extract_option!(ob, params, protocols);
        extract_option!(ob, params, auth);
        extract_option!(ob, params, bearer_auth);
        extract_option!(ob, params, basic_auth);
        extract_option!(ob, params, query);

        extract_option!(ob, params, read_buffer_size);
        extract_option!(ob, params, write_buffer_size);
        extract_option!(ob, params, max_write_buffer_size);
        extract_option!(ob, params, max_message_size);
        extract_option!(ob, params, max_frame_size);
        extract_option!(ob, params, accept_unmasked_frames);
        Ok(params)
    }
}

pub async fn execute_request<U>(
    client: Client,
    method: Method,
    url: U,
    request: Option<Request>,
) -> PyResult<Response>
where
    U: AsRef<str>,
{
    // Create the request builder.
    let mut builder = client
        .inner
        .load()
        .client
        .request(method.into_ffi(), url.as_ref());

    if let Some(mut request) = request {
        // Emulation options.
        apply_option!(set_if_some, builder, request.emulation, emulation);

        // Version options.
        apply_option!(
            set_if_some_map,
            builder,
            request.version,
            version,
            Version::into_ffi
        );

        // Timeout options.
        apply_option!(set_if_some, builder, request.timeout, timeout);
        apply_option!(set_if_some, builder, request.read_timeout, read_timeout);

        // Network options.
        apply_option!(set_if_some_inner, builder, request.proxy, proxy);
        apply_option!(set_if_some, builder, request.local_address, local_address);
        apply_option!(
            set_if_some_tuple_inner,
            builder,
            request.local_addresses,
            local_addresses
        );

        #[cfg(any(
            target_os = "android",
            target_os = "fuchsia",
            target_os = "illumos",
            target_os = "ios",
            target_os = "linux",
            target_os = "macos",
            target_os = "solaris",
            target_os = "tvos",
            target_os = "visionos",
            target_os = "watchos",
        ))]
        apply_option!(set_if_some, builder, request.interface, interface);

        // Headers options.
        apply_option!(set_if_some_inner, builder, request.headers, headers);
        apply_option!(
            set_if_some_inner,
            builder,
            request.orig_headers,
            orig_headers
        );
        apply_option!(
            set_if_some,
            builder,
            request.default_headers,
            default_headers
        );

        // Cookies options.
        apply_option!(
            set_if_some_iter_inner_with_key,
            builder,
            request.cookies,
            header,
            COOKIE
        );
        apply_option!(
            set_if_some_inner,
            builder,
            request.cookie_provider,
            cookie_provider
        );

        // Authentication options.
        apply_option!(
            set_if_some_map_ref,
            builder,
            request.auth,
            auth,
            AsRef::<str>::as_ref
        );
        apply_option!(set_if_some, builder, request.bearer_auth, bearer_auth);
        apply_option!(set_if_some_tuple, builder, request.basic_auth, basic_auth);

        // Allow redirects options.
        apply_option!(set_if_some_inner, builder, request.redirect, redirect);

        // Compression options.
        apply_option!(set_if_some, builder, request.gzip, gzip);
        apply_option!(set_if_some, builder, request.brotli, brotli);
        apply_option!(set_if_some, builder, request.deflate, deflate);
        apply_option!(set_if_some, builder, request.zstd, zstd);

        // Query options.
        apply_option!(set_if_some_ref, builder, request.query, query);

        // Body options.
        apply_option!(set_if_some_ref, builder, request.form, form);
        apply_option!(set_if_some_ref, builder, request.json, json);
        apply_option!(
            set_if_some,
            builder,
            request.multipart.and_then(|form| form.form),
            multipart
        );
        apply_option!(
            set_if_some_map_try,
            builder,
            request.body,
            body,
            wreq::Body::try_from
        );
    }

    // Send request.
    builder
        .send()
        .await
        .and_then(|r| {
            if client.raise_for_status {
                r.error_for_status()
            } else {
                Ok(r)
            }
        })
        .map(Response::new)
        .map_err(Error::Library)
        .map_err(Into::into)
}

pub async fn execute_websocket_request<U>(
    client: Client,
    url: U,
    request: Option<WebSocketRequest>,
) -> PyResult<WebSocket>
where
    U: AsRef<str>,
{
    // Create the WebSocket builder.
    let mut builder = client.inner.load().client.websocket(url.as_ref());

    if let Some(mut request) = request {
        // Emulation options.
        apply_option!(set_if_some, builder, request.emulation, emulation);

        // Version options.
        apply_option!(
            set_if_some_map,
            builder,
            request.version,
            version,
            Version::into_ffi
        );

        // Subprotocols options.
        apply_option!(set_if_some, builder, request.protocols, protocols);

        // WebSocket config
        apply_option!(
            set_if_some,
            builder,
            request.read_buffer_size,
            read_buffer_size
        );
        apply_option!(
            set_if_some,
            builder,
            request.write_buffer_size,
            write_buffer_size
        );
        apply_option!(
            set_if_some,
            builder,
            request.max_write_buffer_size,
            max_write_buffer_size
        );
        apply_option!(set_if_some, builder, request.max_frame_size, max_frame_size);
        apply_option!(
            set_if_some,
            builder,
            request.max_message_size,
            max_message_size
        );
        apply_option!(
            set_if_some,
            builder,
            request.accept_unmasked_frames,
            accept_unmasked_frames
        );

        // Network options.
        apply_option!(set_if_some_inner, builder, request.proxy, proxy);
        apply_option!(set_if_some, builder, request.local_address, local_address);
        apply_option!(
            set_if_some_tuple_inner,
            builder,
            request.local_addresses,
            local_addresses
        );
        #[cfg(any(
            target_os = "android",
            target_os = "fuchsia",
            target_os = "illumos",
            target_os = "ios",
            target_os = "linux",
            target_os = "macos",
            target_os = "solaris",
            target_os = "tvos",
            target_os = "visionos",
            target_os = "watchos",
        ))]
        apply_option!(set_if_some, builder, request.interface, interface);

        // Headers options.
        apply_option!(set_if_some_inner, builder, request.headers, headers);
        apply_option!(
            set_if_some_inner,
            builder,
            request.orig_headers,
            orig_headers
        );
        apply_option!(
            set_if_some,
            builder,
            request.default_headers,
            default_headers
        );
        apply_option!(
            set_if_some_iter_inner_with_key,
            builder,
            request.cookies,
            header,
            COOKIE
        );

        // Authentication options.
        apply_option!(
            set_if_some_map_ref,
            builder,
            request.auth,
            auth,
            AsRef::<str>::as_ref
        );
        apply_option!(set_if_some, builder, request.bearer_auth, bearer_auth);
        apply_option!(set_if_some_tuple, builder, request.basic_auth, basic_auth);

        // Query options.
        apply_option!(set_if_some_ref, builder, request.query, query);
    }

    // Send the WebSocket request.
    builder
        .send()
        .and_then(WebSocket::new)
        .await
        .map_err(Error::Library)
        .map_err(Into::into)
}
