"""Compare candidate HWPX semantics without promoting the parity manifest."""

from __future__ import annotations

import hashlib
import json
import re
from pathlib import Path

import kordoc
import pytest

from tests.parity.compare import compare_json

ROOT = Path(__file__).resolve().parents[2]
CAPTURE = ROOT / "crates/kordoc-hancom/tests/support/hwpx-oracle-results.jsonl"
FIXTURES = ROOT / "tests/golden/document/hwpx/fixtures"
CASES = [json.loads(line) for line in CAPTURE.read_text().splitlines()][:8]
PROVENANCE = ROOT / "crates/kordoc-hancom/tests/support/README.md"


def test_committed_candidate_inputs_match_all_twelve_pinned_h0_recipes() -> None:
    pins = dict(
        re.findall(
            r"\| `([a-z0-9_]+)` \| `([a-f0-9]{64})` \|",
            PROVENANCE.read_text(encoding="utf-8"),
        )
    )
    assert len(pins) == 12
    assert {path.stem for path in FIXTURES.glob("*.hwpx")} == set(pins)
    for recipe, digest in pins.items():
        assert (
            hashlib.sha256((FIXTURES / f"{recipe}.hwpx").read_bytes()).hexdigest()
            == digest
        )
        fuzz_seed = ROOT / "fuzz/corpus/hwpx_package" / f"{recipe}.hwpx"
        assert hashlib.sha256(fuzz_seed.read_bytes()).hexdigest() == digest


@pytest.mark.parametrize("case", CASES, ids=[case["id"] for case in CASES])
def test_hwpx_candidate_matches_complete_captured_semantic_result(case: dict) -> None:
    options = (
        {"password": "fixture-password"} if case["id"].startswith("encrypted_") else {}
    )
    actual = kordoc.try_parse(FIXTURES / f"{case['id']}.hwpx", options).to_dict()
    difference = compare_json(case["result"], actual)
    assert difference is None, (
        f"{case['id']}: {difference.pointer}: expected {difference.expected!r}, "
        f"actual {difference.actual!r}"
    )


@pytest.mark.parametrize("entrypoint", [kordoc.parse, kordoc.parse_hwpx])
def test_public_hwpx_parsers_preserve_complete_malformed_section_result(
    entrypoint,
) -> None:
    case = next(case for case in CASES if case["id"] == "malformed_section")
    actual = entrypoint(FIXTURES / "malformed_section.hwpx").to_dict()
    difference = compare_json(case["result"], actual)
    assert difference is None, (
        f"{difference.pointer}: expected {difference.expected!r}, "
        f"actual {difference.actual!r}"
    )
