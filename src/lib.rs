#![deny(unused)]
#![deny(unsafe_code)]
#![cfg_attr(test, deny(warnings))]
#![cfg_attr(not(test), warn(unused_crate_dependencies))]

#[macro_use]
mod macros;
mod buffer;
mod client;
mod cookie;
mod dns;
mod emulate;
mod error;
mod extractor;
mod header;
mod http;
mod http1;
mod http2;
mod proxy;
mod redirect;
mod tls;

use client::{
    BlockingClient, Client, SocketAddr,
    body::{
        Streamer,
        multipart::{Multipart, Part},
    },
    req::WebSocketRequest,
    resp::{BlockingResponse, BlockingWebSocket, Message, Response, WebSocket},
};
use cookie::{Cookie, Jar, SameSite};
use dns::{DnsOptions, LookupIpStrategy};
use emulate::{Emulation, Platform, Profile};
use error::*;
use header::{HeaderMap, OrigHeaderMap};
use http::{Method, StatusCode, Version};
use http1::Http1Options;
use http2::{
    Http2Options, Priorities, Priority, PseudoId, PseudoOrder, SettingId, SettingsOrder,
    StreamDependency, StreamId,
};
#[cfg(feature = "mimalloc")]
use mimalloc as _;
use proxy::Proxy;
use pyo3::{
    coroutine::CancelHandle, intern, prelude::*, pybacked::PyBackedStr, types::PyDict,
    wrap_pymodule,
};
#[cfg(feature = "jemalloc")]
use tikv_jemallocator as _;
use tls::{
    AlpnProtocol, AlpsProtocol, CertStore, CertificateCompressionAlgorithm, ExtensionType,
    Identity, KeyLog, KeyShare, TlsInfo, TlsOptions, TlsVersion,
};

#[cfg(all(feature = "jemalloc", feature = "mimalloc"))]
compile_error!("features 'jemalloc' and 'mimalloc' are mutually exclusive");

#[cfg(all(feature = "jemalloc", not(feature = "mimalloc")))]
#[global_allocator]
static GLOBAL: tikv_jemallocator::Jemalloc = tikv_jemallocator::Jemalloc;

#[cfg(all(feature = "mimalloc", not(feature = "jemalloc")))]
#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;

mod r#async {
    use pyo3::{coroutine::CancelHandle, prelude::*, pybacked::PyBackedStr};

    use crate::{
        client::{
            Client,
            req::{Request, WebSocketRequest},
            resp::{Response, WebSocket},
        },
        http::Method,
    };

    /// Make a GET request with the given parameters.
    #[inline]
    #[pyfunction]
    #[pyo3(signature = (url, **kwds))]
    pub async fn get(
        #[pyo3(cancel_handle)] cancel: CancelHandle,
        url: PyBackedStr,
        kwds: Option<Request>,
    ) -> PyResult<Response> {
        request(cancel, Method::GET, url, kwds).await
    }

    /// Make a POST request with the given parameters.
    #[inline]
    #[pyfunction]
    #[pyo3(signature = (url, **kwds))]
    pub async fn post(
        #[pyo3(cancel_handle)] cancel: CancelHandle,
        url: PyBackedStr,
        kwds: Option<Request>,
    ) -> PyResult<Response> {
        request(cancel, Method::POST, url, kwds).await
    }

    /// Make a PUT request with the given parameters.
    #[inline]
    #[pyfunction]
    #[pyo3(signature = (url, **kwds))]
    pub async fn put(
        #[pyo3(cancel_handle)] cancel: CancelHandle,
        url: PyBackedStr,
        kwds: Option<Request>,
    ) -> PyResult<Response> {
        request(cancel, Method::PUT, url, kwds).await
    }

    /// Make a PATCH request with the given parameters.
    #[inline]
    #[pyfunction]
    #[pyo3(signature = (url, **kwds))]
    pub async fn patch(
        #[pyo3(cancel_handle)] cancel: CancelHandle,
        url: PyBackedStr,
        kwds: Option<Request>,
    ) -> PyResult<Response> {
        request(cancel, Method::PATCH, url, kwds).await
    }

    /// Make a DELETE request with the given parameters.
    #[inline]
    #[pyfunction]
    #[pyo3(signature = (url, **kwds))]
    pub async fn delete(
        #[pyo3(cancel_handle)] cancel: CancelHandle,
        url: PyBackedStr,
        kwds: Option<Request>,
    ) -> PyResult<Response> {
        request(cancel, Method::DELETE, url, kwds).await
    }

    /// Make a HEAD request with the given parameters.
    #[inline]
    #[pyfunction]
    #[pyo3(signature = (url, **kwds))]
    pub async fn head(
        #[pyo3(cancel_handle)] cancel: CancelHandle,
        url: PyBackedStr,
        kwds: Option<Request>,
    ) -> PyResult<Response> {
        request(cancel, Method::HEAD, url, kwds).await
    }

    /// Make a OPTIONS request with the given parameters.
    #[inline]
    #[pyfunction]
    #[pyo3(signature = (url, **kwds))]
    pub async fn options(
        #[pyo3(cancel_handle)] cancel: CancelHandle,
        url: PyBackedStr,
        kwds: Option<Request>,
    ) -> PyResult<Response> {
        request(cancel, Method::OPTIONS, url, kwds).await
    }

    /// Make a TRACE request with the given parameters.
    #[inline]
    #[pyfunction]
    #[pyo3(signature = (url, **kwds))]
    pub async fn trace(
        #[pyo3(cancel_handle)] cancel: CancelHandle,
        url: PyBackedStr,
        kwds: Option<Request>,
    ) -> PyResult<Response> {
        request(cancel, Method::TRACE, url, kwds).await
    }

    /// Make a request with the given parameters.
    #[inline]
    #[pyfunction]
    #[pyo3(signature = (method, url, **kwds))]
    pub async fn request(
        #[pyo3(cancel_handle)] cancel: CancelHandle,
        method: Method,
        url: PyBackedStr,
        kwds: Option<Request>,
    ) -> PyResult<Response> {
        Client::default().request(cancel, method, url, kwds).await
    }

    /// Make a WebSocket connection with the given parameters.
    #[inline]
    #[pyfunction]
    #[pyo3(signature = (url, **kwds))]
    pub async fn websocket(
        #[pyo3(cancel_handle)] cancel: CancelHandle,
        url: PyBackedStr,
        kwds: Option<WebSocketRequest>,
    ) -> PyResult<WebSocket> {
        Client::default().websocket(cancel, url, kwds).await
    }
}

mod blocking {
    use pyo3::{prelude::*, pybacked::PyBackedStr};

    use crate::{
        client::{
            BlockingClient,
            req::{Request, WebSocketRequest},
            resp::{BlockingResponse, BlockingWebSocket},
        },
        http::Method,
    };

    /// Make a GET request with the given parameters (blocking).
    #[inline]
    #[pyfunction]
    #[pyo3(signature = (url, **kwds))]
    pub fn get(py: Python, url: PyBackedStr, kwds: Option<Request>) -> PyResult<BlockingResponse> {
        request(py, Method::GET, url, kwds)
    }

    /// Make a POST request with the given parameters (blocking).
    #[inline]
    #[pyfunction]
    #[pyo3(signature = (url, **kwds))]
    pub fn post(py: Python, url: PyBackedStr, kwds: Option<Request>) -> PyResult<BlockingResponse> {
        request(py, Method::POST, url, kwds)
    }

    /// Make a PUT request with the given parameters (blocking).
    #[inline]
    #[pyfunction]
    #[pyo3(signature = (url, **kwds))]
    pub fn put(py: Python, url: PyBackedStr, kwds: Option<Request>) -> PyResult<BlockingResponse> {
        request(py, Method::PUT, url, kwds)
    }

    /// Make a PATCH request with the given parameters (blocking).
    #[inline]
    #[pyfunction]
    #[pyo3(signature = (url, **kwds))]
    pub fn patch(
        py: Python,
        url: PyBackedStr,
        kwds: Option<Request>,
    ) -> PyResult<BlockingResponse> {
        request(py, Method::PATCH, url, kwds)
    }

    /// Make a DELETE request with the given parameters (blocking).
    #[inline]
    #[pyfunction]
    #[pyo3(signature = (url, **kwds))]
    pub fn delete(
        py: Python,
        url: PyBackedStr,
        kwds: Option<Request>,
    ) -> PyResult<BlockingResponse> {
        request(py, Method::DELETE, url, kwds)
    }

    /// Make a HEAD request with the given parameters (blocking).
    #[inline]
    #[pyfunction]
    #[pyo3(signature = (url, **kwds))]
    pub fn head(py: Python, url: PyBackedStr, kwds: Option<Request>) -> PyResult<BlockingResponse> {
        request(py, Method::HEAD, url, kwds)
    }

    /// Make a OPTIONS request with the given parameters (blocking).
    #[inline]
    #[pyfunction]
    #[pyo3(signature = (url, **kwds))]
    pub fn options(
        py: Python,
        url: PyBackedStr,
        kwds: Option<Request>,
    ) -> PyResult<BlockingResponse> {
        request(py, Method::OPTIONS, url, kwds)
    }

    /// Make a TRACE request with the given parameters (blocking).
    #[inline]
    #[pyfunction]
    #[pyo3(signature = (url, **kwds))]
    pub fn trace(
        py: Python,
        url: PyBackedStr,
        kwds: Option<Request>,
    ) -> PyResult<BlockingResponse> {
        request(py, Method::TRACE, url, kwds)
    }

    /// Make a request with the given parameters (blocking).
    #[inline]
    #[pyfunction]
    #[pyo3(signature = (method, url, **kwds))]
    pub fn request(
        py: Python,
        method: Method,
        url: PyBackedStr,
        kwds: Option<Request>,
    ) -> PyResult<BlockingResponse> {
        BlockingClient::default().request(py, method, url, kwds)
    }

    /// Make a WebSocket connection with the given parameters (blocking).
    #[inline]
    #[pyfunction]
    #[pyo3(signature = (url, **kwds))]
    pub fn websocket(
        py: Python,
        url: PyBackedStr,
        kwds: Option<WebSocketRequest>,
    ) -> PyResult<BlockingWebSocket> {
        BlockingClient::default().websocket(py, url, kwds)
    }
}

/// Make a WebSocket connection with the given parameters.
#[inline]
#[pyfunction]
#[pyo3(signature = (url, **kwds))]
pub async fn websocket(
    #[pyo3(cancel_handle)] cancel: CancelHandle,
    url: PyBackedStr,
    kwds: Option<WebSocketRequest>,
) -> PyResult<WebSocket> {
    Client::default().websocket(cancel, url, kwds).await
}

#[pymodule(gil_used = false)]
fn wreq(py: Python, m: &Bound<'_, PyModule>) -> PyResult<()> {
    use r#async::{delete, get, head, options, patch, post, put, request, trace, websocket};

    Python::initialize();

    m.add_class::<SocketAddr>()?;
    m.add_class::<Message>()?;
    m.add_class::<StatusCode>()?;
    m.add_class::<Part>()?;
    m.add_class::<Multipart>()?;
    m.add_class::<Client>()?;
    m.add_class::<Response>()?;
    m.add_class::<WebSocket>()?;
    m.add_class::<Streamer>()?;
    m.add_class::<Method>()?;
    m.add_class::<Version>()?;

    m.add_function(wrap_pyfunction!(get, m)?)?;
    m.add_function(wrap_pyfunction!(post, m)?)?;
    m.add_function(wrap_pyfunction!(put, m)?)?;
    m.add_function(wrap_pyfunction!(patch, m)?)?;
    m.add_function(wrap_pyfunction!(delete, m)?)?;
    m.add_function(wrap_pyfunction!(head, m)?)?;
    m.add_function(wrap_pyfunction!(options, m)?)?;
    m.add_function(wrap_pyfunction!(trace, m)?)?;
    m.add_function(wrap_pyfunction!(request, m)?)?;
    m.add_function(wrap_pyfunction!(websocket, m)?)?;

    m.add_wrapped(wrap_pymodule!(proxy_module))?;
    m.add_wrapped(wrap_pymodule!(dns_module))?;
    m.add_wrapped(wrap_pymodule!(http1_module))?;
    m.add_wrapped(wrap_pymodule!(http2_module))?;
    m.add_wrapped(wrap_pymodule!(tls_module))?;
    m.add_wrapped(wrap_pymodule!(header_module))?;
    m.add_wrapped(wrap_pymodule!(cookie_module))?;
    m.add_wrapped(wrap_pymodule!(emulation_module))?;
    m.add_wrapped(wrap_pymodule!(redirect_module))?;
    m.add_wrapped(wrap_pymodule!(blocking_module))?;
    m.add_wrapped(wrap_pymodule!(exceptions_module))?;

    let sys = PyModule::import(py, intern!(py, "sys"))?;
    let sys_modules: Bound<'_, PyDict> = sys.getattr(intern!(py, "modules"))?.cast_into()?;
    sys_modules.set_item(intern!(py, "wreq.proxy"), m.getattr(intern!(py, "proxy"))?)?;
    sys_modules.set_item(intern!(py, "wreq.dns"), m.getattr(intern!(py, "dns"))?)?;
    sys_modules.set_item(intern!(py, "wreq.http1"), m.getattr(intern!(py, "http1"))?)?;
    sys_modules.set_item(intern!(py, "wreq.http2"), m.getattr(intern!(py, "http2"))?)?;
    sys_modules.set_item(intern!(py, "wreq.tls"), m.getattr(intern!(py, "tls"))?)?;
    sys_modules.set_item(
        intern!(py, "wreq.header"),
        m.getattr(intern!(py, "header"))?,
    )?;
    sys_modules.set_item(
        intern!(py, "wreq.cookie"),
        m.getattr(intern!(py, "cookie"))?,
    )?;
    sys_modules.set_item(
        intern!(py, "wreq.emulation"),
        m.getattr(intern!(py, "emulation"))?,
    )?;
    sys_modules.set_item(
        intern!(py, "wreq.redirect"),
        m.getattr(intern!(py, "redirect"))?,
    )?;
    sys_modules.set_item(
        intern!(py, "wreq.blocking"),
        m.getattr(intern!(py, "blocking"))?,
    )?;
    sys_modules.set_item(
        intern!(py, "wreq.exceptions"),
        m.getattr(intern!(py, "exceptions"))?,
    )?;
    Ok(())
}

#[pymodule(gil_used = false, name = "proxy")]
fn proxy_module(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<Proxy>()?;
    Ok(())
}

#[pymodule(gil_used = false, name = "dns")]
fn dns_module(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<LookupIpStrategy>()?;
    m.add_class::<DnsOptions>()?;
    Ok(())
}

#[pymodule(gil_used = false, name = "http1")]
fn http1_module(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<Http1Options>()?;
    Ok(())
}

#[pymodule(gil_used = false, name = "http2")]
fn http2_module(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<Http2Options>()?;
    m.add_class::<StreamId>()?;
    m.add_class::<StreamDependency>()?;
    m.add_class::<Priorities>()?;
    m.add_class::<Priority>()?;
    m.add_class::<PseudoId>()?;
    m.add_class::<PseudoOrder>()?;
    m.add_class::<SettingId>()?;
    m.add_class::<SettingsOrder>()?;
    Ok(())
}

#[pymodule(gil_used = false, name = "tls")]
fn tls_module(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<TlsVersion>()?;
    m.add_class::<Identity>()?;
    m.add_class::<CertStore>()?;
    m.add_class::<KeyLog>()?;
    m.add_class::<AlpnProtocol>()?;
    m.add_class::<AlpsProtocol>()?;
    m.add_class::<KeyShare>()?;
    m.add_class::<CertificateCompressionAlgorithm>()?;
    m.add_class::<ExtensionType>()?;
    m.add_class::<TlsOptions>()?;
    m.add_class::<TlsInfo>()?;
    Ok(())
}

#[pymodule(gil_used = false, name = "header")]
fn header_module(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<HeaderMap>()?;
    m.add_class::<OrigHeaderMap>()?;
    Ok(())
}

#[pymodule(gil_used = false, name = "cookie")]
fn cookie_module(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<Jar>()?;
    m.add_class::<Cookie>()?;
    m.add_class::<SameSite>()?;
    Ok(())
}

#[pymodule(gil_used = false, name = "emulation")]
fn emulation_module(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<Emulation>()?;
    m.add_class::<Profile>()?;
    m.add_class::<Platform>()?;
    Ok(())
}

#[pymodule(gil_used = false, name = "redirect")]
fn redirect_module(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<redirect::Policy>()?;
    m.add_class::<redirect::Attempt>()?;
    m.add_class::<redirect::Action>()?;
    m.add_class::<redirect::History>()?;
    Ok(())
}

#[pymodule(gil_used = false, name = "blocking")]
fn blocking_module(m: &Bound<'_, PyModule>) -> PyResult<()> {
    use blocking::{delete, get, head, options, patch, post, put, request, trace, websocket};

    m.add_function(wrap_pyfunction!(get, m)?)?;
    m.add_function(wrap_pyfunction!(post, m)?)?;
    m.add_function(wrap_pyfunction!(put, m)?)?;
    m.add_function(wrap_pyfunction!(patch, m)?)?;
    m.add_function(wrap_pyfunction!(delete, m)?)?;
    m.add_function(wrap_pyfunction!(head, m)?)?;
    m.add_function(wrap_pyfunction!(options, m)?)?;
    m.add_function(wrap_pyfunction!(trace, m)?)?;
    m.add_function(wrap_pyfunction!(request, m)?)?;
    m.add_function(wrap_pyfunction!(websocket, m)?)?;

    m.add_class::<BlockingClient>()?;
    m.add_class::<BlockingResponse>()?;
    m.add_class::<BlockingWebSocket>()?;
    Ok(())
}

#[pymodule(gil_used = false, name = "exceptions")]
fn exceptions_module(py: Python, m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add(intern!(py, "TlsError"), py.get_type::<TlsError>())?;
    m.add(intern!(py, "BodyError"), py.get_type::<BodyError>())?;
    m.add(intern!(py, "BuilderError"), py.get_type::<BuilderError>())?;
    m.add(
        intern!(py, "ConnectionError"),
        py.get_type::<ConnectionError>(),
    )?;
    m.add(
        intern!(py, "ProxyConnectionError"),
        py.get_type::<ProxyConnectionError>(),
    )?;
    m.add(
        intern!(py, "ConnectionResetError"),
        py.get_type::<ConnectionResetError>(),
    )?;
    m.add(intern!(py, "DecodingError"), py.get_type::<DecodingError>())?;
    m.add(intern!(py, "RedirectError"), py.get_type::<RedirectError>())?;
    m.add(intern!(py, "TimeoutError"), py.get_type::<TimeoutError>())?;
    m.add(intern!(py, "StatusError"), py.get_type::<StatusError>())?;
    m.add(intern!(py, "RequestError"), py.get_type::<RequestError>())?;
    m.add(intern!(py, "UpgradeError"), py.get_type::<UpgradeError>())?;
    m.add(
        intern!(py, "WebSocketError"),
        py.get_type::<WebSocketError>(),
    )?;
    m.add(intern!(py, "RustPanic"), py.get_type::<RustPanic>())?;
    Ok(())
}
