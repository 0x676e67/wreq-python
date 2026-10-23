# Python Type Stubs Module

This directory contains Python type hints (Type Stubs) files for the `wreq` package.

## File Structure

- **`.pyi` files**: Python stub files providing complete type annotations and function signatures
  - `__init__.pyi` - Core type definitions
  - `blocking.py` - Blocking client interface
  - `cookie.py` - Cookie management
  - `dns.py` - DNS resolver options
  - `emulation.py` - Browser emulation
  - `exceptions.py` - Exception hierarchy; unlike the stubs, the runtime classes live here
  - `header.py` - HTTP header handling
  - `http1.py` - HTTP/1.x options
  - `http2.py` - HTTP/2 options
  - `proxy.py` - Proxy support
  - `redirect.py` - Redirect handling
  - `tls.py` - TLS/SSL configuration

- **`py.typed`**: PEP 561 marker file indicating this package supports type checking

- **`wreq.abi3.so`**: Compiled binary extension module (built from Rust code)

## Purpose

These type stub files enable:

1. **IDE Support**: Intelligent code completion, parameter hints, and type checking
2. **Static Type Checking**: Support for tools like mypy, pyright for static analysis
3. **Documentation Generation**: Help generate accurate API documentation
4. **Developer Experience**: Improve code readability and maintainability

## Example

```python
from wreq import Client, Method

# IDE and type checkers can accurately infer types
client = Client()
response = await client.request(
    method=Method.GET,  # Type-safe enum
    url="https://api.example.com"
)
```
