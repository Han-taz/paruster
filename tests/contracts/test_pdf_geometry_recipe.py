from __future__ import annotations

import hashlib
import json
import shutil
import struct
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).parents[2]
FIXTURES = ROOT / "crates/kordoc-pdf/tests/fixtures/pdfjs_geometry"
PINS = {
    "generate.py": (
        2249,
        "2640fec5a5966f6f3edcf6ddfa722a6713da4bdf62dcb035014e260a0feaee45",
    ),
    "fractional_cropbox.pdf": (
        749,
        "d3d9d26a445c99234439c112a5822d8bc34f2b6e0b88ac9c78a7520ce14b253e",
    ),
    "worker-response.json": (
        413,
        "459107dc4d729bae5504ca298455a5e9207016b137562700b8406afbc9629aa9",
    ),
    "oracle-public-output.json": (
        549,
        "579336a388156c85af42b9762fb5cd6c39f8275404b35ef0bfc7491b6746f883",
    ),
    "rounding-vectors.json": (
        2597,
        "ea9db6d3d549b4e3d5aab317fe46b2cb4b03e529b9b6721c37339b73fdc1dea7",
    ),
}


def test_geometry_inventory_and_stdlib_regeneration(tmp_path: Path) -> None:
    assert {p.name for p in FIXTURES.iterdir() if p.is_file()} == set(PINS) | {
        "README.md",
        "LICENSE.txt",
    }
    for name, (size, digest) in PINS.items():
        data = (FIXTURES / name).read_bytes()
        assert len(data) == size
        assert hashlib.sha256(data).hexdigest() == digest
    recipe = tmp_path / "generate.py"
    shutil.copyfile(FIXTURES / recipe.name, recipe)
    subprocess.run(
        [sys.executable, str(recipe), "--write"], check=True, capture_output=True
    )
    pdf = tmp_path / "fractional_cropbox.pdf"
    assert pdf.read_bytes() == (FIXTURES / pdf.name).read_bytes()
    subprocess.run([sys.executable, str(recipe)], check=True, capture_output=True)
    pdf.write_bytes(pdf.read_bytes() + b"changed")
    assert (
        subprocess.run(
            [sys.executable, str(recipe)], capture_output=True, check=False
        ).returncode
        != 0
    )


def test_geometry_captures_preserve_round_before_cropbox_shift() -> None:
    raw = json.loads((FIXTURES / "worker-response.json").read_text())["result"]
    page = raw["pages"][0]
    assert page["view_box"] == [10.25, 20.75, 110.25, 220.75]
    assert page["rotation"] == 90
    assert page["items"][0]["transform"] == [10, 0, 0, 10, 11.5, 21.5]
    oracle = json.loads((FIXTURES / "oracle-public-output.json").read_text())
    assert oracle["markdown"] == "positive tie"
    assert oracle["blocks"][0]["bbox"] == {
        "page": 1,
        "x": 1.75,
        "y": 1.25,
        "width": 47,
        "height": 10,
    }
    vectors = json.loads((FIXTURES / "rounding-vectors.json").read_text())
    assert vectors["rusty_v8_crate"] == "152.2.0"
    assert vectors["v8_version"] == "15.2.124.1-rusty"
    assert len(vectors["vectors"]) == 16
    for vector in vectors["vectors"]:
        assert struct.pack(">d", float(vector["input"])).hex() == vector["input_bits"]
        assert (
            struct.pack(">d", float(vector["rounded"])).hex() == vector["rounded_bits"]
        )
    near_half = next(
        v for v in vectors["vectors"] if v["input"] == "0.49999999999999994"
    )
    assert near_half["rounded"] == "0" and near_half["naive_floor_plus_half"] == "1"
    negative_half = next(v for v in vectors["vectors"] if v["input"] == "-0.5")
    assert (
        negative_half["is_negative_zero"]
        and negative_half["rounded_bits"] == "8000000000000000"
    )
