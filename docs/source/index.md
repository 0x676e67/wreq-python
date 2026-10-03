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

# Python HTTP.<br>Rust at the core.

Browser profiles, streaming transfers, and the HTTP tools you use every day. A native Rust engine behind an async and blocking Python API.
{ .wreq-lead }

<div class="wreq-actions" markdown="1">

[Get started](getting-started/quickstart.md){ .md-button .md-button--primary }
[View on GitHub](https://github.com/0x676e67/wreq-python){ .md-button }

</div>

<p class="wreq-install"><code>pip install wreq</code><span>Python 3.11+ · Apache-2.0</span></p>

</div>

<div class="wreq-example" markdown="1">

<div class="wreq-example-heading"><span>One client. Your next request.</span><span>Python</span></div>

```python
import asyncio
from wreq import Client, Emulation


async def main():
    async with Client(
        emulation=Emulation.Chrome154,
    ) as client:
        async with await client.get(
            "https://example.com"
        ) as response:
            print(await response.text())


asyncio.run(main())
```

<p class="wreq-example-footer">Reuse your client. Keep your connections.</p>

</div>

</section>

<section class="wreq-sponsors" aria-labelledby="sponsor-heading" data-sponsors>
<div class="wreq-section-heading">
  <div><p class="wreq-eyebrow">SUPPORTED BY OUR SPONSORS</p><h2 id="sponsor-heading">Thanks to our sponsors.</h2></div>
  <div class="wreq-carousel-controls" hidden>
    <button type="button" data-sponsor-previous aria-label="Previous sponsors">←</button>
    <button type="button" data-sponsor-pause aria-pressed="false">Pause</button>
    <button type="button" data-sponsor-next aria-label="Next sponsors">→</button>
  </div>
</div>
<div class="wreq-sponsor-window" tabindex="0" role="region" aria-label="Project sponsors; scroll horizontally to see all sponsors" data-sponsor-window>
  <a class="wreq-sponsor" href="https://byteful.com/?utm_source=github_python&amp;utm_medium=github-sponsor&amp;utm_campaign=wreq_github_sponsorship" target="_blank" rel="sponsored noopener noreferrer"><img src="assets/sponsors/byteful-logo.svg" alt="Byteful" width="149" height="47"></a>
  <a class="wreq-sponsor" href="https://go.nodemaven.com/wreqpythonGHaugust" target="_blank" rel="sponsored noopener noreferrer"><img src="assets/sponsors/nodemaven.svg" alt="NodeMaven" width="165" height="47"></a>
  <a class="wreq-sponsor" href="https://scrape.do/?utm_source=github&amp;utm_medium=wreq" target="_blank" rel="sponsored noopener noreferrer"><img src="assets/sponsors/scrapedo.svg" alt="Scrape.do" width="149" height="47"></a>
  <a class="wreq-sponsor" href="https://www.ez-captcha.com/?r=github-wreq" target="_blank" rel="sponsored noopener noreferrer"><img src="assets/sponsors/ezcaptcha.svg" alt="EzCaptcha" width="47" height="47"><span>EzCaptcha</span></a>
  <a class="wreq-sponsor" href="https://hypersolutions.co/?utm_source=github&amp;utm_medium=readme&amp;utm_campaign=wreq" target="_blank" rel="sponsored noopener noreferrer"><img src="assets/sponsors/hypersolutions.jpg" alt="Hyper Solutions" width="149" height="47"></a>
</div>
<p class="wreq-sponsor-note"><a href="sponsors/">Meet our sponsors</a> · <a href="mailto:gngppz@gmail.com">Support the project</a></p>
</section>

<section class="wreq-features" markdown="1">

<p class="wreq-eyebrow">MORE CONTROL, WITHOUT A BROWSER PROCESS</p>

## Familiar Python. Control where it matters.

<div class="wreq-feature-grid" markdown="0">

<a class="wreq-feature" href="guide/emulation/"><span class="wreq-feature-label">01 · EMULATION</span><h3>Choose a browser profile</h3><p>Configure TLS and HTTP/2 behavior with browser and platform profiles. Go beyond changing the User-Agent.</p><span class="wreq-feature-link">Explore emulation ↗</span></a>

<a class="wreq-feature" href="guide/basic/"><span class="wreq-feature-label">02 · HTTP</span><h3>The essentials, included</h3><p>JSON, forms, multipart uploads, cookies, redirects, proxies, and reusable connection pools.</p><span class="wreq-feature-link">Make your first request ↗</span></a>

<a class="wreq-feature" href="guide/advanced/"><span class="wreq-feature-label">03 · STREAMING</span><h3>Work with bytes as they arrive</h3><p>Stream uploads and responses. Read Rust-backed response buffers through read-only Python memoryviews.</p><span class="wreq-feature-link">Read the streaming guide ↗</span></a>

<a class="wreq-feature" href="guide/blocking/"><span class="wreq-feature-label">04 · PYTHON</span><h3>Async or blocking</h3><p>Use await in asynchronous applications, or the blocking client in synchronous code. Keep the same HTTP building blocks.</p><span class="wreq-feature-link">Use the blocking API ↗</span></a>

<a class="wreq-feature" href="api/runtime/"><span class="wreq-feature-label">05 · RUNTIME</span><h3>Choose your runtime</h3><p>Share the global runtime or give a client a custom Tokio runtime, including a single-worker configuration.</p><span class="wreq-feature-link">Configure a runtime ↗</span></a>

<a class="wreq-feature" href="guide/websocket/"><span class="wreq-feature-label">06 · WEBSOCKET</span><h3>Keep the conversation open</h3><p>Upgrade to a WebSocket connection and exchange text or binary frames through the client API.</p><span class="wreq-feature-link">Connect a WebSocket ↗</span></a>

</div>

</section>

<section class="wreq-benchmark-callout" markdown="1">

<p class="wreq-eyebrow">HTTPS BENCHMARKS</p>

## Performance you can inspect.

Our HTTPS benchmarks compare HTTP/1.1 and HTTP/2, complete and streamed uploads, and explicit runtime configurations. Every response is consumed to EOF. See the tested commit, environment, repeated measurements, and raw JSON alongside the results.

[Explore the benchmarks](benchmark.md){ .md-button }

</section>

<section class="wreq-home-footer" markdown="1">

## Start small. Go deeper when you need to.

[Install wreq](getting-started/installation.md) · [Read the guides](guide/basic.md) · [Browse the API](api/wreq.md) · [Join the community](https://discord.gg/rfbvyFkgq3)

Browser profiles configure network behavior. They do not execute JavaScript or guarantee access to a protected website.
{ .wreq-fine-print }

</section>

</div>
