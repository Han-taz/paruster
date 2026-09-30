from __future__ import annotations

import hashlib
import json
import shutil
import struct
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).parents[2]
FIXTURES = ROOT / "crates/kordoc-pdf/tests/fixtures/pdfjs_text_normalization"
PINS = {
    "base_items.pdf": (
        1125,
        "f4abb4f30af5582ecaca9c54e73b93f57632d84b0fd464a01b1cf4f22c07bf0f",
    ),
    "make_fixture.py": (
        2532,
        "9d9a77307876b6804ec8ec599f76356aa0348e8b00e12452b95e1f331f9c58ca",
    ),
    "raw-worker-response.json": (
        2352,
        "a5d5366766cccb1da67754b759a052bad1045de0472b1983805ef1f5c6fbc2e8",
    ),
    "oracle-scalar-projection.json": (
        3394,
        "0f9ac629705012f5f065c8d135ceb617c826af4259d95d8813f4ee604bc56752",
    ),
    "oracle-synthetic-vectors.json": (
        2387,
        "dc1697861512c72e28f625b5b5b5f06d1bf093eb4954f3db1145e45fd666e736",
    ),
    "math-hypot-vectors.json": (
        1934,
        "cca58018caa7e26846b5450fa4bf503f5fc5873b402fbf6699cdd75005bed2a1",
    ),
}


def test_text_scalar_inventory_and_stdlib_regeneration(tmp_path: Path) -> None:
    assert {p.name for p in FIXTURES.iterdir()} == set(PINS) | {
        "README.md",
        "LICENSE.txt",
    }
    for name, (size, digest) in PINS.items():
        data = (FIXTURES / name).read_bytes()
        assert len(data) == size
        assert hashlib.sha256(data).hexdigest() == digest
    recipe = tmp_path / "make_fixture.py"
    shutil.copyfile(FIXTURES / recipe.name, recipe)
    subprocess.run(
        [sys.executable, str(recipe), "--write"], check=True, capture_output=True
    )
    pdf = tmp_path / "base_items.pdf"
    assert pdf.read_bytes() == (FIXTURES / pdf.name).read_bytes()
    subprocess.run([sys.executable, str(recipe)], check=True, capture_output=True)
    pdf.write_bytes(pdf.read_bytes() + b"tampered")
    assert (
        subprocess.run(
            [sys.executable, str(recipe)], check=False, capture_output=True
        ).returncode
        != 0
    )


def test_raw_source_and_synthetic_scalar_evidence_are_distinct() -> None:
    raw = json.loads((FIXTURES / "raw-worker-response.json").read_text())
    assert raw["status"] == "success"
    items = raw["result"]["pages"][0]["items"]
    assert len(items) == 20
    assert items[0]["text"] == "Alpha" and items[1]["text"] == ""
    actual = json.loads((FIXTURES / "oracle-scalar-projection.json").read_text())
    assert actual["input_count"] == 20 and len(actual["items"]) == 15
    assert actual["raw_worker_response_sha256"] == PINS["raw-worker-response.json"][1]
    synthetic = json.loads((FIXTURES / "oracle-synthetic-vectors.json").read_text())
    for capture in [actual, synthetic]:
        assert capture["oracle_commit"] == "bb71f7fb0bf51dd456d27505a8c04772df182144"
        assert (
            capture["text_line_sha256"]
            == "034b6a882eb913b723e21a1729e032bc45147e60acdf88aab655dfd415253072"
        )
    case = synthetic["cases"]["whitespace_gap_and_stable_tie"]
    assert len(case["input"]) == 4
    assert [item["seq"] for item in case["output"]] == [1, 3, 4]
    assert [item["text"] for item in case["output"]] == [
        "before",
        "after",
        "\u0085NEL\u0085",
    ]
    assert [(item["x"], item["y"]) for item in case["output"][:2]] == [
        (10, 20),
        (10, 20),
    ]
    math = json.loads((FIXTURES / "math-hypot-vectors.json").read_text())
    assert (
        math["rusty_v8_crate"] == "152.2.0" and math["v8_version"] == "15.2.124.1-rusty"
    )
    assert math["rust_target"] == "aarch64-apple-darwin"
    for vector in math["vectors"]:
        assert struct.pack(">d", vector["a"]).hex() == vector["a_bits"]
        assert struct.pack(">d", vector["b"]).hex() == vector["b_bits"]
    edge = next(v for v in math["vectors"] if v["a_bits"] == "3fdfffffffffffff")
    assert edge["b_bits"] == "3e3ffffffffffff8"
    assert edge["js_hypot_bits"] == "3fdfffffffffffff"
    assert edge["rust_hypot_bits"] == "3fe0000000000000"
    assert edge["js_font_size"] == 0 and edge["rust_font_size"] == 1
