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

use pyo3::{IntoPyObjectExt, prelude::*, pybacked::PyBackedStr, types::PyDict};
use req::{Request, WebSocketRequest};
use tokio_util::sync::CancellationToken;
use wreq::tls::trust::CertStore;

use self::{
    req::{execute_request, execute_websocket_request},
    resp::{BlockingResponse, BlockingWebSocket, Response, WebSocket},
};
use crate::{
    aio::{self, Coroutine},
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
    redirect, runtime,
    tls::{Identity, KeyLog, TlsOptions, TlsVerify, TlsVersion},
};

/// An IP socket address.
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
    runtime: Option<runtime::Runtime>,
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
        extract_option!(ob, builder, runtime);
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
#[derive(Clone)]
#[pyclass(subclass, frozen, skip_from_py_object)]
pub struct Client {
    inner: wreq::Client,
    /// Runs this client's requests and response reads, keeping the runtime alive.
    runtime: runtime::Runtime,
    /// Cancelled by `close()`; pending and new requests then fail with `CancelledError`.
    cancel: CancellationToken,
    /// Turn error statuses into exceptions for every request.
    raise_for_status: bool,

    /// Get the cookie jar of the client.
    #[pyo3(get)]
    cookie_jar: Option<Jar>,
}

/// A blocking client for making HTTP requests.
#[derive(Default)]
#[pyclass(name = "Client", subclass, frozen, skip_from_py_object)]
pub struct BlockingClient(Client);

// ===== impl Client =====

impl Client {
    /// Return a coroutine named `qualname` that sends the request when awaited.
    fn send<'py>(
        &self,
        py: Python<'py>,
        qualname: &'static str,
        method: Method,
        url: PyBackedStr,
        kwds: Option<Bound<'py, PyDict>>,
    ) -> PyResult<Bound<'py, Coroutine>> {
        let kwds = kwds.map(Bound::unbind);
        aio::managed(py, qualname, self.clone().execute(method, url, kwds))
    }

    /// Send a request on the client's runtime, extracting options on first await so an
    /// async generator body binds to the running loop.
    pub(crate) async fn execute(
        self,
        method: Method,
        url: PyBackedStr,
        kwds: Option<Py<PyDict>>,
    ) -> PyResult<Response> {
        let kwds = Python::attach(|py| kwds.map(|kwds| kwds.bind(py).extract()).transpose())?;
        aio::run(
            self.runtime.clone(),
            execute_request(self, method, url, kwds),
        )
        .await
    }

    /// Open a WebSocket on the client's runtime, extracting options on first await.
    pub(crate) async fn connect(
        self,
        url: PyBackedStr,
        kwds: Option<Py<PyDict>>,
    ) -> PyResult<WebSocket> {
        let kwds = Python::attach(|py| kwds.map(|kwds| kwds.bind(py).extract()).transpose())?;
        aio::run(
            self.runtime.clone(),
            execute_websocket_request(self, url, kwds),
        )
        .await
    }
}

impl Default for Client {
    fn default() -> Self {
        let runtime = runtime::get();
        Self {
            inner: wreq::Client::default(),
            runtime: runtime.clone(),
            cancel: CancellationToken::new(),
            raise_for_status: false,
            cookie_jar: None,
        }
    }
}

#[pymethods]
impl Client {
    /// Creates a new Client instance.
    #[new]
    #[pyo3(signature = (**kwds))]
    fn new(py: Python, kwds: Option<Builder>) -> PyResult<Client> {
        py.detach(|| {
            let runtime = match kwds.as_ref().and_then(|config| config.runtime.as_ref()) {
                Some(runtime) => runtime.select()?,
                None => runtime::get().clone(),
            };
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
                            builder.dns_resolver(HickoryResolver::new(opts.lookup_ip_strategy)?);
                    }
                } else {
                    builder =
                        builder.dns_resolver(HickoryResolver::new(LookupIpStrategy::default())?);
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
                    runtime,
                    cancel: CancellationToken::new(),
                    cookie_jar,
                    raise_for_status,
                })
                .map_err(Error::Library)
                .map_err(Into::into)
        })
    }

    /// Cancel pending requests and reject new ones with asyncio.CancelledError.
    /// Existing responses, WebSockets and the shared runtime remain usable.
    pub fn close(&self) {
        self.cancel.cancel();
    }

    /// The runtime used by this client and its responses.
    #[getter]
    pub fn runtime(&self) -> runtime::Runtime {
        self.runtime.clone()
    }

    /// Make a GET request to the given URL.
    #[pyo3(signature = (url, **kwds))]
    pub fn get<'py>(
        &self,
        py: Python<'py>,
        url: PyBackedStr,
        kwds: Option<Bound<'py, PyDict>>,
    ) -> PyResult<Bound<'py, Coroutine>> {
        self.send(py, "Client.get", Method::GET, url, kwds)
    }

    /// Make a HEAD request to the given URL.
    #[pyo3(signature = (url, **kwds))]
    pub fn head<'py>(
        &self,
        py: Python<'py>,
        url: PyBackedStr,
        kwds: Option<Bound<'py, PyDict>>,
    ) -> PyResult<Bound<'py, Coroutine>> {
        self.send(py, "Client.head", Method::HEAD, url, kwds)
    }

    /// Make a POST request to the given URL.
    #[pyo3(signature = (url, **kwds))]
    pub fn post<'py>(
        &self,
        py: Python<'py>,
        url: PyBackedStr,
        kwds: Option<Bound<'py, PyDict>>,
    ) -> PyResult<Bound<'py, Coroutine>> {
        self.send(py, "Client.post", Method::POST, url, kwds)
    }

    /// Make a PUT request to the given URL.
    #[pyo3(signature = (url, **kwds))]
    pub fn put<'py>(
        &self,
        py: Python<'py>,
        url: PyBackedStr,
        kwds: Option<Bound<'py, PyDict>>,
    ) -> PyResult<Bound<'py, Coroutine>> {
        self.send(py, "Client.put", Method::PUT, url, kwds)
    }

    /// Make a DELETE request to the given URL.
    #[pyo3(signature = (url, **kwds))]
    pub fn delete<'py>(
        &self,
        py: Python<'py>,
        url: PyBackedStr,
        kwds: Option<Bound<'py, PyDict>>,
    ) -> PyResult<Bound<'py, Coroutine>> {
        self.send(py, "Client.delete", Method::DELETE, url, kwds)
    }

    /// Make a PATCH request to the given URL.
    #[pyo3(signature = (url, **kwds))]
    pub fn patch<'py>(
        &self,
        py: Python<'py>,
        url: PyBackedStr,
        kwds: Option<Bound<'py, PyDict>>,
    ) -> PyResult<Bound<'py, Coroutine>> {
        self.send(py, "Client.patch", Method::PATCH, url, kwds)
    }

    /// Make a OPTIONS request to the given URL.
    #[pyo3(signature = (url, **kwds))]
    pub fn options<'py>(
        &self,
        py: Python<'py>,
        url: PyBackedStr,
        kwds: Option<Bound<'py, PyDict>>,
    ) -> PyResult<Bound<'py, Coroutine>> {
        self.send(py, "Client.options", Method::OPTIONS, url, kwds)
    }

    /// Make a TRACE request to the given URL.
    #[pyo3(signature = (url, **kwds))]
    pub fn trace<'py>(
        &self,
        py: Python<'py>,
        url: PyBackedStr,
        kwds: Option<Bound<'py, PyDict>>,
    ) -> PyResult<Bound<'py, Coroutine>> {
        self.send(py, "Client.trace", Method::TRACE, url, kwds)
    }

    /// Make a request with the given method and URL.
    #[pyo3(signature = (method, url, **kwds))]
    pub fn request<'py>(
        &self,
        py: Python<'py>,
        method: Method,
        url: PyBackedStr,
        kwds: Option<Bound<'py, PyDict>>,
    ) -> PyResult<Bound<'py, Coroutine>> {
        self.send(py, "Client.request", method, url, kwds)
    }

    /// Make a WebSocket request to the given URL.
    #[pyo3(signature = (url, **kwds))]
    pub fn websocket<'py>(
        &self,
        py: Python<'py>,
        url: PyBackedStr,
        kwds: Option<Bound<'py, PyDict>>,
    ) -> PyResult<Bound<'py, Coroutine>> {
        let kwds = kwds.map(Bound::unbind);
        aio::managed(py, "Client.websocket", self.clone().connect(url, kwds))
    }
}

#[pymethods]
impl Client {
    fn __aenter__(slf: Bound<'_, Self>) -> PyResult<Bound<'_, Coroutine>> {
        aio::ready("Client.__aenter__", slf)
    }

    /// Close the client like `close()`: cancel pending requests and reject new ones.
    fn __aexit__<'py>(
        &self,
        py: Python<'py>,
        _exc_type: Py<PyAny>,
        _exc_val: Py<PyAny>,
        _traceback: Py<PyAny>,
    ) -> PyResult<Bound<'py, Coroutine>> {
        let cancel = self.cancel.clone();
        aio::local(py, "Client.__aexit__", async move {
            cancel.cancel();
            Ok(())
        })
    }
}

// ===== impl BlockingClient =====

#[pymethods]
impl BlockingClient {
    /// The runtime used by this client and its responses.
    #[getter]
    pub fn runtime(&self) -> runtime::Runtime {
        self.0.runtime()
    }

    /// Creates a new blocking Client instance.
    #[new]
    #[pyo3(signature = (**kwds))]
    fn new(py: Python, kwds: Option<Builder>) -> PyResult<BlockingClient> {
        Client::new(py, kwds).map(BlockingClient)
    }

    /// Get the cookie jar of the client.
    #[getter]
    pub fn cookie_jar(&self) -> Option<Jar> {
        self.0.cookie_jar.clone()
    }

    /// Cancel pending requests and reject new ones with asyncio.CancelledError.
    /// Existing responses, WebSockets and the shared runtime remain usable.
    pub fn close(&self) {
        self.0.close();
    }

    /// Make a GET request to the specified URL.
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
    #[pyo3(signature = (url, **kwds))]
    pub fn trace(
        &self,
        py: Python<'_>,
        url: PyBackedStr,
        kwds: Option<Request>,
    ) -> PyResult<BlockingResponse> {
        self.request(py, Method::TRACE, url, kwds)
    }

    /// Make a request with the specified method and URL.
    #[pyo3(signature = (method, url, **kwds))]
    pub fn request(
        &self,
        py: Python,
        method: Method,
        url: PyBackedStr,
        kwds: Option<Request>,
    ) -> PyResult<BlockingResponse> {
        py.detach(|| {
            nogil::block_on(
                &self.0.runtime,
                execute_request(self.0.clone(), method, url, kwds),
            )
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
            nogil::block_on(
                &self.0.runtime,
                execute_websocket_request(self.0.clone(), url, kwds),
            )
            .map(Into::into)
        })
    }
}

#[pymethods]
impl BlockingClient {
    fn __enter__(slf: PyRef<Self>) -> PyRef<Self> {
        slf
    }

    /// Close the client like `close()`: cancel pending requests and reject new ones.
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
