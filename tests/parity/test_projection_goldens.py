from __future__ import annotations

import hashlib
import json
from collections.abc import Mapping
from dataclasses import asdict, is_dataclass
from pathlib import Path
from typing import Any

import kordoc
import pytest

ROOT = Path(__file__).resolve().parents[1]
GOLDEN_ROOT = ROOT / "golden/document"
MANIFEST_PATH = ROOT / "golden/projection-manifest.json"
ORACLE_SOURCES = {
    "src/chunks.ts": "8de6844ea9c48f242cd0f5f136673eb5e8d93af44616953b9dbf02ae2d890056",
    "src/page-markdown.ts": "32019305ed8f4f41efc3a53ee4724c30685382c10a9681f8ef4caeadab3607d2",
    "src/table/analyze.ts": "89fa6b58f1f1e107882e5ddf0e4a97fb9067eb4c767d8036406a6bba09c24a58",
    "src/table/builder.ts": "062aa444cef210dfeb674d52a8de01158612ca89cfbd747c8077de63941da021",
    "src/table/classifier.ts": "45145c852fd32217ad05c49c7efd286b8c3dea83ae5660d657d56d097b8764cd",
}


def test_public_projection_api_is_available() -> None:
    from kordoc import tables

    assert callable(getattr(kordoc, "blocks_to_markdown", None))
    assert callable(getattr(kordoc, "blocks_to_pages", None))
    assert callable(getattr(kordoc, "blocks_to_chunks", None))
    assert callable(getattr(tables, "classify_table_tree", None))


@pytest.mark.parametrize(
    "case_id",
    [
        "projection-ordered-blocks",
        "projection-pages-gap",
        "projection-chunks-tree",
        "projection-table-merged",
        "projection-classifier-matrix",
        "projection-html-units",
    ],
)
def test_projection_goldens_compare_full_recursive_outputs(case_id: str) -> None:
    manifest = _read_manifest()
    _assert_manifest_fixtures(manifest)
    case = next(case for case in manifest["cases"] if case["id"] == case_id)
    source = json.loads(_golden_path(case["input"]).read_text(encoding="utf-8"))
    expected = json.loads(_golden_path(case["expected"]).read_text(encoding="utf-8"))
    actual = _run_projection(source)
    assert _plain(actual) == expected, case["id"]


def test_projection_manifest_fixture_hashes_and_provenance() -> None:
    _assert_manifest_fixtures(_read_manifest())


def _read_manifest() -> dict[str, Any]:
    return json.loads(MANIFEST_PATH.read_text(encoding="utf-8"))


def _assert_manifest_fixtures(manifest: dict[str, Any]) -> None:
    assert manifest["schema_version"] == 1
    assert manifest["oracle"]["commit"] == "bb71f7fb0bf51dd456d27505a8c04772df182144"
    assert manifest["summary"]["oracle_capture_cases"] == 6
    assert manifest["summary"]["parser_parity_numerator"] == 0
    assert manifest["summary"]["real_document_successful_numerator"] == 0
    assert len(manifest["cases"]) == 6

    for case in manifest["cases"]:
        assert case["license"] == "CC0-1.0"
        assert case["parser_parity_numerator"] is False
        assert case["oracle_execution"] == "captured"
        assert case["runtime"] == {"node": "v22.22.0", "tsx": "4.23.13"}
        assert len(case["dimensions"]) > 0
        assert case["oracle_sources"]
        assert set(case["oracle_sources"]) <= set(ORACLE_SOURCES)
        assert all(
            digest == ORACLE_SOURCES[source]
            for source, digest in case["oracle_sources"].items()
        )
        assert any(
            source in case["oracle_command"] for source in case["oracle_sources"]
        )
        source_bytes = _golden_path(case["input"]).read_bytes()
        expected_path = _golden_path(case["expected"])
        assert hashlib.sha256(source_bytes).hexdigest() == case["input_sha256"]
        assert (
            hashlib.sha256(expected_path.read_bytes()).hexdigest()
            == case["expected_sha256"]
        )


def test_synthetic_projection_goldens_do_not_change_document_numerator() -> None:
    document_manifest = json.loads(
        (ROOT / "golden/document-manifest.json").read_text(encoding="utf-8")
    )
    assert document_manifest["summary"]["parity_numerator_cases"] == 0
    assert document_manifest["summary"]["successful_oracle_cases"] == 0
    assert all(
        case["evidence_kind"] == "source_contract_smoke"
        and case["parity_numerator"] is False
        for case in document_manifest["cases"]
    )


def _run_projection(source: dict[str, Any]) -> Any:
    operation = source["operation"]
    if operation == "markdown":
        return kordoc.blocks_to_markdown(source["blocks"])
    if operation == "pages":
        return kordoc.blocks_to_pages(source["blocks"])
    if operation == "chunks":
        return kordoc.blocks_to_chunks(source["blocks"], source.get("options"))
    if operation == "classify_table_tree":
        from kordoc import tables

        return tables.classify_table_tree(source["blocks"])
    raise AssertionError(f"unknown projection operation: {operation}")


def _plain(value: Any) -> Any:
    if hasattr(value, "to_dict"):
        return _plain(value.to_dict())
    if is_dataclass(value):
        return _plain(asdict(value))
    if isinstance(value, Mapping):
        return {key: _plain(item) for key, item in value.items()}
    if isinstance(value, (list, tuple)):
        return [_plain(item) for item in value]
    return value


def _golden_path(relative_path: str) -> Path:
    path = Path(relative_path)
    assert not path.is_absolute()
    resolved = (GOLDEN_ROOT / path).resolve()
    assert resolved.is_relative_to(GOLDEN_ROOT.resolve())
    return resolved
