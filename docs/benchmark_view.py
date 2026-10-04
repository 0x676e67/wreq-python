"""Render benchmark charts with filters and a static image fallback."""

from datetime import datetime, timezone
import html
import json

from bench.report import REPOSITORY, payload_label, safe_data_link


def escape(value):
    return html.escape(str(value), quote=True)


def render_explorer(document, catalog, asset_prefix="assets/benchmark/charts"):
    cases = catalog["cases"]
    first = next((case for case in cases if case["api"] == "async"), cases[0])
    payloads = sorted({case["payload_bytes"] for case in cases})
    labels = {
        "api": {"async": "Async clients", "blocking": "Blocking clients"},
        "protocol": {"h1": "HTTP/1.1", "h2": "HTTP/2"},
        "body_kind": {"full": "Full upload", "stream": "Stream upload"},
    }
    controls = []
    for field, name in (
        ("api", "Client API"),
        ("protocol", "Protocol"),
        ("body_kind", "Request body"),
        ("concurrency", "Concurrency"),
    ):
        options = []
        for value in sorted({case[field] for case in cases}):
            label = labels.get(field, {}).get(value, str(value))
            selected = " selected" if value == first[field] else ""
            options.append(
                f'<option value="{escape(value)}"{selected}>{escape(label)}</option>'
            )
        controls.append(
            f'<label>{name}<select data-chart-select="{field}">{"".join(options)}</select></label>'
        )
    buttons = "".join(
        f'<button type="button" data-chart-payload="{size}" aria-pressed="{str(size == first["payload_bytes"]).lower()}">{payload_label(size)}</button>'
        for size in payloads
    )
    assets = first["assets"]["light"]
    image = safe_data_link(f'{asset_prefix}/{assets["desktop"]}')
    mobile = safe_data_link(f'{asset_prefix}/{assets["mobile"]}')
    rows = "".join(
        f'<tr><td>{escape(row["label"])}</td><td>{row["rps"]:,.1f}</td></tr>'
        for row in first["rows"]
    ) + "".join(
        f"<tr><td>{escape(label)}</td><td>N/A</td></tr>"
        for label in first["unsupported"]
    )
    # This is inert JSON, not script. Escape HTML delimiters so metadata cannot end the tag.
    data = json.dumps(catalog, ensure_ascii=True, separators=(",", ":"))
    data = data.replace("<", "\\u003c").replace(">", "\\u003e").replace("&", "\\u0026")
    source = document["source"]
    date = datetime.fromisoformat(document["generated_at"]).astimezone(timezone.utc)
    dirty = " · Local checkout had uncommitted changes" if source["dirty"] else ""
    unsupported = (
        "N/A: " + ", ".join(first["unsupported"])
        if first["unsupported"]
        else "All shown clients support this case."
    )
    return f"""<section class="wreq-bench" data-bench-explorer aria-label="Benchmark chart explorer">
<p class="wreq-bench-provenance">Measured <a href="{REPOSITORY}/commit/{source['commit']}">{source['commit'][:12]}</a> · {date:%Y-%m-%d %H:%M UTC}{dirty}</p>
<div class="wreq-bench-heading"><div><p class="wreq-bench-eyebrow">THROUGHPUT BY WORKLOAD</p><h3>Compare clients</h3></div><a href="assets/benchmark/latest.json">Raw JSON</a></div>
<p class="wreq-bench-intro">Pick a body size, then adjust the filters to compare clients. Full and Stream refer to the upload; we read every response to the end.</p>
<div class="wreq-bench-body-controls" data-chart-controls hidden>
<button type="button" data-chart-previous aria-label="Previous body size">←</button>
<div class="wreq-bench-payloads" role="group" aria-label="Upload and echo body size">{buttons}</div>
<button type="button" data-chart-next aria-label="Next body size">→</button>
</div>
<div class="wreq-bench-filters" data-chart-controls hidden>{''.join(controls)}</div>
<p class="wreq-bench-selection" data-chart-caption aria-live="polite">{escape(first['title'])}</p>
<figure class="wreq-bench-figure">
<picture><source media="(max-width: 600px)" data-chart-mobile srcset="{mobile}"><img data-chart-image src="{image}" alt="{escape(first['title'])}. Throughput in requests per second; values in the table below." width="900" decoding="async"></picture>
<figcaption><span data-chart-unsupported>{escape(unsupported)}</span><a data-chart-download href="{image}" download>Download SVG</a></figcaption>
</figure>
<p class="wreq-bench-note">Unit: requests/s (RPS). Higher is better. Each chart uses its own zero-based linear scale; compare the numbers, not bar lengths across different charts.</p>
<details class="wreq-bench-values"><summary>Values for this chart</summary><table><thead><tr><th>Client / runtime</th><th>Requests/s</th></tr></thead><tbody data-chart-values>{rows}</tbody></table></details>
<noscript><p>Chart switching needs JavaScript. The default chart and complete tables below remain available.</p></noscript>
<script type="application/json" data-chart-catalog>{data}</script>
</section>"""
