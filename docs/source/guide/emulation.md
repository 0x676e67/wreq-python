# Emulation

!!! info "Emulation topics"
    - Firefox/Chrome/Android
    - Custom TLS/HTTP2

The built-in profiles include `Profile.Chrome154` and `Profile.Firefox152`.
Use a profile directly, or combine it with a platform using `Emulation`.

### Simple Firefox Emulation

```python
import asyncio
from wreq import Client
from wreq.emulation import Emulation


async def main():
    client = Client(
        emulation=Emulation.Firefox152,
    )
    resp = await client.get("https://tls.peet.ws/api/all")
    print(f"Status: {resp.status}")
    print(f"Content: {await resp.text()}")


if __name__ == "__main__":
    asyncio.run(main())
```

### Chrome on Android Emulation

```python
import asyncio
from wreq import Client
from wreq.emulation import Emulation, Profile, Platform


async def main():
    client = Client()
    resp = await client.get(
        "https://tls.peet.ws/api/all",
        emulation=Emulation(
            profile=Profile.Chrome154,
            platform=Platform.Android,
        ),
        # Disable client default headers
        default_headers=False,
    )
    print(f"Status: {resp.status}")
    print(f"Content: {await resp.text()}")


if __name__ == "__main__":
    asyncio.run(main())
```

### Advanced Configuration with Custom TLS and HTTP/2

```python
import asyncio
from wreq import Client
from wreq.tls import TlsOptions, TlsVersion, AlpnProtocol
from wreq.http2 import Http2Options, PseudoId, PseudoOrder
from wreq.header import HeaderMap, OrigHeaderMap


async def main():
    # TLS options configuration
    tls_options = TlsOptions(
        grease_enabled=True,
        enable_ocsp_stapling=True,
        curves_list=":".join(["X25519", "P-256", "P-384"]),
        cipher_list=":".join(
            [
                "TLS_AES_128_GCM_SHA256",
                "TLS_AES_256_GCM_SHA384",
                "TLS_CHACHA20_POLY1305_SHA256",
                "TLS_ECDHE_ECDSA_WITH_AES_128_GCM_SHA256",
                "TLS_ECDHE_RSA_WITH_AES_128_GCM_SHA256",
            ]
        ),
        sigalgs_list=":".join(
            [
                "ecdsa_secp256r1_sha256",
                "rsa_pss_rsae_sha256",
                "rsa_pkcs1_sha256",
            ]
        ),
        alpn_protocols=[AlpnProtocol.HTTP2, AlpnProtocol.HTTP1],
        min_tls_version=TlsVersion.TLS_1_2,
        max_tls_version=TlsVersion.TLS_1_3,
    )

    # HTTP/2 options configuration
    http2_options = Http2Options(
        initial_stream_id=3,
        initial_window_size=16777216,
        initial_connection_window_size=16711681 + 65535,
        headers_pseudo_order=PseudoOrder(
            PseudoId.METHOD,
            PseudoId.PATH,
            PseudoId.AUTHORITY,
            PseudoId.SCHEME,
        ),
    )

    # Default headers
    headers = HeaderMap()
    headers.insert("User-Agent", "CustomAgent/1.0")
    headers.insert("Accept-Language", "en-US")
    headers.insert("Accept-Encoding", "br, gzip, deflate")

    # Original headers to preserve case and order
    orig_headers = OrigHeaderMap()
    orig_headers.insert("User-Agent")
    orig_headers.insert("Accept-Language")
    orig_headers.insert("Accept-Encoding")

    # Create client with advanced configuration
    client = Client(
        tls_options=tls_options,
        http2_options=http2_options,
        headers=headers,
        orig_headers=orig_headers,
    )

    resp = await client.post("https://tls.peet.ws/api/all")
    print(f"Status: {resp.status}")
    print(f"Content: {await resp.text()}")


if __name__ == "__main__":
    asyncio.run(main())
```
