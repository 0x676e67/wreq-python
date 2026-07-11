# wreq

[![CI](https://github.com/0x676e67/wreq-python/actions/workflows/ci.yml/badge.svg)](https://github.com/0x676e67/wreq-python/actions/workflows/ci.yml)
![PyPI - License](https://img.shields.io/pypi/l/wreq)
![Python Version from PEP 621 TOML](https://img.shields.io/python/required-version-toml?tomlFilePath=https%3A%2F%2Fraw.githubusercontent.com%2F0x676e67%2Fwreq-python%2Fmain%2Fpyproject.toml&logo=python)
[![PyPI](https://img.shields.io/pypi/v/wreq?logo=python)](https://pypi.org/project/wreq/)
[![Discord chat][discord-badge]][discord-url]

[discord-badge]: https://img.shields.io/discord/1486741856397164788.svg?logo=discord
[discord-url]: https://discord.gg/rfbvyFkgq3

> 🚀 Help me work seamlessly with open source sharing by [sponsoring me on GitHub](https://github.com/0x676e67/0x676e67/blob/main/SPONSOR.md)

An ergonomic and modular Python HTTP Client for high-fidelity protocol matching, featuring customizable TLS, JA3/JA4, and HTTP/2 signature capabilities, powered by [wreq](https://github.com/0x676e67/wreq).

## Features

- Async and Blocking `Client`s
- Plain bodies, JSON, urlencoded, multipart
- HTTP Trailer
- Cookie Store
- Redirect Policy
- Original Header
- Rotating Proxies
- Connection Pooling
- Streaming Transfers
- Zero-Copy Transfers
- WebSocket Upgrade
- HTTPS via BoringSSL
- Free-Threaded Safety
- Automatic Decompression
- Certificate Store (CAs & mTLS)

## Example

The following example uses the `asyncio` runtime with `wreq` installed via pip:

```bash
pip install wreq
```

And then the code:

```python
import asyncio
from wreq import Client, Emulation


async def main():
    # Build a client
    client = Client(emulation=Emulation.Firefox149)

    # Use the API you're already familiar with
    resp = await client.get("https://tls.peet.ws/api/all")
    print(await resp.text())


if __name__ == "__main__":
    asyncio.run(main())

```

Additional learning resources include:

- [Examples](https://github.com/0x676e67/wreq-python/tree/main/examples)
- [DeepWiki](https://deepwiki.com/0x676e67/wreq-python)
- [Documentation](https://python.wreq.org)

## Behavior

1. **HTTP/2 over TLS**

Due to the complexity of TLS encryption and the widespread adoption of HTTP/2, browser fingerprints such as **JA3**, **JA4**, and **Akamai** cannot be reliably emulated using simple fingerprint strings. Instead of parsing and emulating these string-based fingerprints, `wreq` provides fine-grained control over TLS and HTTP/2 extensions and settings for precise browser behavior emulation.

2. **Device Emulation**

**TLS** and **HTTP/2** fingerprints are often identical across various browser models because these underlying protocols evolve slower than browser release cycles. **100+ browser device emulation [profiles](https://python.wreq.org/en/latest/getting-started/introduction/#behavior)** are maintained in **wreq**.

## Building

1. Platforms

- Linux(**glibc**/**musl**): `x86_64`, `aarch64`, `armv7`, `i686`
- macOS: `x86_64`,`aarch64`
- Windows: `x86_64`,`i686`,`aarch64`
- Android: `aarch64`, `x86_64`

2. Development

Install the BoringSSL build environment by referring to [boringssl](https://github.com/google/boringssl/blob/main/BUILDING.md).

```bash
# on ubuntu or debian
sudo apt install -y build-essential cmake perl pkg-config libclang-dev musl-tools git
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
pip install uv maturin

uv venv
source .venv/bin/activate

# development
maturin develop --uv

# build wheels
maturin build --release
```

## Benchmark

Outperforms `requests`, `httpx`, `aiohttp`, and `curl_cffi` according to our [benchmark](https://github.com/0x676e67/wreq-python/tree/main/bench) suite driven by [pyperf](https://github.com/psf/pyperf), though results are for reference only as they vary by environment.

## Services

Help sustain the ongoing development of this open-source project by reaching out for [commercial support](mailto:gngppz@gmail.com). Receive private guidance, expert reviews, or direct access to the maintainer, with personalized technical assistance tailored to your needs.

## License

Licensed under either of Apache License, Version 2.0 ([LICENSE](./LICENSE) or http://www.apache.org/licenses/LICENSE-2.0).

## Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted for inclusion in the work by you, as defined in the [Apache-2.0](./LICENSE) license, shall be licensed as above, without any additional terms or conditions.

## Sponsors

<a href="https://scrape.do/?utm_source=github&utm_medium=rnet" target="_blank">
  <img src="https://raw.githubusercontent.com/0x676e67/wreq-python/main/.github/assets/scrapedo.svg" width="149" height="47">
</a>

**[Scrape.do](https://scrape.do/?utm_source=github&utm_medium=rnet)** is the ultimate toolkit for collecting public data at scale. Unmatched speed, unbeatable prices, unblocked access.

One line of code. Instant data access

**Automatic Proxy Rotation** • **Bypass Anti-bot Solutions**  • **Seamless Web Scraping**

**[Dashboard](https://dashboard.scrape.do/login)** | **[Linkedin](https://www.linkedin.com/company/scrape-do/)** | **[Docs](https://scrape.do/documentation)**

---

<a href="https://www.ez-captcha.com" target="_blank">
  <img src="https://www.ez-captcha.com/siteLogo.png" height="47" width="47">
</a>

Captcha solving can be slow and unreliable, but **[EzCaptcha](https://www.ez-captcha.com/?r=github-rnet)** delivers fast, reliable solving through a simple API — supporting a wide range of captcha types with no complex integration required.  

**ReCaptcha** • **FunCaptcha** • **CloudFlare** • **Akamai** • **AkamaiSbsd** • **HCaptcha**  

Designed for developers, it offers high accuracy, low price, low latency, and easy integration, helping you automate verification while keeping traffic secure and user flows smooth.

**[Dashboard](https://www.ez-captcha.com/?r=github-rnet)** | **[Docs](https://ezcaptcha.atlassian.net/wiki/spaces/IS/pages/7045121/EzCaptcha+API+Docs+English)** | **[Telegram](https://t.me/+NrVmPhlb9ZFkZGY5)**

---

<a href="https://hypersolutions.co/?utm_source=github&utm_medium=readme&utm_campaign=rnet" target="_blank"><img src="https://raw.githubusercontent.com/0x676e67/wreq-python/main/.github/assets/hypersolutions.jpg" height="47" width="149"></a>

TLS fingerprinting alone isn't enough for modern bot protection. **[Hyper Solutions](https://hypersolutions.co?utm_source=github&utm_medium=readme&utm_campaign=rnet)** provides the missing piece - API endpoints that generate valid antibot tokens for:

**Akamai** • **DataDome** • **Kasada** • **Incapsula**

No browser automation. Just simple API calls that return the exact cookies and headers these systems require.

**[Dashboard](https://hypersolutions.co?utm_source=github&utm_medium=readme&utm_campaign=rnet)** | **[Docs](https://docs.justhyped.dev)** | **[Discord](https://discord.gg/akamai)**
