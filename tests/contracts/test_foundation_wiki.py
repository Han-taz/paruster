from __future__ import annotations

import re
from pathlib import Path

ROOT = Path(__file__).parents[2]
ENTRY_RELATIVE = "2026/09/2026-09-30-foundation-execution.md"
ENTRY = ROOT / "docs" / "WIKI" / ENTRY_RELATIVE
INDEX = ROOT / "docs" / "WIKI" / "README.md"


def _entry_text() -> str:
    assert ENTRY.is_file(), (
        f"append-only foundation record is missing: {ENTRY_RELATIVE}"
    )
    return ENTRY.read_text(encoding="utf-8")


def test_foundation_execution_entry_uses_template_metadata_and_records_commits() -> (
    None
):
    text = _entry_text()
    header = text.split("---", maxsplit=2)[1]
    for field in (
        "id: 2026-09-30-foundation-execution",
        "date: 2026-09-30",
        "status: recorded",
        "component: ci-foundation",
        "issue: null",
        "pr: https://github.com/Han-taz/paruster/pull/5",
        "commit: 33196b5",
    ):
        assert field in header
    for commit in (
        "a4e4f38",
        "4bb9622",
        "247f336",
        "afce280",
        "50cc75d",
        "9868850",
        "c6ba4d3",
        "33196b5",
    ):
        assert commit in text


def test_foundation_execution_entry_records_evidence_and_pending_work() -> None:
    text = _entry_text()
    required = (
        "88.63%",
        "12,287,343",
        "8,590,277",
        "31-second",
        "actionlint",
        "1.7.12",
        "official release artifact checksum was verified",
        "zizmor",
        "1.30.1",
        "cargo deny",
        "bsd-2-clause",
        "bsd-3-clause",
        "isc",
        "cargo audit",
        "86 dependencies",
        "1,277 advisories",
        "no vulnerabilities",
        "currently in progress",
        "36598937256",
        "36598937186",
        "36598937310",
        "36598937227",
        "parsers",
        "ocr",
        "transformations",
        "rendering",
        "mcp",
        "pending",
    )
    lowered = text.casefold()
    assert all(item in lowered for item in required)
    assert "pull/5" in text and "pull/4" in text
    assert "pull/1" in text and "pull/2" in text and "pull/3" in text


def test_foundation_execution_entry_appends_final_hosted_run_evidence() -> None:
    text = _entry_text()
    required = (
        "## Hosted run update",
        "At the time of the historical snapshot, CI, bounded fuzzing, and native wheels had already succeeded",
        "only Security was still in progress",
        "That statement was inaccurate for the other three runs",
        "last fully validated implementation head",
        "a9df4851e8fc1c8b2cd889fdec09e1f3fc111c1d",
        "d7fb6bd",
        "1a74688",
        "6c3a0d2",
        "d13b0ac",
        "485ceed",
        "a9df485",
        "36600464167",
        "36600464228",
        "36600464274",
        "36600464235",
        "ShellCheck checksum redirection",
        "zizmor installer artifact",
        "non-default stacked dependency review",
        "CodeQL dynamic release ref",
        "scope-message only",
        "main-target proof",
        "pending PR #4",
        "kordoc-0.1.0-cp310-abi3-macosx_11_0_arm64.whl",
        "e6a3117454deea9acf70ee86d2f20404d1ee04a37ee0707045f0f37d7226f35c",
        "kordoc-0.1.0.tar.gz",
        "23922328ce71ff671c3c83e145b08f1c781a9c32e35013a905d40f446aff7e1a",
        "succeeded across all six wheel jobs",
        "linked validation cycle",
        "docs-only evidence commit needs its own hosted pass",
    )
    assert all(item.casefold() in text.casefold() for item in required)
    for url in (
        "https://github.com/Han-taz/paruster/actions/runs/36600464167",
        "https://github.com/Han-taz/paruster/actions/runs/36600464228",
        "https://github.com/Han-taz/paruster/actions/runs/36600464274",
        "https://github.com/Han-taz/paruster/actions/runs/36600464235",
    ):
        assert url in text
    assert text.index("currently in progress") < text.index("## Hosted run update")


def test_foundation_execution_entry_is_indexed_exactly_once() -> None:
    assert INDEX.is_file()
    destinations = re.findall(r"\]\(([^)]+)\)", INDEX.read_text(encoding="utf-8"))
    assert destinations.count(ENTRY_RELATIVE) == 1
