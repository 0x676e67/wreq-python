pub mod body;
pub mod nogil;
pub mod req;
pub mod resp;

mod param;
mod query;

use std::{
    net::{IpAddr, Ipv4Addr, Ipv6Addr},
    time::Duration,
};

use pyo3::{IntoPyObjectExt, coroutine::CancelHandle, prelude::*, pybacked::PyBackedStr};
use req::{Request, WebSocketRequest};
use tokio_util::sync::CancellationToken;
use wreq::tls::trust::CertStore;

use self::{
    nogil::NoGIL,
    req::{execute_request, execute_websocket_request},
    resp::{BlockingResponse, BlockingWebSocket, Response, WebSocket},
};
use crate::{
    cookie::Jar,
    dns::{DnsOptions, HickoryResolver, LookupIpStrategy},
    emulate::EmulationLike,
    error::Error,
    extractor::Extractor,
    header::{HeaderMap, OrigHeaderMap},
    http::Method,
    http1::Http1Options,
    http2::Http2Options,
    proxy::Proxy,
    redirect,
    tls::{Identity, KeyLog, TlsOptions, TlsVerify, TlsVersion},
};

/// A IP socket address.
#[derive(Clone, Copy, PartialEq, Eq)]
#[pyclass(eq, str, frozen, skip_from_py_object)]
pub struct SocketAddr(pub std::net::SocketAddr);

#[pymethods]
impl SocketAddr {
    /// Returns the IP address of the socket address.
    fn ip<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        self.0.ip().into_bound_py_any(py)
    }

    /// Returns the port number of the socket address.
    fn port(&self) -> u16 {
        self.0.port()
    }
}

impl_print_str!(Display, SocketAddr);

/// A builder for `Client`.
#[derive(Default)]
struct Builder {
    /// The Emulation settings for the client.
    emulation: Option<EmulationLike>,
    /// The user agent to use for the client.
    user_agent: Option<PyBackedStr>,
    /// The headers to use for the client.
    headers: Option<HeaderMap>,
    /// The original headers to use for the client.
    orig_headers: Option<OrigHeaderMap>,
    /// Whether to use referer.
    referer: Option<bool>,
    /// Whether to redirect policy.
    redirect: Option<redirect::Policy>,
    /// Whether to raise for status.
    raise_for_status: Option<bool>,

    // ========= Cookie options =========
    /// Whether to use cookie store.
    cookie_store: Option<bool>,
    /// Whether to use cookie store provider.
    cookie_provider: Option<Jar>,

    // ========= Timeout options =========
    /// The timeout to use for the client.
    timeout: Option<Duration>,
    /// The connect timeout to use for the client.
    connect_timeout: Option<Duration>,
    /// The read timeout to use for the client.
    read_timeout: Option<Duration>,

    // ========= TCP options =========
    /// Set that all sockets have `SO_KEEPALIVE` set with the supplied duration.
    tcp_keepalive: Option<Duration>,
    /// Set the interval between TCP keepalive probes.
    tcp_keepalive_interval: Option<Duration>,
    /// Set the number of retries for TCP keepalive.
    tcp_keepalive_retries: Option<u32>,
    /// Set an optional user timeout for TCP sockets.
    tcp_user_timeout: Option<Duration>,
    /// Set that all sockets have `NO_DELAY` set.
    tcp_nodelay: Option<bool>,
    /// Set that all sockets have `SO_REUSEADDR` set.
    tcp_reuse_address: Option<bool>,

    // ========= Connection pool options =========
    /// Set an optional timeout for idle sockets being kept-alive.
    pool_idle_timeout: Option<Duration>,
    /// Sets the maximum idle connection per host allowed in the pool.
    pool_max_idle_per_host: Option<usize>,
    /// Sets the maximum number of connections in the pool.
    pool_max_size: Option<usize>,

    // ========= Protocol options =========
    /// Whether to use the HTTP/1 protocol only.
    http1_only: Option<bool>,
    /// Whether to use the HTTP/2 protocol only.
    http2_only: Option<bool>,
    /// Whether to use HTTPS only.
    https_only: Option<bool>,
    /// Sets the HTTP/1 options for the client.
    http1_options: Option<Http1Options>,
    /// sets the HTTP/2 options for the client.
    http2_options: Option<Http2Options>,

    // ========= TLS options =========
    /// Whether to verify the SSL certificate or root certificate file path.
    tls_verify: Option<TlsVerify>,
    /// Whether to verify the hostname in the SSL certificate.
    tls_verify_hostname: Option<bool>,
    /// Represents a private key and X509 cert as a client certificate.
    tls_identity: Option<Identity>,
    /// Key logging policy for TLS session keys.
    tls_keylog: Option<KeyLog>,
    /// Add TLS information as `TlsInfo` extension to responses.
    tls_info: Option<bool>,
    /// The minimum TLS version to use for the client.
    tls_min_version: Option<TlsVersion>,
    /// The maximum TLS version to use for the client.
    tls_max_version: Option<TlsVersion>,
    /// Sets the TLS options for the client.
    tls_options: Option<TlsOptions>,

    // ========= Network options =========
    /// Whether to disable the proxy for the client.
    no_proxy: Option<bool>,
    /// The proxies to use for the client.
    proxies: Option<Vec<Proxy>>,
    /// Bind to a local IP Address.
    local_address: Option<IpAddr>,
    /// Bind to local IP Addresses (IPv4, IPv6).
    local_addresses: Option<Extractor<(Option<Ipv4Addr>, Option<Ipv6Addr>)>>,
    /// Bind to an interface by `SO_BINDTODEVICE`.
    interface: Option<String>,

    // ========= DNS options =========
    dns_options: Option<DnsOptions>,

    // ========= Compression options =========
    /// Sets gzip as an accepted encoding.
    gzip: Option<bool>,
    /// Sets brotli as an accepted encoding.
    brotli: Option<bool>,
    /// Sets deflate as an accepted encoding.
    deflate: Option<bool>,
    /// Sets zstd as an accepted encoding.
    zstd: Option<bool>,
}

impl FromPyObject<'_, '_> for Builder {
    type Error = PyErr;

    fn extract(ob: Borrowed<PyAny>) -> PyResult<Self> {
        let mut builder = Self::default();
        extract_option!(ob, builder, emulation);
        extract_option!(ob, builder, user_agent);
        extract_option!(ob, builder, headers);
        extract_option!(ob, builder, orig_headers);
        extract_option!(ob, builder, referer);
        extract_option!(ob, builder, redirect);
        extract_option!(ob, builder, raise_for_status);

        extract_option!(ob, builder, cookie_store);
        extract_option!(ob, builder, cookie_provider);

        extract_option!(ob, builder, timeout);
        extract_option!(ob, builder, connect_timeout);
        extract_option!(ob, builder, read_timeout);

        extract_option!(ob, builder, tcp_keepalive);
        extract_option!(ob, builder, tcp_keepalive_interval);
        extract_option!(ob, builder, tcp_keepalive_retries);
        extract_option!(ob, builder, tcp_user_timeout);
        extract_option!(ob, builder, tcp_nodelay);
        extract_option!(ob, builder, tcp_reuse_address);

        extract_option!(ob, builder, pool_idle_timeout);
        extract_option!(ob, builder, pool_max_idle_per_host);
        extract_option!(ob, builder, pool_max_size);

        extract_option!(ob, builder, no_proxy);
        extract_option!(ob, builder, proxies);
        extract_option!(ob, builder, local_address);
        extract_option!(ob, builder, local_addresses);
        extract_option!(ob, builder, interface);

        extract_option!(ob, builder, https_only);
        extract_option!(ob, builder, http1_only);
        extract_option!(ob, builder, http2_only);
        extract_option!(ob, builder, http1_options);
        extract_option!(ob, builder, http2_options);

        extract_option!(ob, builder, tls_verify);
        extract_option!(ob, builder, tls_verify_hostname);
        extract_option!(ob, builder, tls_identity);
        extract_option!(ob, builder, tls_keylog);
        extract_option!(ob, builder, tls_info);
        extract_option!(ob, builder, tls_min_version);
        extract_option!(ob, builder, tls_max_version);
        extract_option!(ob, builder, tls_options);

        extract_option!(ob, builder, dns_options);

        extract_option!(ob, builder, gzip);
        extract_option!(ob, builder, brotli);
        extract_option!(ob, builder, deflate);
        extract_option!(ob, builder, zstd);
        Ok(builder)
    }
}

/// A client for making HTTP requests.
#[derive(Default, Clone)]
#[pyclass(subclass, frozen, skip_from_py_object)]
pub struct Client {
    inner: wreq::Client,
    cancel: CancellationToken,
    raise_for_status: bool,

    /// Get the cookie jar of the client.
    #[pyo3(get)]
    cookie_jar: Option<Jar>,
}

/// A blocking client for making HTTP requests.
#[derive(Default)]
#[pyclass(name = "Client", subclass, frozen, skip_from_py_object)]
pub struct BlockingClient(Client);

// ====== Client =====

#[pymethods]
impl Client {
    /// Creates a new Client instance.
    #[new]
    #[pyo3(signature = (**kwds))]
    fn new(py: Python, kwds: Option<Builder>) -> PyResult<Client> {
        py.detach(|| {
            // Create the client builder.
            let mut builder = wreq::Client::builder();
            let mut cookie_jar: Option<Jar> = None;
            let mut raise_for_status = false;

            if let Some(mut config) = kwds {
                // Emulation options.
                apply_option!(set_if_some, builder, config.emulation, emulation);

                // User agent options.
                apply_option!(
                    set_if_some_map_ref,
                    builder,
                    config.user_agent,
                    user_agent,
                    AsRef::<str>::as_ref
                );

                // Default headers options.
                apply_option!(set_if_some_inner, builder, config.headers, default_headers);
                apply_option!(
                    set_if_some_inner,
                    builder,
                    config.orig_headers,
                    orig_headers
                );

                // Allow redirects options.
                apply_option!(set_if_some, builder, config.referer, referer);
                apply_option!(set_if_some_inner, builder, config.redirect, redirect);

                // Cookie options.
                if let Some(jar) = config.cookie_provider.take() {
                    builder = builder.cookie_provider(jar.clone().0);
                    cookie_jar = Some(jar);
                } else if config.cookie_store.unwrap_or_default() {
                    // `cookie_store` is true and no provider was given, so create a default jar to
                    // be accessed later through the client interface.
                    let jar = Jar::new();
                    builder = builder.cookie_provider(jar.clone().0);
                    cookie_jar = Some(jar);
                }

                // TCP options.
                apply_option!(set_if_some, builder, config.tcp_keepalive, tcp_keepalive);
                apply_option!(
                    set_if_some,
                    builder,
                    config.tcp_keepalive_interval,
                    tcp_keepalive_interval
                );
                apply_option!(
                    set_if_some,
                    builder,
                    config.tcp_keepalive_retries,
                    tcp_keepalive_retries
                );
                #[cfg(any(target_os = "android", target_os = "fuchsia", target_os = "linux"))]
                apply_option!(
                    set_if_some,
                    builder,
                    config.tcp_user_timeout,
                    tcp_user_timeout
                );
                apply_option!(set_if_some, builder, config.tcp_nodelay, tcp_nodelay);
                apply_option!(
                    set_if_some,
                    builder,
                    config.tcp_reuse_address,
                    tcp_reuse_address
                );

                // Timeout options.
                apply_option!(set_if_some, builder, config.timeout, timeout);
                apply_option!(
                    set_if_some,
                    builder,
                    config.connect_timeout,
                    connect_timeout
                );
                apply_option!(set_if_some, builder, config.read_timeout, read_timeout);

                // Pool options.
                apply_option!(
                    set_if_some,
                    builder,
                    config.pool_idle_timeout,
                    pool_idle_timeout
                );
                apply_option!(
                    set_if_some,
                    builder,
                    config.pool_max_idle_per_host,
                    pool_max_idle_per_host
                );
                apply_option!(set_if_some, builder, config.pool_max_size, pool_max_size);

                // Protocol options.
                apply_option!(set_if_true, builder, config.http1_only, http1_only, false);
                apply_option!(set_if_true, builder, config.http2_only, http2_only, false);
                apply_option!(set_if_some, builder, config.https_only, https_only);
                apply_option!(
                    set_if_some_inner,
                    builder,
                    config.http1_options,
                    http1_options
                );
                apply_option!(
                    set_if_some_inner,
                    builder,
                    config.http2_options,
                    http2_options
                );

                // TLS options.
                apply_option!(
                    set_if_some_map,
                    builder,
                    config.tls_min_version,
                    tls_min_version,
                    TlsVersion::into_ffi
                );
                apply_option!(
                    set_if_some_map,
                    builder,
                    config.tls_max_version,
                    tls_max_version,
                    TlsVersion::into_ffi
                );
                apply_option!(set_if_some, builder, config.tls_info, tls_info);
                apply_option!(
                    set_if_some,
                    builder,
                    config.tls_verify_hostname,
                    tls_verify_hostname
                );
                apply_option!(
                    set_if_some_inner,
                    builder,
                    config.tls_identity,
                    tls_identity
                );
                apply_option!(set_if_some_inner, builder, config.tls_keylog, tls_keylog);
                apply_option!(set_if_some_inner, builder, config.tls_options, tls_options);
                if let Some(verify) = config.tls_verify.take() {
                    builder = match verify {
                        TlsVerify::Verification(verify) => builder.tls_cert_verification(verify),
                        TlsVerify::CertificatePath(path_buf) => {
                            let pem_data = std::fs::read(path_buf)?;
                            let store =
                                CertStore::from_pem_stack(pem_data).map_err(Error::Library)?;
                            builder.tls_cert_store(store)
                        }
                        TlsVerify::CertificateStore(cert_store) => {
                            builder.tls_cert_store(cert_store.0)
                        }
                    }
                }

                // Network options.
                apply_option!(set_if_some_iter_inner, builder, config.proxies, proxy);
                apply_option!(set_if_true, builder, config.no_proxy, no_proxy, false);
                apply_option!(set_if_some, builder, config.local_address, local_address);
                apply_option!(
                    set_if_some_tuple_inner,
                    builder,
                    config.local_addresses,
                    local_addresses
                );
                #[cfg(any(
                    target_os = "android",
                    target_os = "fuchsia",
                    target_os = "linux",
                    target_os = "ios",
                    target_os = "visionos",
                    target_os = "macos",
                    target_os = "tvos",
                    target_os = "watchos"
                ))]
                apply_option!(set_if_some, builder, config.interface, interface);

                // DNS options.
                if let Some(opts) = config.dns_options.take() {
                    for (domain, addrs) in opts.resolve_to_addrs {
                        builder = builder.resolve_to_addrs(domain.as_ref().to_string(), addrs);
                    }

                    if !opts.system_dns {
                        builder =
                            builder.dns_resolver(HickoryResolver::new(opts.lookup_ip_strategy));
                    }
                } else {
                    builder =
                        builder.dns_resolver(HickoryResolver::new(LookupIpStrategy::default()));
                };

                // Compression options.
                apply_option!(set_if_some, builder, config.gzip, gzip);
                apply_option!(set_if_some, builder, config.brotli, brotli);
                apply_option!(set_if_some, builder, config.deflate, deflate);
                apply_option!(set_if_some, builder, config.zstd, zstd);

                raise_for_status = config.raise_for_status.unwrap_or(false);
            }

            builder
                .build()
                .map(|inner| Client {
                    inner,
                    cancel: CancellationToken::new(),
                    cookie_jar,
                    raise_for_status,
                })
                .map_err(Error::Library)
                .map_err(Into::into)
        })
    }

    /// Close the client, preventing any new requests.
    #[inline]
    pub fn close(&self) {
        self.cancel.cancel();
    }

    /// Make a GET request to the given URL.
    #[inline(always)]
    #[pyo3(signature = (url, **kwds))]
    pub async fn get(
        &self,
        #[pyo3(cancel_handle)] cancel: CancelHandle,
        url: PyBackedStr,
        kwds: Option<Request>,
    ) -> PyResult<Response> {
        self.request(cancel, Method::GET, url, kwds).await
    }

    /// Make a HEAD request to the given URL.
    #[inline(always)]
    #[pyo3(signature = (url, **kwds))]
    pub async fn head(
        &self,
        #[pyo3(cancel_handle)] cancel: CancelHandle,
        url: PyBackedStr,
        kwds: Option<Request>,
    ) -> PyResult<Response> {
        self.request(cancel, Method::HEAD, url, kwds).await
    }

    /// Make a POST request to the given URL.
    #[inline(always)]
    #[pyo3(signature = (url, **kwds))]
    pub async fn post(
        &self,
        #[pyo3(cancel_handle)] cancel: CancelHandle,
        url: PyBackedStr,
        kwds: Option<Request>,
    ) -> PyResult<Response> {
        self.request(cancel, Method::POST, url, kwds).await
    }

    /// Make a PUT request to the given URL.
    #[inline(always)]
    #[pyo3(signature = (url, **kwds))]
    pub async fn put(
        &self,
        #[pyo3(cancel_handle)] cancel: CancelHandle,
        url: PyBackedStr,
        kwds: Option<Request>,
    ) -> PyResult<Response> {
        self.request(cancel, Method::PUT, url, kwds).await
    }

    /// Make a DELETE request to the given URL.
    #[inline(always)]
    #[pyo3(signature = (url, **kwds))]
    pub async fn delete(
        &self,
        #[pyo3(cancel_handle)] cancel: CancelHandle,
        url: PyBackedStr,
        kwds: Option<Request>,
    ) -> PyResult<Response> {
        self.request(cancel, Method::DELETE, url, kwds).await
    }

    /// Make a PATCH request to the given URL.
    #[inline(always)]
    #[pyo3(signature = (url, **kwds))]
    pub async fn patch(
        &self,
        #[pyo3(cancel_handle)] cancel: CancelHandle,
        url: PyBackedStr,
        kwds: Option<Request>,
    ) -> PyResult<Response> {
        self.request(cancel, Method::PATCH, url, kwds).await
    }

    /// Make a OPTIONS request to the given URL.
    #[inline(always)]
    #[pyo3(signature = (url, **kwds))]
    pub async fn options(
        &self,
        #[pyo3(cancel_handle)] cancel: CancelHandle,
        url: PyBackedStr,
        kwds: Option<Request>,
    ) -> PyResult<Response> {
        self.request(cancel, Method::OPTIONS, url, kwds).await
    }

    /// Make a TRACE request to the given URL.
    #[inline(always)]
    #[pyo3(signature = (url, **kwds))]
    pub async fn trace(
        &self,
        #[pyo3(cancel_handle)] cancel: CancelHandle,
        url: PyBackedStr,
        kwds: Option<Request>,
    ) -> PyResult<Response> {
        self.request(cancel, Method::TRACE, url, kwds).await
    }

    /// Make a request with the given method and URL.
    #[inline]
    #[pyo3(signature = (method, url, **kwds))]
    pub async fn request(
        &self,
        #[pyo3(cancel_handle)] cancel: CancelHandle,
        method: Method,
        url: PyBackedStr,
        kwds: Option<Request>,
    ) -> PyResult<Response> {
        NoGIL::new_with_token(
            execute_request(self.clone(), method, url, kwds),
            cancel,
            self.cancel.clone(),
        )
        .await
    }

    /// Make a WebSocket request to the given URL.
    #[inline]
    #[pyo3(signature = (url, **kwds))]
    pub async fn websocket(
        &self,
        #[pyo3(cancel_handle)] cancel: CancelHandle,
        url: PyBackedStr,
        kwds: Option<WebSocketRequest>,
    ) -> PyResult<WebSocket> {
        NoGIL::new_with_token(
            execute_websocket_request(self.clone(), url, kwds),
            cancel,
            self.cancel.clone(),
        )
        .await
    }
}

#[pymethods]
impl Client {
    #[inline]
    async fn __aenter__(slf: Py<Self>) -> PyResult<Py<Self>> {
        Ok(slf)
    }

    #[inline]
    async fn __aexit__(&self, _exc_type: Py<PyAny>, _exc_val: Py<PyAny>, _traceback: Py<PyAny>) {
        self.close();
    }
}

// ===== impl BlockingClient =====

#[pymethods]
impl BlockingClient {
    /// Creates a new blocking Client instance.
    #[new]
    #[inline]
    #[pyo3(signature = (**kwds))]
    fn new(py: Python, kwds: Option<Builder>) -> PyResult<BlockingClient> {
        Client::new(py, kwds).map(BlockingClient)
    }

    /// Get the cookie jar of the client.
    #[inline]
    #[getter]
    pub fn cookie_jar(&self) -> Option<Jar> {
        self.0.cookie_jar.clone()
    }

    /// Close the client, preventing any new requests.
    #[inline]
    pub fn close(&self) {
        self.0.close();
    }

    /// Make a GET request to the specified URL.
    #[inline(always)]
    #[pyo3(signature = (url, **kwds))]
    pub fn get(
        &self,
        py: Python<'_>,
        url: PyBackedStr,
        kwds: Option<Request>,
    ) -> PyResult<BlockingResponse> {
        self.request(py, Method::GET, url, kwds)
    }

    /// Make a POST request to the specified URL.
    #[inline(always)]
    #[pyo3(signature = (url, **kwds))]
    pub fn post(
        &self,
        py: Python<'_>,
        url: PyBackedStr,
        kwds: Option<Request>,
    ) -> PyResult<BlockingResponse> {
        self.request(py, Method::POST, url, kwds)
    }

    /// Make a PUT request to the specified URL.
    #[inline(always)]
    #[pyo3(signature = (url, **kwds))]
    pub fn put(
        &self,
        py: Python<'_>,
        url: PyBackedStr,
        kwds: Option<Request>,
    ) -> PyResult<BlockingResponse> {
        self.request(py, Method::PUT, url, kwds)
    }

    /// Make a PATCH request to the specified URL.
    #[inline(always)]
    #[pyo3(signature = (url, **kwds))]
    pub fn patch(
        &self,
        py: Python<'_>,
        url: PyBackedStr,
        kwds: Option<Request>,
    ) -> PyResult<BlockingResponse> {
        self.request(py, Method::PATCH, url, kwds)
    }

    /// Make a DELETE request to the specified URL.
    #[inline(always)]
    #[pyo3(signature = (url, **kwds))]
    pub fn delete(
        &self,
        py: Python<'_>,
        url: PyBackedStr,
        kwds: Option<Request>,
    ) -> PyResult<BlockingResponse> {
        self.request(py, Method::DELETE, url, kwds)
    }

    /// Make a HEAD request to the specified URL.
    #[inline(always)]
    #[pyo3(signature = (url, **kwds))]
    pub fn head(
        &self,
        py: Python<'_>,
        url: PyBackedStr,
        kwds: Option<Request>,
    ) -> PyResult<BlockingResponse> {
        self.request(py, Method::HEAD, url, kwds)
    }

    /// Make a OPTIONS request to the specified URL.
    #[inline(always)]
    #[pyo3(signature = (url, **kwds))]
    pub fn options(
        &self,
        py: Python<'_>,
        url: PyBackedStr,
        kwds: Option<Request>,
    ) -> PyResult<BlockingResponse> {
        self.request(py, Method::OPTIONS, url, kwds)
    }

    /// Make a TRACE request to the specified URL.
    #[inline(always)]
    #[pyo3(signature = (url, **kwds))]
    pub fn trace(
        &self,
        py: Python<'_>,
        url: PyBackedStr,
        kwds: Option<Request>,
    ) -> PyResult<BlockingResponse> {
        self.request(py, Method::TRACE, url, kwds)
    }

    /// Make a rqeuest with the specified method and URL.
    #[pyo3(signature = (method, url, **kwds))]
    pub fn request(
        &self,
        py: Python,
        method: Method,
        url: PyBackedStr,
        kwds: Option<Request>,
    ) -> PyResult<BlockingResponse> {
        py.detach(|| {
            pyo3_async_runtimes::tokio::get_runtime()
                .block_on(execute_request(self.0.clone(), method, url, kwds))
                .map(Into::into)
        })
    }

    /// Make a WebSocket request to the specified URL.
    #[pyo3(signature = (url, **kwds))]
    pub fn websocket(
        &self,
        py: Python,
        url: PyBackedStr,
        kwds: Option<WebSocketRequest>,
    ) -> PyResult<BlockingWebSocket> {
        py.detach(|| {
            pyo3_async_runtimes::tokio::get_runtime()
                .block_on(execute_websocket_request(self.0.clone(), url, kwds))
                .map(Into::into)
        })
    }
}

#[pymethods]
impl BlockingClient {
    #[inline]
    fn __enter__(slf: PyRef<Self>) -> PyRef<Self> {
        slf
    }

    #[inline]
    fn __exit__<'py>(
        &self,
        _py: Python<'py>,
        _exc_type: &Bound<'py, PyAny>,
        _exc_value: &Bound<'py, PyAny>,
        _traceback: &Bound<'py, PyAny>,
    ) {
        self.close();
    }
}
