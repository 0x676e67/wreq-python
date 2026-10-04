"""Static SVG exports preserve measured values, capabilities and metadata."""

import copy
from itertools import product
import json
import math
from pathlib import Path
import xml.etree.ElementTree as ET

import pytest

from bench.charts import write_charts
from bench.clients import CAPABILITIES
from bench.report import LABELS
from bench.tests.test_benchmark import make_document
from bench.workloads import BODY_CASES

SVG = {"svg": "http://www.w3.org/2000/svg"}
CASE_FIELDS = ("api", "protocol", "body_kind", "concurrency", "payload_bytes")


@pytest.fixture
def small_document():
    document = make_document(
        {
            "clients": ["ry", "requests", "wreq_st", "httpx", "wreq", "wreq_blocking"],
            "requests": 10000,
        }
    )
    for index, cell in enumerate(document["results"], 1):
        for sample in cell["samples"]:
            sample["seconds"] *= index
        for summary in (cell, *cell["rounds"]):
            summary["total_seconds"] *= index
            summary["rps"] /= index
            summary["mbps"] /= index
    return document


def asset_files(catalog):
    return [
        filename
        for case in catalog["cases"]
        for layouts in case["assets"].values()
        for filename in layouts.values()
    ]


def test_chart_catalog_numeric_values_and_responsive_layout(tmp_path, small_document):
    document = small_document
    catalog = write_charts(document, tmp_path)
    assert catalog["schema_version"] == 1
    assert catalog["source"] == {
        "commit": document["source"]["commit"],
        "dirty": document["source"]["dirty"],
    }
    assert catalog["generated_at"] == document["generated_at"]
    expected_cases = list(
        product(("async", "blocking"), ("h1", "h2"), ("full", "stream"), (2,), (10240,))
    )
    assert [
        tuple(case[field] for field in CASE_FIELDS) for case in catalog["cases"]
    ] == expected_cases
    filenames = asset_files(catalog)
    assert len(filenames) == len(set(filenames)) == 32
    assert set(filenames) == {path.name for path in tmp_path.glob("*.svg")}
    measured = {
        (
            cell["client"],
            cell["protocol"],
            cell["body_kind"],
            cell["concurrency"],
            cell["payload_bytes"],
        ): cell["rps"]
        for cell in document["results"]
    }
    for case in catalog["cases"]:
        key = (
            f"{case['api']}-{case['protocol']}-{case['body_kind']}"
            f"-c{case['concurrency']}-b{case['payload_bytes']}"
        )
        scenario = tuple(case[field] for field in CASE_FIELDS[1:])
        assert case["key"] == key and case["title"]
        group = [
            client
            for client in LABELS
            if client in document["clients"]
            and CAPABILITIES[client]["api"] == case["api"]
        ]
        expected_rows = []
        unsupported = []
        for client in group:
            rps = measured.get((client, *scenario))
            if rps is None:
                unsupported.append(LABELS[client])
            else:
                expected_rows.append(
                    {"client": client, "label": LABELS[client], "rps": rps}
                )
        assert case["rows"] == expected_rows
        assert case["unsupported"] == unsupported
        assert set(case["assets"]) == {"light", "dark"}
        for theme, layouts in case["assets"].items():
            assert set(layouts) == {"desktop", "mobile"}
            for layout, filename in layouts.items():
                suffix = "-mobile" if layout == "mobile" else ""
                assert filename == f"{key}-{theme}{suffix}.svg"
                assert Path(filename).name == filename
                root = ET.parse(tmp_path / filename).getroot()
                width = 440 if layout == "mobile" else 900
                assert root.tag == f"{{{SVG['svg']}}}svg"
                assert float(root.attrib["width"]) == width
                height = float(root.attrib["height"])
                assert list(map(float, root.attrib["viewBox"].split())) == [
                    0,
                    0,
                    width,
                    height,
                ]
                rows = root.findall(".//svg:g[@class='client-row']", SVG)
                assert [row.attrib["data-client"] for row in rows] == group
                colors = {}
                scales = []
                positions = []
                value_positions = []
                for row in rows:
                    client = row.attrib["data-client"]
                    label = row.find("svg:text[@class='label']", SVG)
                    assert label is not None
                    assert "".join(label.itertext()) == LABELS[client]
                    bar = row.find("svg:rect[@class='bar']", SVG)
                    value = row.find("svg:text[@class='value']", SVG)
                    rps = measured.get((client, *scenario))
                    if rps is None:
                        assert row.attrib["data-status"] == "unsupported"
                        assert bar is None
                        assert "N/A" in "".join(row.itertext())
                        assert "unsupported" in "".join(row.itertext())
                        continue
                    assert row.attrib["data-status"] == "measured"
                    assert bar is not None and value is not None
                    assert (
                        bar.attrib["data-client"]
                        == value.attrib["data-client"]
                        == client
                    )
                    assert float(bar.attrib["data-rps"]) == rps
                    assert "".join(value.itertext()) == f"{rps:,.1f}"
                    positions.append(float(bar.attrib["x"]))
                    scales.append(float(bar.attrib["width"]) / rps)
                    colors[client] = bar.attrib["fill"]
                    value_positions.append(float(value.attrib["x"]))
                    assert float(bar.attrib["y"]) + float(bar.attrib["height"]) < height
                    assert float(value.attrib["x"]) > float(bar.attrib["x"]) + float(
                        bar.attrib["width"]
                    )
                    if layout == "mobile":
                        assert float(label.attrib["y"]) < float(bar.attrib["y"])
                assert all(position == positions[0] for position in positions)
                assert all(
                    position == value_positions[0] for position in value_positions
                )
                assert all(
                    scale == pytest.approx(scales[0], rel=1e-3) for scale in scales
                )
                if "wreq" in colors:
                    assert colors["wreq"] != colors["wreq_st"]
                    assert colors["ry"] == colors["httpx"]
                    assert colors["ry"] not in (colors["wreq"], colors["wreq_st"])
                axis = root.find(".//svg:g[@class='axis']", SVG)
                assert axis is not None and float(axis.attrib["data-min"]) == 0
                maximum = float(axis.attrib["data-max"])
                assert maximum >= max((row["rps"] for row in expected_rows), default=0)
                ticks = axis.findall("svg:g[@class='tick']", SVG)
                values = [float(tick.attrib["data-value"]) for tick in ticks]
                assert len(values) >= 2 and values[0] == 0
                assert values[-1] == maximum
                assert values == sorted(set(values))
                assert all(
                    tick.find("svg:line", SVG) is not None
                    and tick.find("svg:text", SVG) is not None
                    for tick in ticks
                )
                step = values[1]
                assert all(
                    value == pytest.approx(index * step)
                    for index, value in enumerate(values)
                )
                normalized = step / 10 ** math.floor(math.log10(step))
                assert any(normalized == pytest.approx(nice) for nice in (1, 2, 2.5, 5))


def test_chart_metadata_is_escaped_inert_xml(tmp_path, small_document):
    document = small_document
    document["environment"]["cpu"] = 'CPU <script onload="bad()">& "quoted"</script>'
    document["clients"]["wreq"][
        "version"
    ] = '1.0 </metadata><foreignObject onload="bad()">&'
    document["source"]["dirty"] = True
    catalog = write_charts(document, tmp_path)
    assert catalog["source"]["dirty"] is True
    for filename in asset_files(catalog):
        text = (tmp_path / filename).read_text(encoding="utf-8")
        root = ET.fromstring(text)
        assert "&lt;script" in text and "&lt;/metadata&gt;" in text
        assert all(
            element.tag.rsplit("}", 1)[-1] not in {"script", "foreignObject"}
            for element in root.iter()
        )
        assert all(
            not attribute.lower().startswith("on")
            for element in root.iter()
            for attribute in element.attrib
        )
        metadata = root.find("svg:metadata", SVG)
        assert metadata is not None
        decoded = json.loads(metadata.text)
        assert decoded["environment"]["cpu"] == document["environment"]["cpu"]
        assert decoded["source"]["commit"] == document["source"]["commit"]
        assert decoded["source"]["dirty"] is True
        assert (
            decoded["client_versions"]["wreq"] == document["clients"]["wreq"]["version"]
        )


def test_invalid_input_writes_nothing_and_repeated_exports_preserve_files(tmp_path):
    document = make_document({"protocols": ["h1"], "body_kinds": ["full"]})
    for failure in ("schema", "incomplete", "numeric"):
        invalid = copy.deepcopy(document)
        if failure == "schema":
            invalid["schema_version"] = 2
        elif failure == "incomplete":
            invalid["results"].pop()
        else:
            invalid["results"][0]["rps"] = float("nan")
        destination = tmp_path / failure
        with pytest.raises(ValueError):
            write_charts(invalid, destination)
        assert not destination.exists() or not list(destination.rglob("*"))
    output = tmp_path / "valid"
    output.mkdir()
    unrelated = output / "keep.svg"
    unrelated.write_text("not a generated chart", encoding="utf-8")
    first = write_charts(document, output)
    contents = {name: (output / name).read_bytes() for name in asset_files(first)}
    second = write_charts(document, output)
    assert first == second
    assert len(first["cases"]) == 1 and first["cases"][0]["api"] == "async"
    assert contents == {
        name: (output / name).read_bytes() for name in asset_files(second)
    }
    assert unrelated.read_text(encoding="utf-8") == "not a generated chart"


def test_full_chart_matrix_generates_four_unique_assets_per_case(tmp_path):
    document = make_document(
        {
            "clients": list(LABELS),
            "payload_bytes": list(reversed(BODY_CASES)),
            "concurrency": [150, 10, 100, 50],
            "requests": 300,
        }
    )
    catalog = write_charts(document, tmp_path)
    expected = list(
        product(
            ("async", "blocking"),
            ("h1", "h2"),
            ("full", "stream"),
            (10, 50, 100, 150),
            sorted(BODY_CASES),
        )
    )
    assert [
        tuple(case[field] for field in CASE_FIELDS) for case in catalog["cases"]
    ] == expected
    assert len(catalog["cases"]) == 224
    filenames = asset_files(catalog)
    assert len(filenames) == len(set(filenames)) == 896
    assert set(filenames) == {path.name for path in tmp_path.glob("*.svg")}
    for filename in filenames:
        assert ET.parse(tmp_path / filename).getroot().tag == f"{{{SVG['svg']}}}svg"


def test_api_filter_keeps_snapshot_and_asset_metadata(tmp_path, small_document):
    original = copy.deepcopy(small_document)
    filenames = set()
    for api in ("async", "blocking"):
        catalog = write_charts(small_document, tmp_path, api=api)
        assert {case["api"] for case in catalog["cases"]} == {api}
        selected = set(asset_files(catalog))
        assert not filenames.intersection(selected)
        filenames.update(selected)
        metadata = (
            ET.parse(tmp_path / next(iter(selected)))
            .getroot()
            .find("svg:metadata", SVG)
        )
        assert json.loads(metadata.text)["source"] == original["source"]
    assert small_document == original
    assert filenames == {path.name for path in tmp_path.glob("*.svg")}
    with pytest.raises(ValueError, match="Chart API"):
        write_charts(small_document, tmp_path / "invalid", api="other")
    assert not (tmp_path / "invalid").exists()
