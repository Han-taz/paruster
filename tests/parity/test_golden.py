import json
from pathlib import Path

import kordoc

ROOT = Path(__file__).parents[1]


def test_committed_golden_cases() -> None:
    manifest_path = ROOT / "golden/manifest.json"
    assert manifest_path.exists(), "the committed golden manifest is missing"
    manifest = json.loads(manifest_path.read_text(encoding="utf-8"))
    assert manifest["cases"]

    for case in manifest["cases"]:
        raw = (ROOT / "golden" / case["input"]).read_bytes()
        actual = {"fileType": kordoc.detect_format(raw)}
        expected = json.loads(
            (ROOT / "golden" / case["expected"]).read_text(encoding="utf-8")
        )
        assert actual == expected
