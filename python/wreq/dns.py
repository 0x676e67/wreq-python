"""DNS resolution types and utilities."""

from enum import Enum, auto
from typing import Sequence, final
from ipaddress import IPv4Address, IPv6Address

__all__ = [
    "LookupIpStrategy",
    "DnsOptions",
]


@final
class LookupIpStrategy(Enum):
    """IP lookup strategy for DNS resolution.

    Determines the order and types of IP addresses to resolve.
    """

    IPV4_ONLY = auto()
    """Only resolve IPv4 addresses."""

    IPV6_ONLY = auto()
    """Only resolve IPv6 addresses."""

    IPV4_AND_IPV6 = auto()
    """Resolve both IPv4 and IPv6 addresses."""

    IPV6_THEN_IPV4 = auto()
    """Prefer IPv6, fall back to IPv4."""

    IPV4_THEN_IPV6 = auto()
    """Prefer IPv4, fall back to IPv6."""


class DnsOptions:
    """DNS options for customizing DNS resolution behavior.

    Args:
        system_dns: Whether to use the system DNS resolver. Defaults to False,
            which means using the hickory DNS resolver.
        lookup_ip_strategy: The IP lookup strategy to use. Defaults to
            IPV4_AND_IPV6. This option only applies to the hickory DNS
            resolver and has no effect when using the system DNS resolver.

    Example:
        >>> from wreq import DnsOptions, LookupIpStrategy
        >>> from ipaddress import IPv4Address
        >>> options = DnsOptions(LookupIpStrategy.IPV4_ONLY)
        >>> options.add_resolve("example.com", [IPv4Address("127.0.0.1")])
    """

    def __init__(
        self,
        system_dns: bool = False,
        lookup_ip_strategy: LookupIpStrategy = LookupIpStrategy.IPV4_AND_IPV6,
    ) -> None:
        """Create a new DnsOptions.

        Args:
            system_dns: Whether to use the system DNS resolver. When False,
                the hickory DNS resolver is used.
            lookup_ip_strategy: The IP lookup strategy to use. Only effective
                when using the hickory DNS resolver.
        """
        ...

    def add_resolve(
        self,
        domain: str,
        addrs: Sequence[IPv4Address | IPv6Address],
    ) -> None:
        """Add a custom DNS resolve mapping.

        Maps a domain name to a list of IP addresses, bypassing normal DNS resolution.

        Args:
            domain: The domain name to map.
            addrs: List of IP addresses to resolve the domain to.

        Example:
            >>> from ipaddress import IPv4Address, IPv6Address
            >>> options = DnsOptions()
            >>> options.add_resolve("api.example.com", [IPv4Address("192.168.1.1")])
            >>> options.add_resolve("cdn.example.com", [
            ...     IPv6Address("2001:db8::1"),
            ...     IPv4Address("203.0.113.1"),
            ... ])
        """
        ...
