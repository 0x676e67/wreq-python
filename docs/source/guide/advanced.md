# Advanced features

Use streaming uploads for data produced incrementally, and multipart forms for
named fields and files. Response streaming is covered in [basic usage](basic.md).

## Streaming request bodies

Pass a synchronous iterator or an async generator to `body`. Each item must
be a supported body chunk, such as `bytes` or `str`:

```python
import asyncio

from wreq import Client


async def chunks():
    for number in range(3):
        await asyncio.sleep(0.01)
        yield f"line {number}\n".encode()


async def main():
    async with Client() as client:
        async with client.post("https://httpbin.io/post", body=chunks()) as response:
            response.raise_for_status()
            print(await response.json())


asyncio.run(main())
```

Async upload generators run on the caller's event loop with its context
variables. An exception fails the request. The producer closes the generator
after use; cancellation or dropping an upload schedules its cancellation and
cleanup on that loop. Keep the loop running until cleanup finishes.

Use synchronous iterators for blocking uploads. A blocking call on the
producer's event-loop thread prevents an async generator from progressing.
Create a fresh iterator for each request; an exhausted upload cannot be replayed.

??? note "Upload buffering"

    Async generators and synchronous iterators on worker schedulers read ahead
    within a 256 KiB queue budget. Each chunk counts
    as at least 4 KiB, so up to 64 small chunks can queue. A chunk larger than
    256 KiB queues alone at its full size. One more chunk can wait outside the
    budget, so this is not a hard limit on total upload memory.

    Synchronous iterators use the runtime's blocking pool with worker schedulers.
    A current-thread runtime reads them on demand on its driving thread. See
    [runtimes](runtime.md#current-thread) for its restrictions.

## Multipart fields and files

`Part` accepts text, bytes, a `pathlib.Path`, or an upload iterator. The following
example expects `report.txt` in the current directory. A path streams the file
without first loading it into Python memory:

```python
import asyncio
from pathlib import Path

from wreq import Client, Multipart, Part


async def main():
    form = Multipart(
        Part(name="description", value="Monthly report"),
        Part(
            name="document",
            value=Path("report.txt"),
            filename="report.txt",
            mime="text/plain",
        ),
    )
    async with Client() as client:
        async with client.post("https://httpbin.io/post", multipart=form) as response:
            response.raise_for_status()
            print(await response.json())


asyncio.run(main())
```

File paths are opened on the client's runtime when building the request. A
missing file raises an error at that point. Stream parts are consumed once;
recreate them for another request. Text, bytes and path parts can be reused.

Construct async-generator parts inside a running event loop: their producers
start at `Part` construction. Their buffering and cleanup follow the same rules
as async request bodies. The [Multipart and Part reference](../api/wreq.md)
also covers stream lengths and per-part headers.

## Custom runtimes

The [runtime guide](runtime.md) explains shared and dedicated worker pools,
MT/ST settings, caller-driven blocking I/O, and client lifetime. Use the default
runtime unless your workload needs a different scheduling configuration.

## TLS certificates and key logging

Certificate and hostname verification are enabled by default. For a private
certificate authority, pass a PEM bundle using `tls_verify=Path("ca.pem")`.
For mutual TLS, construct an `Identity` and pass it as `tls_identity`. See the
[TLS reference](../api/tls.md) for certificate and key formats.

Use `tls_keylog` to write TLS session keys for tools such as Wireshark:

```python
import asyncio

from wreq import Client
from wreq.tls import KeyLog


async def main():
    async with Client(tls_keylog=KeyLog.file("keylog.log")) as client:
        async with client.get("https://httpbin.io/get") as response:
            print(await response.text())


asyncio.run(main())
```

The log contains keys that can decrypt captured traffic. Keep it private and
enable logging only when you need it.

## DNS and protocol options

`dns_options` selects the resolver and can override specific host addresses.
Overrides retain the URL's hostname for HTTP and TLS. This configuration uses
system DNS; omit it to use the default Hickory resolver:

```python
from wreq.blocking import Client
from wreq.dns import DnsOptions


with Client(dns_options=DnsOptions(system_dns=True)) as client:
    with client.get("https://httpbin.io/get") as response:
        print(response.text())
```

For explicit addresses, use `DnsOptions.add_resolve()` with `ipaddress` objects.
`lookup_ip_strategy` applies to Hickory, not system DNS. See the
[DNS reference](../api/dns.md).

At client construction, `http1_only=True` or `http2_only=True` restricts the
protocol. A request can also specify `version=Version.HTTP_11` or
`version=Version.HTTP_2`. Check `response.version` to see the protocol used.
Use [Http1Options](../api/http1.md), [Http2Options](../api/http2.md) and
[TlsOptions](../api/tls.md) for individual protocol settings, or an
[emulation profile](emulation.md) for a coordinated browser configuration.

## Header case and order

`headers` supplies values. `orig_headers` supplies HTTP/1 header spelling and
ordering; naming a header there does not create its value. HTTP/2 uses lowercase
field names and separate pseudo-header settings.

```python
import asyncio

from wreq import Client


async def main():
    async with Client(
        http1_only=True,
        headers={"User-Agent": "my-app/1.0", "Accept": "application/json"},
        orig_headers=["Host", "User-Agent", "Accept"],
    ) as client:
        async with client.get("https://httpbin.io/headers") as response:
            print(await response.json())


asyncio.run(main())
```

For repeated headers and explicit ordering, see
[HeaderMap and OrigHeaderMap](../api/header.md).
