---
title: wreq — Python HTTP, powered by Rust
hide:
  - navigation
  - toc
---

<div class="wreq-home" markdown="1">

<section class="wreq-hero" markdown="1">

<div class="wreq-hero-copy" markdown="1">

<p class="wreq-eyebrow">WREQ · PYTHON HTTP CLIENT</p>

# HTTP for Python,<br>powered by Rust.

wreq is a Python HTTP client built on Rust. Use it with async or blocking code, choose a browser profile, or tune TLS and HTTP/2 yourself. You can stream uploads and responses when you don't want the whole body in memory.
{ .wreq-lead }

<div class="wreq-actions" markdown="1">

[Get started](getting-started/quickstart.md){ .md-button .md-button--primary }
[View on GitHub](https://github.com/0x676e67/wreq-python){ .md-button }

</div>

<p class="wreq-install"><code>pip install wreq</code><span>Python 3.11+ · Apache-2.0</span></p>

</div>

<div class="wreq-example" markdown="1">

<div class="wreq-example-heading"><span>Make a request</span><span>Python</span></div>

```python
import asyncio
from wreq import Client, Emulation


async def main():
    async with Client(
        emulation=Emulation.Chrome154,
    ) as client:
        async with client.get(
            "https://example.com"
        ) as response:
            print(await response.text())


asyncio.run(main())
```

<p class="wreq-example-footer">Keep the client open to reuse connections across requests.</p>

</div>

</section>

<section class="wreq-sponsors" aria-labelledby="sponsor-heading" data-sponsors>
<div class="wreq-section-heading">
  <div><p class="wreq-eyebrow">SUPPORTED BY OUR SPONSORS</p><h2 id="sponsor-heading">Thanks to our sponsors.</h2></div>
</div>
<div class="wreq-sponsor-window" tabindex="0" role="region" aria-label="Project sponsors; focus or hover to pause automatic scrolling" data-sponsor-window>
  <a class="wreq-sponsor" href="https://byteful.com/?utm_source=github_python&amp;utm_medium=github-sponsor&amp;utm_campaign=wreq_github_sponsorship" target="_blank" rel="sponsored noopener noreferrer"><img src="assets/sponsors/byteful-logo.svg" alt="Byteful" width="149" height="47"></a>
  <a class="wreq-sponsor" href="https://go.nodemaven.com/wreqpythonGHaugust" target="_blank" rel="sponsored noopener noreferrer"><img src="assets/sponsors/nodemaven.svg" alt="NodeMaven" width="165" height="47"></a>
  <a class="wreq-sponsor" href="https://scrape.do/?utm_source=github&amp;utm_medium=wreq" target="_blank" rel="sponsored noopener noreferrer"><img src="assets/sponsors/scrapedo.svg" alt="Scrape.do" width="149" height="47"></a>
  <a class="wreq-sponsor" href="https://www.ez-captcha.com/?r=github-wreq" target="_blank" rel="sponsored noopener noreferrer"><img src="assets/sponsors/ezcaptcha.svg" alt="EzCaptcha" width="47" height="47"><span>EzCaptcha</span></a>
  <a class="wreq-sponsor" href="https://hypersolutions.co/?utm_source=github&amp;utm_medium=readme&amp;utm_campaign=wreq" target="_blank" rel="sponsored noopener noreferrer"><img src="assets/sponsors/hypersolutions.jpg" alt="Hyper Solutions" width="149" height="47"></a>
</div>
<p class="wreq-sponsor-note"><a href="sponsors/">Meet our sponsors</a> · <a href="mailto:gngppz@gmail.com">Support the project</a></p>
</section>

<section class="wreq-features" markdown="1">

<p class="wreq-eyebrow">CLIENT FEATURES</p>

## What you can do with wreq

<div class="wreq-feature-grid" markdown="0">

<a class="wreq-feature" href="guide/emulation/"><span class="wreq-feature-label">01 · EMULATION</span><h3>Choose a browser profile</h3><p>Pick a browser and platform profile to configure TLS, HTTP/2 and the User-Agent together.</p><span class="wreq-feature-link">Emulation guide</span></a>

<a class="wreq-feature" href="benchmark/"><span class="wreq-feature-label">02 · PERFORMANCE</span><h3>High-throughput HTTPS</h3><p>Rust handles the network I/O. See how wreq compares with other Python clients, then choose the body size and runtime mode that matter to your workload.</p><span class="wreq-feature-link">View performance results</span></a>

<a class="wreq-feature" href="guide/advanced/"><span class="wreq-feature-label">03 · STREAMING</span><h3>Stream uploads and responses</h3><p>Send an upload in chunks or read a response as it arrives. Response chunks are read-only memoryviews backed by Rust buffers.</p><span class="wreq-feature-link">Streaming guide</span></a>

<a class="wreq-feature" href="guide/blocking/"><span class="wreq-feature-label">04 · PYTHON</span><h3>Async and blocking APIs</h3><p>Use await in async code. For a synchronous script, import Client from wreq.blocking.</p><span class="wreq-feature-link">Blocking API</span></a>

<a class="wreq-feature" href="api/runtime/"><span class="wreq-feature-label">05 · RUNTIME</span><h3>Choose how the client runs</h3><p>Clients share a global Tokio runtime by default. Give a client its own runtime when you need separate resources or a single worker.</p><span class="wreq-feature-link">Runtime options</span></a>

<a class="wreq-feature" href="guide/websocket/"><span class="wreq-feature-label">06 · WEBSOCKET</span><h3>Connect with WebSockets</h3><p>Open a WebSocket connection through the client and send or receive text and binary frames.</p><span class="wreq-feature-link">WebSocket guide</span></a>

</div>

</section>

<section class="wreq-benchmark-callout" markdown="1">

<p class="wreq-eyebrow">HTTPS BENCHMARKS</p>

## See the benchmark results

Choose a body size, protocol and concurrency level to see the results for your workload. We test complete and streamed uploads over HTTP/1.1 and HTTP/2, reading every response to the end. The tested commit, machine details and raw timings are there if you want to check or reproduce a result.

[Explore the benchmarks](benchmark.md){ .md-button }

</section>

<section class="wreq-home-footer" markdown="1">

## Documentation and community

[Install wreq](getting-started/installation.md) · [Read the guides](guide/basic.md) · [Browse the API](api/wreq.md) · [Join the Discord](https://discord.gg/rfbvyFkgq3)

Browser profiles configure network behavior. They do not execute JavaScript or guarantee access to a protected website.
{ .wreq-fine-print }

</section>

</div>
