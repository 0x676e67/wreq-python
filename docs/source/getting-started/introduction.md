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


## Why wreq?


1. When your **HTTP** requests succeed in a browser but get blocked in Python due to network fingerprint issues, this tool bridges the gap. [wreq] allows you to customize your **TLS**, **JA3/JA4**, and **HTTP/2** fingerprints to mimic real browsers, making it ideal for web scraping, penetration testing, and security research.

2. The standard **HTTP Client**, such as [requests] and [httpx], have different network fingerprints from browsers. The main differences lie in **TLS handshake**, **HTTP/2 frame characteristics**, and **JA3/JA4** fingerprints. Browsers use specific encryption suites and extensions in the TLS handshake, while standard HTTP clients may use different default settings, causing servers to recognize and block these requests.

3. [wreq] uses the **BoringSSL** library, which is fully sufficient to set TLS fingerprints that match mainstream browsers while maintaining native performance.

4. In addition, the basic functions of [wreq] are similar to those of the standard **HTTP Client**, offering a wide range of features such as connection pooling, redirection policies, Cookie storage, and streaming transmission, which can meet various complex HTTP request requirements.


## Behavior

1. **HTTP/2 over TLS**

Due to the complexity of **TLS** encryption and the widespread adoption of **HTTP/2**, browser fingerprints such as **JA3**, **JA4**, and **Akamai** cannot be reliably emulated using simple fingerprint strings. Instead of parsing and emulating these string-based fingerprints, [wreq] provides fine-grained control over **TLS** and **HTTP/2** extensions and settings for precise browser behavior emulation.

2. **Device Emulation**

**TLS** and **HTTP/2** fingerprints are often identical across various browser models because these underlying protocols evolve slower than browser release cycles. **100+ browser device emulation profiles** are maintained in **[wreq]**.

???+ note "Available OS emulations"

    <div class="grid cards" markdown>

    -   :fontawesome-brands-windows: **Windows**
    -   :fontawesome-brands-apple: **macOS**
    -   :fontawesome-brands-linux: **Linux**
    -   :fontawesome-brands-android: **Android**
    -   :fontawesome-brands-apple: **iOS**

    </div>

???+ note "Available browser emulations"

    <div class="grid cards" markdown>

    -   :fontawesome-brands-chrome: **Chrome**

        ---

        `Chrome100` `Chrome101` `Chrome104` `Chrome105` `Chrome106` `Chrome107` `Chrome108` `Chrome109` `Chrome110` `Chrome114` `Chrome116` `Chrome117` `Chrome118` `Chrome119` `Chrome120` `Chrome123` `Chrome124` `Chrome126` `Chrome127` `Chrome128` `Chrome129` `Chrome130` `Chrome131` `Chrome132` `Chrome133` `Chrome134` `Chrome135` `Chrome136` `Chrome137` `Chrome138` `Chrome139` `Chrome140` `Chrome141` `Chrome142` `Chrome143` `Chrome144` `Chrome145` `Chrome146` `Chrome147` `Chrome148` `Chrome149`

    -   :fontawesome-brands-edge: **Edge**

        ---

        `Edge101` `Edge122` `Edge127` `Edge131` `Edge134` `Edge135` `Edge136` `Edge137` `Edge138` `Edge139` `Edge140` `Edge141` `Edge142` `Edge143` `Edge144` `Edge145` `Edge146` `Edge147` `Edge148`

    -   :fontawesome-brands-firefox: **Firefox**

        ---

        `Firefox109` `Firefox117` `Firefox128` `Firefox133` `Firefox135` `FirefoxPrivate135` `FirefoxAndroid135` `Firefox136` `FirefoxPrivate136` `Firefox139` `Firefox142` `Firefox143` `Firefox144` `Firefox145` `Firefox146` `Firefox147` `Firefox148` `Firefox149` `Firefox150` `Firefox151`

    -   :fontawesome-brands-safari: **Safari**

        ---

        `SafariIos17_2` `SafariIos17_4_1` `SafariIos16_5` `Safari15_3` `Safari15_5` `Safari15_6_1` `Safari16` `Safari16_5` `Safari17_0` `Safari17_2_1` `Safari17_4_1` `Safari17_5` `Safari17_6` `Safari18` `SafariIPad18` `Safari18_2` `Safari18_3` `Safari18_3_1` `SafariIos18_1_1` `Safari18_5` `Safari26` `Safari26_1` `Safari26_2` `Safari26_3` `Safari26_4` `SafariIos26` `SafariIos26_2` `SafariIPad26` `SafariIpad26_2`

    -   :fontawesome-brands-opera: **Opera**

        ---

        `Opera116` `Opera117` `Opera118` `Opera119` `Opera120` `Opera121` `Opera122` `Opera123` `Opera124` `Opera125` `Opera126` `Opera127` `Opera128` `Opera129` `Opera130` `Opera131`

    -   :material-web: **OkHttp**

        ---

        `OkHttp3_9` `OkHttp3_11` `OkHttp3_13` `OkHttp3_14` `OkHttp4_9` `OkHttp4_10` `OkHttp4_12` `OkHttp5`

    </div>


## License

Licensed under either of Apache License, Version 2.0 ([LICENSE](./LICENSE) or http://www.apache.org/licenses/LICENSE-2.0).

## Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted for inclusion in the work by you, as defined in the [Apache-2.0](./LICENSE) license, shall be licensed as above, without any additional terms or conditions.


[wreq]: https://github.com/0x676e67/wreq-python
[requests]: https://github.com/psf/requests
[httpx]: https://github.com/encode/httpx
[Buffer]: https://docs.python.org/3/c-api/buffer.html
[GIL]: https://docs.python.org/3/c-api/init.html#thread-and-gil-management
