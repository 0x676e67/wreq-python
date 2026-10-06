"""Static, zero-baseline SVG charts from a validated benchmark snapshot."""

from decimal import Decimal, ROUND_CEILING
import html
import json
from pathlib import Path

if __package__:
    from .registry import CAPABILITIES, SPECS
    from .report import LABELS, payload_label
    from .results import validate_document
else:
    from registry import CAPABILITIES, SPECS
    from report import LABELS, payload_label
    from results import validate_document


PALETTES = {
    "light": {
        "background": "#ffffff",
        "text": "#172033",
        "muted": "#58677c",
        "grid": "#dce3ed",
        "track": "#edf1f8",
        "wreq": "#4657d9",
        "wreq_st": "#0f8a76",
        "wreq_ct": "#b45309",
        "peer": "#8393aa",
    },
    "dark": {
        "background": "#111827",
        "text": "#eef2f7",
        "muted": "#b2bfd2",
        "grid": "#2b3a50",
        "track": "#223047",
        "wreq": "#a0aaff",
        "wreq_st": "#55ccb3",
        "wreq_ct": "#f6b14b",
        "peer": "#8b9db8",
    },
}


def _escape(value):
    return html.escape(str(value), quote=True)


def _case_key(api, protocol, body_kind, concurrency, payload_bytes):
    return f"{api}-{protocol}-{body_kind}-c{concurrency}-b{payload_bytes}"


def _nice_axis(maximum, intervals=4):
    """Round upwards to readable decimal ticks, without changing measured rates."""
    maximum = Decimal(str(maximum)) if maximum else Decimal(1)
    rough = maximum / intervals
    magnitude = Decimal(10) ** rough.adjusted()
    step = next(
        value * magnitude
        for value in map(Decimal, ("1", "2", "2.5", "5", "10"))
        if value * magnitude >= rough
    )
    limit = (maximum / step).to_integral_value(rounding=ROUND_CEILING) * step
    ticks = [step * index for index in range(int(limit / step) + 1)]
    return limit, ticks


def _tick_label(value):
    if value == value.to_integral_value():
        return f"{value:,.0f}"
    return format(value, "f").rstrip("0").rstrip(".")


def _color(client, palette):
    return palette[SPECS[client].color]


def _text(x, y, value, *, size, color, anchor="start", attributes=""):
    return (
        f'<text x="{x:.2f}" y="{y:.2f}" font-size="{size}" '
        f'fill="{color}" text-anchor="{anchor}" {attributes}>'
        f"{_escape(value)}</text>"
    )


def _render_svg(document, case, rows, theme, mobile):
    palette = PALETTES[theme]
    width = 440 if mobile else 900
    padding = 24 if mobile else 32
    plot_x = padding if mobile else 236
    value_x = width - padding
    plot_width = value_x - plot_x - (104 if mobile else 118)
    row_start = 112 if mobile else 132
    row_height = 67 if mobile else 48
    bar_height = 18 if mobile else 26
    axis_y = row_start + len(rows) * row_height + 8
    height = axis_y + 99
    maximum = max((row["rps"] or 0 for row in rows), default=0)
    axis_max, ticks = _nice_axis(maximum, 3 if mobile else 4)
    protocol = "HTTP/1.1" if case["protocol"] == "h1" else "HTTP/2"
    body = payload_label(case["payload_bytes"])
    api = "Async" if case["api"] == "async" else "Blocking"
    heading = f"{body} · {api} · {protocol}"
    subtitle = f"{case['body_kind'].title()} upload · Concurrency {case['concurrency']}"
    description = "; ".join(
        (
            f"{row['label']}: {row['rps']:,.1f} requests/s"
            if row["rps"] is not None
            else f"{row['label']}: unsupported"
        )
        for row in rows
    )
    metadata = {
        "source": document["source"],
        "generated_at": document["generated_at"],
        "environment": document["environment"],
        "client_versions": {
            client: value["version"] for client, value in document["clients"].items()
        },
    }
    parts = [
        '<svg xmlns="http://www.w3.org/2000/svg" '
        f'width="{width}" height="{height}" viewBox="0 0 {width} {height}" '
        'role="img" aria-labelledby="chart-title chart-description" '
        'font-family="Inter, Segoe UI, Arial, sans-serif">',
        f'<title id="chart-title">{_escape(case["title"])}</title>',
        f'<desc id="chart-description">{_escape(description)}</desc>',
        f"<metadata>{_escape(json.dumps(metadata, ensure_ascii=False))}</metadata>",
        f'<rect width="{width}" height="{height}" rx="12" '
        f'fill="{palette["background"]}"/>',
        _text(
            padding,
            35 if mobile else 45,
            heading,
            size=21 if mobile else 28,
            color=palette["text"],
            attributes='font-weight="700"',
        ),
        _text(
            padding,
            62 if mobile else 77,
            subtitle,
            size=17 if mobile else 20,
            color=palette["text"],
        ),
        _text(
            padding,
            86 if mobile else 105,
            "requests/s · Higher is better",
            size=14 if mobile else 16,
            color=palette["muted"],
        ),
        f'<g class="axis" data-min="0" data-max="{axis_max}">',
    ]
    for tick in ticks:
        x = plot_x + float(tick / axis_max) * plot_width
        parts += [
            f'<g class="tick" data-value="{tick}">',
            f'<line x1="{x:.2f}" x2="{x:.2f}" y1="{row_start - 6}" '
            f'y2="{axis_y}" stroke="{palette["grid"]}" '
            f'stroke-width="{2 if tick == 0 else 1}"/>',
            _text(
                x,
                axis_y + 24,
                _tick_label(tick),
                size=13 if mobile else 15,
                color=palette["muted"],
                anchor="middle",
            ),
            "</g>",
        ]
    parts += [
        f'<line x1="{plot_x}" x2="{plot_x + plot_width}" '
        f'y1="{axis_y}" y2="{axis_y}" stroke="{palette["muted"]}"/>',
        "</g>",
    ]
    for index, row in enumerate(rows):
        y = row_start + index * row_height
        client = _escape(row["client"])
        status = "measured" if row["rps"] is not None else "unsupported"
        label_y = y + (18 if mobile else 20)
        bar_y = y + (29 if mobile else 0)
        value_y = y + (44 if mobile else 20)
        parts += [
            f'<g class="client-row" data-client="{client}" data-status="{status}">',
            _text(
                padding,
                label_y,
                row["label"],
                size=18,
                color=palette["text"],
                attributes=f'class="label" data-client="{client}"',
            ),
        ]
        if row["rps"] is None:
            parts.append(
                _text(
                    value_x,
                    value_y,
                    "N/A: unsupported",
                    size=17,
                    color=palette["muted"],
                    anchor="end",
                    attributes=f'class="value" data-client="{client}"',
                )
            )
        else:
            bar_width = float(Decimal(str(row["rps"])) / axis_max) * plot_width
            parts += [
                f'<rect class="track" x="{plot_x}" y="{bar_y}" '
                f'width="{plot_width}" height="{bar_height}" rx="4" '
                f'fill="{palette["track"]}"/>',
                f'<rect class="bar" data-client="{client}" '
                f'data-rps="{row["rps"]!r}" x="{plot_x}" y="{bar_y}" '
                f'width="{bar_width:.4f}" height="{bar_height}" rx="4" '
                f'fill="{_color(row["client"], palette)}"/>',
                _text(
                    value_x,
                    value_y,
                    f"{row['rps']:,.1f}",
                    size=20,
                    color=palette["text"],
                    anchor="end",
                    attributes=f'class="value" data-client="{client}"',
                ),
            ]
        parts.append("</g>")
    parts += [
        _text(
            padding,
            axis_y + 53,
            "Zero baseline · N/A means unsupported",
            size=13 if mobile else 15,
            color=palette["muted"],
        ),
        _text(
            padding,
            axis_y + 78,
            f"Revision {document['source']['commit'][:12]} · "
            f"{document['generated_at'][:10]}"
            + (" · local changes" if document["source"]["dirty"] else ""),
            size=12 if mobile else 14,
            color=palette["muted"],
        ),
        "</svg>",
    ]
    return "\n".join(parts) + "\n"


def write_charts(document: dict, directory: Path, *, api: str | None = None) -> dict:
    """Write four SVG variants per configured case, without deleting any files."""
    validate_document(document)
    if api not in (None, "async", "blocking"):
        raise ValueError("Chart API must be async or blocking")
    apis = (api,) if api is not None else ("async", "blocking")
    config = document["configuration"]
    ordered = [client for client in LABELS if client in config["clients"]]
    ordered += [client for client in config["clients"] if client not in LABELS]
    indexed = {
        (
            row["client"],
            row["protocol"],
            row["body_kind"],
            row["concurrency"],
            row["payload_bytes"],
        ): row["rps"]
        for row in document["results"]
    }
    directory = Path(directory)
    directory.mkdir(parents=True, exist_ok=True)
    cases = []
    for api in apis:
        clients = [client for client in ordered if CAPABILITIES[client]["api"] == api]
        if not clients:
            continue
        for protocol in ("h1", "h2"):
            if protocol not in config["protocols"]:
                continue
            for kind in ("full", "stream"):
                if kind not in config["body_kinds"]:
                    continue
                for concurrency in sorted(config["concurrency"]):
                    for size in sorted(config["payload_bytes"]):
                        key = _case_key(api, protocol, kind, concurrency, size)
                        protocol_label = "HTTP/1.1" if protocol == "h1" else "HTTP/2"
                        title = (
                            f"{payload_label(size)} body · {api.title()} API · "
                            f"{protocol_label} · {kind.title()} upload · "
                            f"Concurrency {concurrency} · requests/s"
                        )
                        rows = [
                            {
                                "client": client,
                                "label": LABELS.get(client, client),
                                "rps": indexed.get(
                                    (client, protocol, kind, concurrency, size)
                                ),
                            }
                            for client in clients
                        ]
                        case = {
                            "key": key,
                            "api": api,
                            "protocol": protocol,
                            "body_kind": kind,
                            "payload_bytes": size,
                            "concurrency": concurrency,
                            "title": title,
                            "rows": [row for row in rows if row["rps"] is not None],
                            "unsupported": [
                                row["label"] for row in rows if row["rps"] is None
                            ],
                            "assets": {},
                        }
                        for theme in PALETTES:
                            case["assets"][theme] = {}
                            for layout, mobile in (
                                ("desktop", False),
                                ("mobile", True),
                            ):
                                filename = (
                                    f"{key}-{theme}{'-mobile' if mobile else ''}.svg"
                                )
                                output = directory / filename
                                if output.is_symlink():
                                    raise ValueError("Chart output cannot be a symlink")
                                output.write_text(
                                    _render_svg(document, case, rows, theme, mobile),
                                    encoding="utf-8",
                                )
                                case["assets"][theme][layout] = filename
                        cases.append(case)
    return {
        "schema_version": 1,
        "source": {
            "commit": document["source"]["commit"],
            "dirty": document["source"]["dirty"],
        },
        "generated_at": document["generated_at"],
        "cases": cases,
    }
