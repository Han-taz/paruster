from __future__ import annotations

import hashlib
import json
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).parents[2]
SUPPORT = ROOT / "crates/kordoc-hancom/tests/support/hwpml"
FIXTURES = ROOT / "tests/golden/document/hwpml/fixtures"
PINS = {
    "normal_metadata_styles.xml": (
        1467,
        "7c4bde519ae9ffbf7e65b2330f82e6838fc6d430c55c7cb84a2d84a12188f715",
    ),
    "empty_body.xml": (
        133,
        "b19c14c589cfecae3f7adfc1723a8286f6a6488775e3a297ba475349b408f92f",
    ),
    "empty_sections.xml": (
        146,
        "c65386caf4be41ee33859e631c71ba67be0c664ded192a6f8f79b87842c97151",
    ),
    "nested_table.xml": (
        949,
        "3255695c5b3a0a0cd677687d9e27fa9248f30118a7ebf0be6afc19287e86c308",
    ),
    "malformed_partial.xml": (
        223,
        "326aa488832d9979de62ac896db29d8b958763803375dca597ffe99b8a3449ea",
    ),
    "malformed_unclosed.xml": (
        178,
        "907d2491125acd4076ec954559d77f35832387237f3ae9638836448b4e4deb42",
    ),
    "dtd_external_entity.xml": (
        221,
        "6055036457910c3d11acd3b6f142c61fd4c34ae03a20a152bf833e5791017a92",
    ),
    "dtd_external_entity_reference.xml": (
        259,
        "4463607f952b8a62b79fd498d03ea49e8f4685975e96cb040ee84ba4fd8134e8",
    ),
}
CASE_OPTIONS = {
    "normal_default": {},
    "normal_page_2": {"pages": [2]},
    "empty_body_default": {},
    "empty_sections_default": {},
    "empty_sections_page_2": {"pages": [2]},
    "nested_table_default": {},
    "nested_table_keep_trailing_empty_cols": {"keepTrailingEmptyCols": True},
    "nested_table_drop_trailing_empty_cols": {"keepTrailingEmptyCols": False},
    "malformed_partial_default": {},
    "dtd_external_entity_default": {},
    "generated_over_50_mib": {},
    "malformed_unclosed_xml": {},
    "dtd_external_entity_reference": {},
}


def _captures() -> list[dict]:
    return [
        json.loads(line)
        for line in (SUPPORT / "hwpml-oracle-results.jsonl")
        .read_text(encoding="utf-8")
        .splitlines()
    ]


def test_authored_hwpml_inputs_regenerate_without_oracle(tmp_path: Path) -> None:
    generator = SUPPORT / "generate.py"
    assert hashlib.sha256(generator.read_bytes()).hexdigest() == (
        "13a40938e58cacac0f3194a3813ecac187450afc1a68b2c9e36b652115e60cfe"
    )
    subprocess.run(
        [sys.executable, str(generator), "--write", "--output-dir", str(tmp_path)],
        check=True,
        capture_output=True,
    )
    assert {p.name for p in tmp_path.iterdir()} == set(PINS)
    assert {p.name for p in FIXTURES.iterdir()} == set(PINS)
    for name, (size, digest) in PINS.items():
        data = (FIXTURES / name).read_bytes()
        assert data == (tmp_path / name).read_bytes()
        assert len(data) == size
        assert hashlib.sha256(data).hexdigest() == digest
    subprocess.run(
        [sys.executable, str(generator), "--output-dir", str(tmp_path)],
        check=True,
        capture_output=True,
    )
    # Verify mode must reject changed or extra inputs rather than overwriting them.
    changed = tmp_path / "empty_body.xml"
    changed.write_bytes(b"tampered")
    result = subprocess.run(
        [sys.executable, str(generator), "--output-dir", str(tmp_path)],
        check=False,
        capture_output=True,
    )
    assert result.returncode != 0
    assert changed.read_bytes() == b"tampered"


def test_hwpml_complete_captures_and_input_inventory_are_frozen() -> None:
    capture = SUPPORT / "hwpml-oracle-results.jsonl"
    assert hashlib.sha256(capture.read_bytes()).hexdigest() == (
        "aba0dd587753d2c898340be9fe10e4dfd79a535c9f1356c4c83f0ede250551c6"
    )
    records = _captures()
    assert [row["case_id"] for row in records] == list(CASE_OPTIONS)
    observed = set()
    for row in records:
        assert row["options"] == CASE_OPTIONS[row["case_id"]]
        assert row["result"]["fileType"] == "hwpml"
        assert isinstance(row["result"]["success"], bool)
        if row["input_path"] is not None:
            name = Path(row["input_path"]).name
            assert row["input_path"] == f"fixtures/{name}"
            assert (row["input_bytes"], row["input_sha256"]) == PINS[name]
            observed.add(name)
    assert observed == set(PINS)


def test_hwpml_oversize_recipe_hash_is_verified_without_large_fixture() -> None:
    row = next(r for r in _captures() if r["case_id"] == "generated_over_50_mib")
    size = 50 * 1024 * 1024 + 1
    prefix = b'<?xml version="1.0"?><HWPML/>'
    digest = hashlib.sha256(prefix)
    remaining = size - len(prefix)
    chunk = b" " * (64 * 1024)
    while remaining:
        count = min(remaining, len(chunk))
        digest.update(chunk[:count])
        remaining -= count
    assert row["input_path"] is None
    assert row["input_bytes"] == size
    assert row["input_sha256"] == digest.hexdigest()
    assert row["result"] == {
        "success": False,
        "fileType": "hwpml",
        "error": "HWPML 파일 크기 초과 (50.0MB > 50MB)",
        "code": "DECOMPRESSION_BOMB",
    }


def test_hwpml_captures_distinguish_entity_recovery_and_fatal_xml() -> None:
    by_id = {row["case_id"]: row["result"] for row in _captures()}
    assert by_id["malformed_unclosed_xml"] == {
        "success": False,
        "fileType": "hwpml",
        "error": "문서 처리 중 오류가 발생했습니다",
        "code": "PARSE_ERROR",
    }
    assert by_id["dtd_external_entity_default"]["markdown"] == "앞&local;뒤"
    reference = by_id["dtd_external_entity_reference"]
    assert reference["markdown"] == "앞&probe;뒤"
    assert reference["warnings"] == [
        {
            "message": "HWPML XML 파싱 경고: entity not found:&probe;",
            "code": "MALFORMED_XML",
        }
    ]
    assert b"&amp;local;" in (FIXTURES / "dtd_external_entity.xml").read_bytes()
    assert b"&probe;" in (FIXTURES / "dtd_external_entity_reference.xml").read_bytes()
    # Ordinary verification uses only Python and frozen observations.
    assert {p.name for p in SUPPORT.iterdir() if p.is_file()} == {
        "generate.py",
        "hwpml-oracle-results.jsonl",
    }
