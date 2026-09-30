from __future__ import annotations

from pathlib import Path

ROOT = Path(__file__).parents[2]


def test_shared_table_builder_keeps_a_source_neutral_dependency_boundary() -> None:
    manifest = (ROOT / "crates/kordoc-tables/Cargo.toml").read_text(encoding="utf-8")
    dependencies = manifest.split("[dependencies]", 1)[1].strip().splitlines()
    assert dependencies == ['kordoc-ir = { path = "../kordoc-ir", version = "=0.1.0" }']
    for consumer in ("kordoc-core", "kordoc-hancom"):
        text = (ROOT / f"crates/{consumer}/Cargo.toml").read_text(encoding="utf-8")
        assert (
            'kordoc-tables = { path = "../kordoc-tables", version = "=0.1.0" }' in text
        )
    assert "kordoc-core" not in (ROOT / "crates/kordoc-hancom/Cargo.toml").read_text(
        encoding="utf-8"
    )


def test_moved_table_code_remains_in_the_required_coverage_gate() -> None:
    workflow = (ROOT / ".github/workflows/ci.yml").read_text(encoding="utf-8")
    assert (
        "-p kordoc-ir -p kordoc-core -p kordoc-tables --locked --fail-under-lines 80"
        in workflow
    )
