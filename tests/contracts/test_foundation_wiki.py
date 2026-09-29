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


def test_foundation_execution_entry_is_indexed_exactly_once() -> None:
    assert INDEX.is_file()
    destinations = re.findall(r"\]\(([^)]+)\)", INDEX.read_text(encoding="utf-8"))
    assert destinations.count(ENTRY_RELATIVE) == 1
