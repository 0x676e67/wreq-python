# Browser emulation

Profiles configure TLS, HTTP/2, and request headers to resemble a browser or
another supported HTTP client. They include Chrome, Firefox, Safari, Edge,
and OkHttp profiles. See the [profile reference](../api/emulation.md) for the
available versions.

## Choosing a profile

Set `emulation` on a client when its requests should use the same profile.
`Profile.Chrome154` and `Profile.Firefox152` are available in the current API.

```python
import asyncio
from wreq import Client
from wreq.emulation import Profile


async def main():
    async with Client(emulation=Profile.Firefox152) as client:
        async with client.get("https://tls.peet.ws/api/all") as response:
            print(await response.json())


asyncio.run(main())
```

`Emulation.Firefox152` is also accepted as a profile alias. The endpoint above
reports the connection and headers it received; it is an external diagnostic
service.

## Choosing a platform or overriding one request

Use `Emulation` to combine a profile with a platform. Passing it to a request
applies it to that request instead of changing the client's configuration.
Use `default_headers=False` to keep the client's original profile headers
out of that request.

```python
import asyncio
from wreq import Client
from wreq.emulation import Emulation, Platform, Profile


async def main():
    async with Client(emulation=Profile.Firefox152) as client:
        async with client.get(
            "https://tls.peet.ws/api/all",
            emulation=Emulation(
                profile=Profile.Chrome154,
                platform=Platform.Android,
            ),
            default_headers=False,
        ) as response:
            print(await response.json())


asyncio.run(main())
```

The platform affects the profile's platform-specific headers and user agent.
Emulation configures HTTP traffic; it does not run JavaScript or provide a
browser DOM.

## Supplying your own headers

`Emulation(headers=False)` skips the profile's header preset. Set `headers`
on the client or request to supply your own values.

```python
import asyncio
from wreq import Client
from wreq.emulation import Emulation, Profile


async def main():
    async with Client(
        emulation=Emulation(profile=Profile.Chrome154, headers=False),
        headers={"User-Agent": "MyClient/1.0", "Accept": "application/json"},
    ) as client:
        async with client.get("https://httpbin.org/headers") as response:
            print(await response.json())


asyncio.run(main())
```

The request option `default_headers=False` separately suppresses client default
headers for that request. Changing headers or protocol settings changes the
resulting fingerprint.

`Emulation(http2=False)` skips the profile's HTTP/2 settings; it does not
disable the protocol. Use `Client(http1_only=True)` to restrict requests to
HTTP/1.

## Custom TLS and HTTP/2 settings

Use `TlsOptions` and `Http2Options` when you need specific protocol settings.
This example configures ALPN, a minimum TLS version, HTTP/2 flow control, and
pseudo-header order without using a browser profile.

```python
import asyncio
from wreq import Client
from wreq.http2 import Http2Options, PseudoId, PseudoOrder
from wreq.tls import AlpnProtocol, TlsOptions, TlsVersion


async def main():
    async with Client(
        tls_options=TlsOptions(
            alpn_protocols=[AlpnProtocol.HTTP2, AlpnProtocol.HTTP1],
            min_tls_version=TlsVersion.TLS_1_2,
        ),
        http2_options=Http2Options(
            initial_window_size=1024 * 1024,
            initial_connection_window_size=4 * 1024 * 1024,
            headers_pseudo_order=PseudoOrder(
                PseudoId.METHOD,
                PseudoId.SCHEME,
                PseudoId.AUTHORITY,
                PseudoId.PATH,
            ),
        ),
        headers={"User-Agent": "MyClient/1.0", "Accept": "application/json"},
        orig_headers=["User-Agent", "Accept"],
    ) as client:
        async with client.get("https://tls.peet.ws/api/all") as response:
            print(await response.json())


asyncio.run(main())
```

`orig_headers` controls original header spelling and order where the protocol
allows it; header values still come from `headers`. HTTP/2 transmits lowercase
field names. For individual settings, see [TLS options](../api/tls.md) and
[HTTP/2 options](../api/http2.md). The same configuration works with
`wreq.blocking.Client`.
