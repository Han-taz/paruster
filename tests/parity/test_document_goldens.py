from __future__ import annotations

import hashlib
import json
from pathlib import Path

import kordoc
import pytest

from tests.parity.compare import JsonDifference, compare_json
from tests.parity.normalize import normalize

ROOT = Path(__file__).resolve().parents[1]
GOLDEN_ROOT = ROOT / "golden/document"
MANIFEST_PATH = ROOT / "golden/document-manifest.json"


def test_committed_document_goldens_compare_the_full_recursive_result() -> None:
    manifest = json.loads(MANIFEST_PATH.read_text(encoding="utf-8"))
    assert set(manifest) == {"schema_version", "oracle", "summary", "cases"}
    assert manifest["schema_version"] == 1
    assert set(manifest["oracle"]) == {
        "repository",
        "commit",
        "source_file",
        "source_sha256",
        "oracle_capture_command",
    }
    assert manifest["oracle"]["repository"] == "local ignored migration oracle"
    assert len(manifest["oracle"]["commit"]) == 40
    assert len(manifest["oracle"]["source_sha256"]) == 64
    assert "node --import tsx" in manifest["oracle"]["oracle_capture_command"]
    assert set(manifest["summary"]) == {
        "oracle_capture_cases",
        "parity_numerator_cases",
        "successful_oracle_cases",
        "parity_status",
    }
    assert manifest["cases"]
    oracle_capture_cases = 0
    parity_numerator_cases = 0
    successful_oracle_cases = 0

    for case in manifest["cases"]:
        assert set(case) == {
            "id",
            "input",
            "input_sha256",
            "expected",
            "expected_sha256",
            "license",
            "generator",
            "options",
            "dimensions",
            "evidence_kind",
            "oracle_execution",
            "parity_numerator",
            "normalization",
        }
        assert case["license"] == "CC0-1.0"
        assert case["generator"].strip()
        assert isinstance(case["options"], dict)
        assert case["dimensions"]
        assert case["evidence_kind"] in {"source_contract_smoke", "oracle_capture"}
        assert case["oracle_execution"] in {"unavailable", "captured"}
        assert type(case["parity_numerator"]) is bool
        if case["evidence_kind"] == "source_contract_smoke":
            assert case["oracle_execution"] == "unavailable"
            assert case["parity_numerator"] is False
        else:
            assert case["oracle_execution"] == "captured"
            assert case["parity_numerator"] is True
        assert set(case["normalization"]) == {
            "zip_timestamp_pointers",
            "xml_attribute_pointers",
        }
        raw = _manifest_path(case["input"]).read_bytes()
        expected_path = _manifest_path(case["expected"])
        assert hashlib.sha256(raw).hexdigest() == case["input_sha256"]
        assert (
            hashlib.sha256(expected_path.read_bytes()).hexdigest()
            == case["expected_sha256"]
        )
        expected = json.loads(expected_path.read_text(encoding="utf-8"))
        if case["evidence_kind"] == "oracle_capture":
            oracle_capture_cases += 1
            successful_oracle_cases += expected.get("success") is True
        parity_numerator_cases += case["parity_numerator"]
        actual = kordoc.try_parse(raw, options=case["options"]).to_dict()
        difference = compare_json(
            normalize(
                expected,
                zip_timestamp_pointers=case["normalization"]["zip_timestamp_pointers"],
                xml_attribute_pointers=case["normalization"]["xml_attribute_pointers"],
            ),
            normalize(
                actual,
                zip_timestamp_pointers=case["normalization"]["zip_timestamp_pointers"],
                xml_attribute_pointers=case["normalization"]["xml_attribute_pointers"],
            ),
        )
        assert difference is None, _format_difference(case["id"], difference)

    assert manifest["summary"]["oracle_capture_cases"] == oracle_capture_cases
    assert manifest["summary"]["parity_numerator_cases"] == parity_numerator_cases
    assert manifest["summary"]["successful_oracle_cases"] == successful_oracle_cases
    if successful_oracle_cases == 0:
        assert manifest["summary"]["parity_status"] == "pending"


@pytest.mark.parametrize("path", ["../outside.json", "/tmp/outside.json"])
def test_document_manifest_paths_cannot_escape_golden_root(path: str) -> None:
    with pytest.raises(AssertionError):
        _manifest_path(path)


def test_document_manifest_reports_first_recursive_difference() -> None:
    difference = compare_json(
        {"blocks": [{"children": [{"text": "expected"}]}]},
        {"blocks": [{"children": [{"text": "actual"}]}]},
    )
    assert difference is not None
    assert difference.pointer == "/blocks/0/children/0/text"
    assert difference.expected == "expected"
    assert difference.actual == "actual"


def _manifest_path(relative_path: str) -> Path:
    path = Path(relative_path)
    assert not path.is_absolute()
    resolved = (GOLDEN_ROOT / path).resolve()
    assert resolved.is_relative_to(GOLDEN_ROOT.resolve())
    return resolved


def _format_difference(case_id: str, difference: JsonDifference | None) -> str:
    if difference is None:
        return f"{case_id}: no difference"
    return (
        f"{case_id}: first difference at {difference.pointer or '<root>'}: "
        f"expected {difference.expected!r}, got {difference.actual!r}"
    )
