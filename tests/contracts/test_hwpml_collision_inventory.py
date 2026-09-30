from __future__ import annotations

import hashlib
import json
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).parents[2]
CORPUS = ROOT / "tests/golden/document/hwpml-collision"
FIXTURES = CORPUS / "fixtures"
GENERATOR_SHA256 = "502caa7d3dab075c05582b3ce1913522e505d0aafd1ac819080fa703d83c1be8"
CAPTURE_SHA256 = "0387de2f63a23795a10dee84feb337f9b3f4f377d89a0edf6256415c77f1564d"
FIXTURE_PINS = {
    "collision_unmatched.xml": (
        511,
        "27279b6d327574607fcf060f7c4ff929bbc9e4bf33df382336eeacaf1d32d230",
    ),
    "collision_repeated_text_decoy.xml": (
        743,
        "92f7e789b361091abed261e14ef683a9865b249f79d208d46e23de6af6f77b96",
    ),
    "collision_blank_cell.xml": (
        503,
        "5a120b4cb1aa5517c68c7ecc6dc0689bb294ab4eee2c12ef3a830c40e6c8d92b",
    ),
}
CASE_OPTIONS = {
    "collision_unmatched_default": ("collision_unmatched.xml", {}),
    "collision_decoy_default": ("collision_repeated_text_decoy.xml", {}),
    "collision_decoy_keep_trailing_empty_cols": (
        "collision_repeated_text_decoy.xml",
        {"keepTrailingEmptyCols": True},
    ),
    "collision_decoy_drop_trailing_empty_cols": (
        "collision_repeated_text_decoy.xml",
        {"keepTrailingEmptyCols": False},
    ),
    "collision_blank_cell_default": ("collision_blank_cell.xml", {}),
}


def _records() -> list[dict]:
    return [
        json.loads(line)
        for line in (CORPUS / "oracle-results.jsonl")
        .read_text(encoding="utf-8")
        .splitlines()
    ]


def _table(result: dict) -> dict:
    blocks = result["blocks"]
    assert len(blocks) == 1
    assert blocks[0]["type"] == "table"
    return blocks[0]["table"]


def test_collision_fixtures_regenerate_and_verify_offline(tmp_path: Path) -> None:
    generator = CORPUS / "generate.py"
    assert hashlib.sha256(generator.read_bytes()).hexdigest() == GENERATOR_SHA256
    assert {path.name for path in CORPUS.iterdir()} == {
        "README.md",
        "fixtures",
        "generate.py",
        "oracle-results.jsonl",
    }
    assert {path.name for path in FIXTURES.iterdir()} == set(FIXTURE_PINS)

    generated = tmp_path / "generated"
    subprocess.run(
        [sys.executable, str(generator), "--write", "--output-dir", str(generated)],
        check=True,
        capture_output=True,
        text=True,
    )
    subprocess.run(
        [sys.executable, str(generator)],
        check=True,
        capture_output=True,
        text=True,
    )

    expected_names = set(FIXTURE_PINS)
    assert {path.name for path in FIXTURES.iterdir()} == expected_names
    assert {path.name for path in generated.iterdir()} == expected_names
    for name, (size, digest) in FIXTURE_PINS.items():
        source_bytes = (FIXTURES / name).read_bytes()
        assert source_bytes == (generated / name).read_bytes()
        assert len(source_bytes) == size
        assert hashlib.sha256(source_bytes).hexdigest() == digest

    tampered = generated / "collision_blank_cell.xml"
    tampered.write_bytes(b"tampered")
    rejected = subprocess.run(
        [sys.executable, str(generator), "--output-dir", str(generated)],
        check=False,
        capture_output=True,
        text=True,
    )
    assert rejected.returncode != 0
    assert tampered.read_bytes() == b"tampered"

    tampered.unlink()
    (generated / "extra.xml").write_bytes(b"<extra/>")
    rejected_extra = subprocess.run(
        [sys.executable, str(generator), "--output-dir", str(generated)],
        check=False,
        capture_output=True,
        text=True,
    )
    assert rejected_extra.returncode != 0


def test_five_complete_collision_captures_are_pinned_and_ordered() -> None:
    capture = CORPUS / "oracle-results.jsonl"
    assert hashlib.sha256(capture.read_bytes()).hexdigest() == CAPTURE_SHA256
    records = _records()
    assert [record["case_id"] for record in records] == list(CASE_OPTIONS)

    observed_fixtures = set()
    for record in records:
        name, options = CASE_OPTIONS[record["case_id"]]
        assert record["input_path"] == f"fixtures/{name}"
        assert record["options"] == options
        assert (record["input_bytes"], record["input_sha256"]) == FIXTURE_PINS[name]
        assert record["result"]["success"] is True
        assert record["result"]["fileType"] == "hwpml"
        observed_fixtures.add(name)
    assert observed_fixtures == set(FIXTURE_PINS)

    by_id = {record["case_id"]: record["result"] for record in records}
    unmatched = _table(by_id["collision_unmatched_default"])
    unmatched_cell = unmatched["cells"][0][0]
    assert unmatched["rows"] == unmatched["cols"] == 1
    assert unmatched_cell["text"] == "owner\ninner\ncollision"
    assert "blocks" not in unmatched_cell

    default = _table(by_id["collision_decoy_default"])
    keep = _table(by_id["collision_decoy_keep_trailing_empty_cols"])
    drop = _table(by_id["collision_decoy_drop_trailing_empty_cols"])
    assert (default["rows"], default["cols"]) == (1, 2)
    assert (drop["rows"], drop["cols"]) == (1, 2)
    assert (keep["rows"], keep["cols"]) == (1, 3)
    assert default["cells"][0][0]["text"] == "owner\ninner\ncollision"
    assert default["cells"][0][1]["text"] == "owner\ninner"
    assert default["cells"][0][1]["blocks"] == [
        {"type": "paragraph", "text": "owner"},
        {
            "type": "table",
            "table": {
                "rows": 1,
                "cols": 1,
                "cells": [[{"text": "inner", "colSpan": 1, "rowSpan": 1}]],
                "hasHeader": False,
            },
        },
    ]
    assert keep["cells"][0][2]["text"] == ""
    assert [default["cells"][0][column] for column in range(2)] == [
        drop["cells"][0][column] for column in range(2)
    ]

    blank = _table(by_id["collision_blank_cell_default"])
    blank_cell = blank["cells"][0][0]
    assert (blank["rows"], blank["cols"]) == (1, 1)
    assert blank_cell["text"] == "owner\ninner"
    assert blank_cell["blocks"] == [
        {"type": "paragraph", "text": "owner"},
        {
            "type": "table",
            "table": {
                "rows": 1,
                "cols": 1,
                "cells": [[{"text": "inner", "colSpan": 1, "rowSpan": 1}]],
                "hasHeader": False,
            },
        },
    ]

    # These are exact legacy public results. In particular, the oracle's
    # unmatched omission and wrong-cell decoy fallback intentionally differ
    # from the private Rust adapter's fail-closed UnsupportedFormat policy.
    # Only the blank-collider control retains the nested blocks at their owner.
