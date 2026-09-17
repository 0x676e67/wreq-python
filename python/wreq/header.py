"""
HTTP Header Management

This module provides efficient storage and manipulation of HTTP headers with
support for multiple values per header name. The HeaderMap class is designed
to handle the complexities of HTTP header processing, including case-insensitive
header names and multiple header values.

The implementation follows HTTP specifications (RFC 7230) for header handling,
including proper support for headers that can have multiple values (like
Set-Cookie, Accept-Encoding, etc.).
"""

from typing import Mapping, Iterator, Sequence, Tuple

__all__ = ["HeaderMap", "OrigHeaderMap"]


class HeaderMap:
    r"""
    A case-insensitive HTTP header map supporting multiple values per header.

    This class provides efficient storage and retrieval of HTTP headers,
    automatically handling case-insensitive header names and supporting
    headers with multiple values (such as Set-Cookie or Accept-Encoding).

    The implementation follows HTTP/1.1 specifications for header handling
    and provides both dictionary-like access and specialized methods for
    HTTP header manipulation.
    """

    def __getitem__(self, key: str) -> bytes | None:
        """Get the first value for a header name (case-insensitive)."""
        ...

    def __setitem__(self, key: str, value: str) -> None:
        """Set a header to a single value, replacing any existing values."""
        ...

    def __delitem__(self, key: str) -> None:
        """Remove all values for a header name."""
        ...

    def __contains__(self, key: str) -> bool:
        """Check if a header name exists (case-insensitive)."""
        ...

    def __len__(self) -> int:
        """Return the total number of header values (not unique names)."""
        ...

    def __iter__(self) -> Iterator[Tuple[bytes, bytes]]:
        """Iterate all header(name, value) pairs, including duplicates for multiple values."""
        ...

    def __str__(self) -> str:
        """Return a string representation of all headers."""
        ...

    def __init__(
        self, init: Mapping[str, str] | None = None, capacity: int | None = None
    ) -> None:
        """
        Create a new HeaderMap.

        Args:
            init: Optional dictionary to initialize headers from
            capacity: Optional initial capacity hint for performance

        Returns:
            A new HeaderMap instance

        Example:
            ```python
            # Empty header map
            headers = HeaderMap()

            # Initialize from dictionary
            headers = HeaderMap({
                'Content-Type': 'text/html',
                'Cache-Control': 'no-cache'
            })

            # Pre-allocate capacity for performance
            headers = HeaderMap(capacity=50)
            ```
        """

    def contains_key(self, key: str) -> bool:
        r"""
        Check if the header map contains the given key.

        This is equivalent to using the 'in' operator but provides
        an explicit method name. Header name comparison is case-insensitive.

        Args:
            key: The header name to check

        Returns:
            True if the header exists, False otherwise
        """
        ...

    def insert(self, key: str, value: str) -> None:
        r"""
        Insert a header, replacing any existing values.

        This method replaces all existing values for the given header name
        with the new value. For adding additional values, use append() instead.

        Args:
            key: The header name (case-insensitive)
            value: The header value to set
        """
        ...

    def append(self, key: str, value: str) -> None:
        r"""
        Append a value to an existing header or create a new one.

        If the header already exists, this adds an additional value.
        If the header doesn't exist, it creates a new header with this value.
        This is useful for headers that can have multiple values.

        Args:
            key: The header name (case-insensitive)
            value: The header value to append
        """
        ...

    def update(self, headers: Mapping[str, str] | "HeaderMap") -> None:
        r"""
        Extend the map with the given headers.

        The value of a header that is already present is replaced, and a header that
        is missing is added.

        Args:
            headers: The headers to merge into the map
        """
        ...

    def remove(self, key: str) -> None:
        r"""
        Remove all values for a header name.

        This removes the header entirely from the map. If the header
        doesn't exist, this method does nothing.

        Args:
            key: The header name to remove (case-insensitive)
        """
        ...

    def get(self, key: str, default: bytes | None = None) -> bytes | None:
        r"""
        Get the first value for a header name with optional default.

        Returns the first value associated with the header name, or the
        default value if the header doesn't exist. For headers with multiple
        values, use get_all() to retrieve all values.

        Args:
            key: The header name (case-insensitive)
            default: Value to return if header doesn't exist

        Returns:
            The first header value as bytes, or the default value
        """
        ...

    def get_all(self, key: str) -> Iterator[bytes]:
        r"""
        Get all values for a header name.

        Returns an iterator over all values associated with the header name.
        This is useful for headers that can have multiple values, such as
        Set-Cookie, Accept-Encoding, or custom headers.

        Args:
            key: The header name (case-insensitive)

        Returns:
            An iterator over all header values
        """
        ...

    def values(self) -> Iterator[bytes]:
        """
        Iterate over all header values.

        Returns:
            An iterator over all header values as bytes.
        """
        ...

    def keys(self) -> Iterator[bytes]:
        """
        Iterate over unique header names.

        Returns:
            An iterator over unique header names as bytes.
        """
        ...

    def len(self) -> int:
        """
        Get the total number of header values.

        This returns the total count of header values, which can be greater
        than the number of unique header names if some headers have multiple
        values.

        Returns:
            Total number of header values stored
        """
        ...

    def keys_len(self) -> int:
        """
        Get the number of unique header names.

        This returns the count of unique header names, regardless of how
        many values each header has.

        Returns:
            Number of unique header names
        """
        ...

    def is_empty(self) -> bool:
        """
        Check if the header map is empty.

        Returns:
            True if no headers are stored, False otherwise
        """
        ...

    def clear(self) -> None:
        """
        Remove all headers from the map.

        After calling this method, the header map will be empty and
        is_empty() will return True.
        """
        ...


class OrigHeaderMap:
    """
    A map from header names to their original casing as received in an HTTP message.

    OrigHeaderMap not only preserves the original case of each header name as it appeared
    in the HTTP message, but also maintains the insertion order of headers. This makes
    it suitable for use cases where the order of headers matters, such as HTTP/1.x message
    serialization, proxying, or reproducing requests/responses exactly as received.

    The map stores a mapping between the case-insensitive (standard) header name and the
    original case-sensitive header name as it appeared in the HTTP message.

    Example:
        If an HTTP message included the following headers:

            x-Bread: Baguette
            X-BREAD: Pain
            x-bread: Ficelle

        Then the OrigHeaderMap would preserve both the exact casing and order of these headers:
        - Standard name "x-bread" maps to original "x-Bread"
        - Standard name "x-bread" maps to original "X-BREAD"
        - Standard name "x-bread" maps to original "x-bread"

        This allows the client to reproduce the exact header casing when forwarding or
        reconstructing the HTTP message.
    """

    def __init__(
        self,
        init: Sequence[str] | None = None,
        capacity: int | None = None,
    ) -> None:
        """
        Creates a new OrigHeaderMap from an optional list of header names.

        Args:
            init: Optional list of header names to initialize with.
            capacity: Optional initial capacity for the map.
        """
        ...

    def __iter__(self) -> Iterator[Tuple[bytes, bytes]]:
        """
        Returns an iterator over the (standard_name, original_name) pairs.

        Returns:
            An iterator over header name pairs.
        """
        ...

    def __len__(self) -> int:
        """
        Returns the number of header names stored in the map.
        """
        ...

    def insert(self, value: str) -> bool:
        """
        Insert a new header name into the collection.

        If the map did not previously have this key present, then False is returned.
        If the map did have this key present, the new value is pushed to the end
        of the list of values currently associated with the key. The key is not
        updated, though; this matters for types that can be == without being identical.

        Args:
            value: The header name to insert.

        Returns:
            True if the key was newly inserted, False if it already existed.
        """
        ...

    def extend(self, other: "OrigHeaderMap") -> None:
        """
        Extends the map with all entries from another OrigHeaderMap, preserving order.

        Args:
            other: Another OrigHeaderMap to extend from.
        """
        ...
