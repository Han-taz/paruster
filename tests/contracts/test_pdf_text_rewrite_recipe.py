from __future__ import annotations

import hashlib
import json
import shutil
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).parents[2]
FIXTURES = ROOT / "crates/kordoc-pdf/tests/fixtures/pdf_text_rewrites"
PINS = {
    "generate.py": (
        5472,
        "78cdf38ee06f602eb17e8563f9d097f5c42321df2a9ea9eca890fc0f391861ba",
    ),
    "rewrite-inputs.json": (
        8009,
        "bb4a4e5b25772550579fb7ac971bda3b89c20c5305c92479dcb154b71b2db9c7",
    ),
    "radical-inputs.json": (
        13609,
        "015b8a8592cca5b1b1ef8a8a9dcb5040fae4cc6c82e76c67d6007e58aec20340",
    ),
    "oracle-rewrite-vectors.json": (
        3196,
        "7ef94f2751e0b6ef35ecd6c29b093a807bcebcb6e1b409f94628a62f28254373",
    ),
    "oracle-radical-mappings.json": (
        31031,
        "4f2a58c679a3d6c7681143455fbef58505bf97c96ff3a5d6bc939353506c605b",
    ),
}


def test_rewrite_inventory_regenerates_only_authored_inputs(tmp_path: Path) -> None:
    assert {p.name for p in FIXTURES.iterdir()} == set(PINS) | {
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
    assert {p.name for p in tmp_path.iterdir()} == {
        "generate.py",
        "rewrite-inputs.json",
        "radical-inputs.json",
    }
    for name in ["rewrite-inputs.json", "radical-inputs.json"]:
        assert (tmp_path / name).read_bytes() == (FIXTURES / name).read_bytes()
    subprocess.run([sys.executable, str(recipe)], check=True, capture_output=True)
    (tmp_path / "rewrite-inputs.json").write_bytes(b"tampered")
    assert (
        subprocess.run(
            [sys.executable, str(recipe)], check=False, capture_output=True
        ).returncode
        != 0
    )


def test_complete_radical_range_and_text_only_evidence_are_distinct() -> None:
    inputs = json.loads((FIXTURES / "rewrite-inputs.json").read_text())
    captured = json.loads((FIXTURES / "oracle-rewrite-vectors.json").read_text())
    radical_inputs = json.loads((FIXTURES / "radical-inputs.json").read_text())
    radicals = json.loads((FIXTURES / "oracle-radical-mappings.json").read_text())
    assert len(inputs["cases"]) == 21 and len(captured["cases"]) == 20
    assert [v["name"] for v in inputs["cases"][:20]] == [
        v["name"] for v in captured["cases"]
    ]
    assert all(
        set(v) == {"name", "output_text"} and len(v["output_text"]) == 1
        for v in captured["cases"]
    )
    for capture in [captured, radicals]:
        assert capture["oracle_commit"] == "bb71f7fb0bf51dd456d27505a8c04772df182144"
        assert (
            capture["text_line_sha256"]
            == "034b6a882eb913b723e21a1729e032bc45147e60acdf88aab655dfd415253072"
        )
        assert capture["unicode_version"] == "17.0"
    assert len(radical_inputs["items"]) == len(radicals["mappings"]) == 214
    for index, (authored, mapping) in enumerate(
        zip(radical_inputs["items"], radicals["mappings"], strict=True)
    ):
        code_point = 0x2F00 + index
        assert authored == {
            "code_point": f"U+{code_point:04X}",
            "text": chr(code_point),
        }
        assert mapping["code_point"] == authored["code_point"]
        assert mapping["input"] == authored["text"]
        assert len(mapping["output"]) == 1
        assert mapping["output_code_points"] == [f"U+{ord(mapping['output']):04X}"]
    cases = {v["name"]: v["output_text"][0] for v in captured["cases"]}
    assert cases["nfkc_is_limited_to_radical_block"] == "Ａ一①龠⿖"
    assert (
        cases["numeric_non_ascii_whitespace_survives_ascii_space_removal"]
        == "1\t2\u00a03\u300045"
    )
    assert cases["nel_is_not_ecmascript_regex_whitespace"] == "1\u00852 3"
    # Evidence for the next split/sort stage is preserved, not claimed by this helper.
    assert (
        inputs["cases"][-1]["name"]
        == "rewrite_precedes_even_spacing_split_and_sort_preserves_sequence"
    )
    assert captured["split_order_evidence"] == [
        {"text": "가", "x": 10, "y": 30, "seq": 2},
        {"text": "나", "x": 20, "y": 30, "seq": 2.001},
        {"text": "다", "x": 30, "y": 30, "seq": 2.002},
        {"text": "123", "x": 100, "y": 20, "seq": 1},
    ]
