from __future__ import annotations

import hashlib
import json
import shutil
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).parents[2]
FIXTURES = ROOT / "crates/kordoc-pdf/tests/fixtures/pdfjs_metadata"
PINS = {
    "mixed.pdf": (
        810,
        "aa843ac436da42c84b938bd59a544cd11842b97fd5edf7be73180abbbb645145",
    ),
    "delimiters.pdf": (
        676,
        "0b729cf3a66503eaefb161d2b41fb64f5dd86d69a0c4a6b85efba09efa7acdea",
    ),
    "dates.pdf": (
        739,
        "208287a8ac86bb4191e2c67aa0d4803f7ede12770826047072f0328ec15721e2",
    ),
}


def test_pdf_metadata_inputs_regenerate_with_only_the_stdlib(tmp_path: Path) -> None:
    recipe = FIXTURES / "generate.py"
    assert hashlib.sha256(recipe.read_bytes()).hexdigest() == (
        "f4c19e1cbf63c80059329c7501ee58e51e56f15e902ff9d00217be1a20f3be8d"
    )
    copied = tmp_path / recipe.name
    shutil.copyfile(recipe, copied)
    subprocess.run(
        [sys.executable, str(copied), "--write"], check=True, capture_output=True
    )
    assert {p.name for p in tmp_path.glob("*.pdf")} == set(PINS)
    for name, (size, digest) in PINS.items():
        data = (tmp_path / name).read_bytes()
        assert data == (FIXTURES / name).read_bytes()
        assert len(data) == size
        assert hashlib.sha256(data).hexdigest() == digest


def test_pdf_metadata_capture_inventory_and_partial_semantics_are_frozen() -> None:
    capture = FIXTURES / "oracle-captures.jsonl"
    assert hashlib.sha256(capture.read_bytes()).hexdigest() == (
        "d79d30dc6b189bbfcdb4e05fbb8bec54de8c9740298c46ee1034ce7838413145"
    )
    rows = [
        json.loads(line) for line in capture.read_text(encoding="utf-8").splitlines()
    ]
    assert [(row["input"], row["mode"]) for row in rows] == [
        (name, mode) for name in PINS for mode in ("full-parse", "metadata-only")
    ]
    for row in rows:
        assert row["schema_version"] == 1
        assert row["input_sha256"] == PINS[row["input"]][1]
        assert row["oracle_version"] == "4.16.3"
        assert row["oracle_commit"] == "bb71f7fb0bf51dd456d27505a8c04772df182144"
        assert row["oracle_parser_sha256"] == (
            "2eb7018d17bf9bb3d39b7d2cb21145fe812a5257a239331aa13d44da9776bf70"
        )
        assert row["pdfjs_version"] == "4.10.38"
        metadata = row["metadata"]
        assert metadata["pageCount"] == 1
        if row["mode"] == "full-parse":
            assert metadata["pageMode"] == "layout"
        else:
            assert "pageMode" not in metadata
        if row["input"] == "mixed.pdf":
            assert metadata["keywords"] == ["alpha", "beta", "alpha", "gamma"]
            assert "creator" not in metadata and "description" not in metadata
        elif row["input"] == "delimiters.pdf":
            assert metadata["keywords"] == []
        else:
            assert metadata["createdAt"] == "2025-12-01T00:00:00"
            assert metadata["modifiedAt"] == "2025-13-99T00:00:00"
