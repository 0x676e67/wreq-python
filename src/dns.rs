//! DNS resolution via the [hickory-resolver](https://github.com/hickory-dns/hickory-dns) crate

use std::{
    net::{IpAddr, SocketAddr},
    sync::{Arc, OnceLock},
};

use hickory_resolver::{
    TokioResolver, config::ResolverConfig, lookup_ip::LookupIpIntoIter,
    name_server::TokioConnectionProvider,
};
use pyo3::{prelude::*, pybacked::PyBackedStr};
use wreq::dns::{Addrs, Name, Resolve, Resolving};

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

// Static resolvers for each IP strategy, lazily initialized
static RESOLVER_IPV4_ONLY: OnceLock<TokioResolver> = OnceLock::new();
static RESOLVER_IPV6_ONLY: OnceLock<TokioResolver> = OnceLock::new();
static RESOLVER_IPV4_AND_IPV6: OnceLock<TokioResolver> = OnceLock::new();
static RESOLVER_IPV6_THEN_IPV4: OnceLock<TokioResolver> = OnceLock::new();
static RESOLVER_IPV4_THEN_IPV6: OnceLock<TokioResolver> = OnceLock::new();

/// Wrapper around an [`TokioResolver`], which implements the `Resolve` trait.
#[derive(Clone)]
pub struct HickoryResolver {
    /// Shared, lazily-initialized Tokio-based DNS resolver.
    resolver: &'static TokioResolver,
}

impl HickoryResolver {
    /// Create a new resolver with the default configuration,
    /// which reads from `/etc/resolve.conf`. The options are
    /// overriden to look up for both IPv4 and IPv6 addresses
    /// to work with "happy eyeballs" algorithm.
    pub fn new(strategy: LookupIpStrategy) -> HickoryResolver {
        let cell = match strategy {
            LookupIpStrategy::IPV4_ONLY => &RESOLVER_IPV4_ONLY,
            LookupIpStrategy::IPV6_ONLY => &RESOLVER_IPV6_ONLY,
            LookupIpStrategy::IPV4_AND_IPV6 => &RESOLVER_IPV4_AND_IPV6,
            LookupIpStrategy::IPV6_THEN_IPV4 => &RESOLVER_IPV6_THEN_IPV4,
            LookupIpStrategy::IPV4_THEN_IPV6 => &RESOLVER_IPV4_THEN_IPV6,
        };

        HickoryResolver {
            resolver: cell.get_or_init(move || {
                let mut builder = match TokioResolver::builder_tokio() {
                    Ok(resolver) => resolver,
                    Err(err) => {
                        eprintln!("error reading DNS system conf: {}, using defaults", err);
                        TokioResolver::builder_with_config(
                            ResolverConfig::default(),
                            TokioConnectionProvider::default(),
                        )
                    }
                };
                builder.options_mut().ip_strategy = strategy.into_ffi();
                builder.build()
            }),
        }
    }
}

struct SocketAddrs {
    iter: LookupIpIntoIter,
}

impl Resolve for HickoryResolver {
    fn resolve(&self, name: Name) -> Resolving {
        let resolver = self.clone();
        Box::pin(async move {
            let lookup = resolver.resolver.lookup_ip(name.as_str()).await?;
            let addrs: Addrs = Box::new(SocketAddrs {
                iter: lookup.into_iter(),
            });
            Ok(addrs)
        })
    }
}

impl Iterator for SocketAddrs {
    type Item = SocketAddr;

    #[inline(always)]
    fn next(&mut self) -> Option<Self::Item> {
        self.iter.next().map(|ip_addr| SocketAddr::new(ip_addr, 0))
    }
}
