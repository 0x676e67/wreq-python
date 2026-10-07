//! DNS resolution via the [hickory-resolver](https://github.com/hickory-dns/hickory-dns) crate

use std::{
    net::{IpAddr, SocketAddr},
    sync::Arc,
};

use futures_util::FutureExt;
use hickory_resolver::{
    TokioResolver,
    config::{CLOUDFLARE, ResolverConfig},
    net::runtime::TokioRuntimeProvider,
};
use pyo3::{prelude::*, pybacked::PyBackedStr};
use wreq::dns::{Addrs, Name, Resolve, Resolving};

use crate::error::Error;

define_enum!(
    /// The lookup ip strategy.
    const,
    LookupIpStrategy,
    hickory_resolver::config::LookupIpStrategy,
    (IPV4_ONLY, Ipv4Only),
    (IPV6_ONLY, Ipv6Only),
    (IPV4_AND_IPV6, Ipv4AndIpv6),
    (IPV6_THEN_IPV4, Ipv6thenIpv4),
    (IPV4_THEN_IPV6, Ipv4thenIpv6)
);

impl Default for LookupIpStrategy {
    #[inline]
    fn default() -> Self {
        LookupIpStrategy::IPV4_AND_IPV6
    }
}

/// DNS resolver options for customizing DNS resolution behavior.
#[derive(Clone)]
#[pyclass(from_py_object)]
pub struct DnsOptions {
    pub system_dns: bool,
    pub lookup_ip_strategy: LookupIpStrategy,
    pub resolve_to_addrs: Vec<(Arc<PyBackedStr>, Vec<SocketAddr>)>,
}

#[pymethods]
impl DnsOptions {
    /// Create a new [`DnsOptions`].
    ///
    /// When `system_dns` is `false`, hickory is used as the DNS resolver.
    /// `lookup_ip_strategy` only applies to hickory and has no effect when
    /// using the system DNS resolver.
    #[new]
    #[pyo3(signature=(system_dns = false, lookup_ip_strategy = LookupIpStrategy::IPV4_AND_IPV6))]
    pub fn new(system_dns: bool, lookup_ip_strategy: LookupIpStrategy) -> Self {
        DnsOptions {
            system_dns,
            lookup_ip_strategy,
            resolve_to_addrs: Vec::new(),
        }
    }

    /// Add a custom DNS resolve mapping.
    #[pyo3(signature=(domain, addrs))]
    pub fn add_resolve(&mut self, domain: PyBackedStr, addrs: Vec<IpAddr>) {
        self.resolve_to_addrs.push((
            Arc::new(domain),
            addrs.into_iter().map(|ip| SocketAddr::new(ip, 0)).collect(),
        ));
    }
}

/// Wrapper around an [`TokioResolver`], which implements the `Resolve` trait.
#[derive(Clone)]
pub struct HickoryResolver {
    // DNS connections must not outlive or cross the client's selected runtime.
    resolver: TokioResolver,
}

impl HickoryResolver {
    /// Use the system DNS configuration, falling back to Cloudflare if unreadable.
    pub fn new(strategy: LookupIpStrategy) -> Result<Self, Error> {
        let mut builder = match TokioResolver::builder_tokio() {
            Ok(resolver) => resolver,
            Err(err) => {
                eprintln!(
                    "error reading DNS system conf: {}, using Cloudflare DNS",
                    err
                );
                TokioResolver::builder_with_config(
                    ResolverConfig::udp_and_tcp(&CLOUDFLARE),
                    TokioRuntimeProvider::default(),
                )
            }
        };
        builder.options_mut().ip_strategy = strategy.into_ffi();
        let resolver = builder.build().map_err(Error::Dns)?;
        Ok(Self { resolver })
    }
}

impl Resolve for HickoryResolver {
    fn resolve(&self, name: Name) -> Resolving {
        let resolver = self.clone();
        async move {
            let lookup = resolver.resolver.lookup_ip(name.as_str()).await?;
            let addrs: Addrs = Box::new(lookup.into_iter().map(|ip| SocketAddr::new(ip, 0)));
            Ok(addrs)
        }
        .boxed()
    }
}
