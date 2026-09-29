from __future__ import annotations

import hashlib
import json
import re
from pathlib import Path
from typing import Any

import kordoc

from tests.parity.format_helper_inputs import build_input

PARITY_DIR = Path(__file__).resolve().parent
MANIFEST = json.loads((PARITY_DIR / "format_helper_manifest.json").read_text())
EXPECTED = json.loads((PARITY_DIR / "format_helper_expected.json").read_text())
API_NAMES = (
    "is_zip_file",
    "is_hwpx_file",
    "is_old_hwp_file",
    "is_pdf_file",
    "detect_ole2_format",
    "detect_zip_format",
)


def test_format_helpers_match_captured_oracle_matrix() -> None:
    cases = MANIFEST["cases"]
    oracle = MANIFEST["oracle"]
    assert MANIFEST["schema_version"] == 1
    assert re.fullmatch(r"[0-9a-f]{40}", oracle["commit"])
    assert oracle["sources"]
    assert all(
        re.fullmatch(r"[0-9a-f]{64}", digest) for digest in oracle["sources"].values()
    )
    assert oracle["runtime"].strip()
    assert oracle["capture_setup"].strip()
    assert oracle["capture_command"].strip()

    expected_digest = hashlib.sha256(
        (PARITY_DIR / "format_helper_expected.json").read_bytes()
    ).hexdigest()
    assert expected_digest == MANIFEST["expected_sha256"]

    case_ids = [case["id"] for case in cases]
    assert len(case_ids) == len(set(case_ids))
    assert set(case_ids) == set(EXPECTED)
    generator_for = {
        "hex": "literal-hex-v1",
        "zip_entries": "python-stdlib-zipfile-stored-fixed-timestamp-v1",
        "cfb_stream": "minimal-cfb-builder-v1",
    }

    for case in cases:
        case_id = case["id"]
        recipes = set(case) & generator_for.keys()
        assert len(recipes) == 1, f"{case_id} must have exactly one input recipe"
        recipe = recipes.pop()
        assert case["generator"] == generator_for[recipe], case_id
        assert case["license"].strip(), case_id
        assert case["provenance"].strip(), case_id

        expected = EXPECTED[case_id]
        assert set(expected) == set(API_NAMES), f"{case_id} API keys are incomplete"
        payload = build_input(case)
        digest = hashlib.sha256(payload).hexdigest()
        assert digest == case["sha256"], f"{case_id} input digest changed"

        actual: dict[str, Any] = {
            name: getattr(kordoc, name)(payload) for name in API_NAMES
        }
        assert actual == expected, f"{case_id} differs from captured oracle"
