from __future__ import annotations

import re
from pathlib import Path

ROOT = Path(__file__).parents[2]
PLANS = ROOT / "docs" / "SSOT" / "migration" / "plans"
HWPX = PLANS / "2026-09-30-hwpx-implementation-plan.md"
PDF = PLANS / "2026-09-30-pdf-implementation-plan.md"


def test_hwpx_plan_locks_security_and_ownership_decisions() -> None:
    text = HWPX.read_text(encoding="utf-8")
    required = (
        "depends only on `kordoc-ir`",
        "Never scan local headers to bypass that gate",
        "500 central-directory records, including directory records",
        "268,435,456 bytes",
        "caps element depth at **200**",
        "4,000,000 per document",
        "P1 exposes no Python `on_progress` callback",
        "core applies P7 Markdown/table projections",
        "Luna A, package/security",
        "Luna B, content/IR",
        "Coordinator only",
        "hwpx_package",
        "hwpx_xml",
    )
    assert all(item in text for item in required)


def test_pdf_plan_locks_pure_rust_semantics_and_budgets() -> None:
    text = PDF.read_text(encoding="utf-8")
    required = (
        "pure-Rust PDF semantic parser",
        "depends on `kordoc-ir` only",
        "PDFium may later supply bounded raster pages",
        "never needed for the base parse",
        "Neither Node nor pdfjs is a product dependency",
        "Spike exit criterion",
        "PDFium/pdfjs are not fallback semantic parsers",
        "At most 1,000,000 distinct object IDs",
        "32 MiB per stream and 256 MiB cumulative decoded bytes",
        "36,000,000 pixels per image",
        "P2a, Luna A",
        "P2b, Luna B",
        "coordinator core/Python checkpoint",
    )
    assert all(item in text for item in required)


def test_parser_wave_plans_preserve_oracle_and_parity_boundaries() -> None:
    for plan in (HWPX, PDF):
        text = plan.read_text(encoding="utf-8")
        assert "read-only" in text
        assert "oracle" in text
        assert "kordoc/" in text
        assert "ParsedDocument" in text
        assert "real-document" in text or "real-PDF" in text
        assert "successful real-document parity numerator" in text or (
            "parser-success numerator" in text
        )


def _section(text: str, start: str, end: str) -> str:
    match = re.search(
        rf"^#{{2,3}} {re.escape(start)}.*?(?=^#{{2,3}} {re.escape(end)})",
        text,
        flags=re.MULTILINE | re.DOTALL,
    )
    assert match is not None
    return match.group(0)


def test_hwpx_first_task_has_an_executable_scaffold_precondition() -> None:
    text = HWPX.read_text(encoding="utf-8")
    h0 = _section(text, "Task H0:", "Task H1a:")
    assert "feature/parser-wave-scaffold" in text
    assert "`crates/kordoc-hancom/Cargo.toml`" in h0
    assert "`src/lib.rs`" in h0
    assert "`tests/hwpx_integration.rs`" in h0
    assert "test -p kordoc-hancom --test hwpx_integration --locked" in h0
    assert "nested `[workspace]`" in text
    assert "tests/hwpx_security.rs" not in text
    assert "tests/hwpx_structure.rs" not in text
    assert "colocated module test files" in text
    assert "H1a and H2a run concurrently" in text
    assert "H1b consumes H2a's bounded XML reader" in text
    assert "H2b consumes H1a's metered `Package` reader" in text
    assert "neither starts before both prerequisite commits are green" in text


def test_pdf_worker_interfaces_are_compilable_and_budget_aware() -> None:
    text = PDF.read_text(encoding="utf-8")
    p2b = _section(text, "Task 2:", "Task 3:")
    assert "colocated `src/{geometry,layout}/tests.rs`" in p2b
    assert "tests/pdf_geometry.rs" not in p2b
    assert "budget: &mut PdfBudget" in p2b
    assert "Result<Vec<IrBlock>, KordocError>" in p2b
    assert "scaffolded `src/lib.rs` declares compilable module shells" in text
    assert "transfers exclusively to P2a" in text
    assert "src/table/tests.rs" in text
    assert "colocated `src/{image,quality}/tests.rs`" in text
    assert "tests/pdf_{tables,table_security}.rs" not in text
    assert "tests/pdf_{assets,quality}.rs" not in text


def test_shared_golden_manifest_has_one_coordinator_owner() -> None:
    hwpx = HWPX.read_text(encoding="utf-8")
    pdf = PDF.read_text(encoding="utf-8")
    assert "Coordinator only" in hwpx
    assert "`tests/golden/document-manifest.json`" in hwpx
    assert "coordinator alone" in pdf
    assert "format-local candidate fixture entries" in pdf
    assert (
        "SOL edits `crates/kordoc-pdf/src/parser.rs`, `tests/golden/document-manifest.json`"
        not in pdf
    )


def test_scaffold_sequence_matches_master_and_child_checkpoints() -> None:
    master = (PLANS / "2026-09-30-product-port-implementation-plan.md").read_text(
        encoding="utf-8"
    )
    hwpx = HWPX.read_text(encoding="utf-8")
    pdf = PDF.read_text(encoding="utf-8")
    assert "protected coordinator scaffold registers both minimal crates" in master
    assert "before worker branches" in master
    assert "Verify the scaffolded workspace membership" in hwpx
    assert "verifies the scaffolded workspace membership" in pdf
    assert "adds workspace member/dependency pins" not in pdf
