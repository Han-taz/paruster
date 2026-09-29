from pathlib import Path

ROOT = Path(__file__).parents[2]
SSOT = ROOT / "docs" / "SSOT"


def test_workspace_architecture_and_operations_are_indexed() -> None:
    index = (SSOT / "README.md").read_text(encoding="utf-8")
    normalized_index = " ".join(index.split())
    architecture = SSOT / "architecture" / "workspace.md"

    assert architecture.is_file()
    assert "[Workspace architecture](architecture/workspace.md)" in index
    for page in (
        "operations/README.md",
        "operations/development.md",
        "operations/release.md",
    ):
        assert f"]({page})" in index
    assert "## Organization" in index
    assert (
        "`architecture/` — current system boundaries and data flow" in normalized_index
    )
    assert "`operations/` — development and release procedures" in normalized_index


def test_workspace_page_describes_current_boundaries_and_data_flow() -> None:
    workspace = (SSOT / "architecture" / "workspace.md").read_text(encoding="utf-8")
    normalized_workspace = " ".join(workspace.split())
    for fact in (
        "kordoc-ir",
        "wire DTOs",
        "kordoc-core",
        "bounded format detection",
        "kordoc-python",
        "PyO3",
        "Python facade",
        "GIL",
        "bytes",
        "filesystem paths",
        "binary streams",
        "standalone `fuzz/` workspace",
        "CI workflows",
        "ignored `kordoc/` oracle",
        "runtime",
        "build",
        "package",
    ):
        assert fact in normalized_workspace


def test_status_records_remediation_ci_and_protected_branch_enforcement() -> None:
    status = (SSOT / "migration" / "status.md").read_text(encoding="utf-8")
    for fact in (
        "88.63% line coverage",
        "two 30-second fuzz campaigns",
        "without crashes",
        "PR #5 implementation head",
        "a9df485` passed",
        "PR #5 documentation head `ad86e43` also passed",
        "36601581589",
        "36601581506",
        "36601581485",
        "36601581555",
        "PR #4 remediation head `31212275` passed",
        "36603823278",
        "36603823358",
        "36603823436",
        "36603823386",
        "main-target Dependency review",
        "ruleset `24182744`",
        "strict",
        "ci-gate",
        "security-gate",
        "wheels-gate",
        "fuzz-gate",
        "CodeQL",
        "squash-only",
        "later heads must pass",
        "parsers",
        "OCR",
        "transformations",
        "renderers",
        "17 MCP tools",
        "Product parity remains pending",
    ):
        assert fact in status
    for stale_fact in (
        "pytest 9.0.3 remediation checks are pending PR #4",
        "Required-check enforcement remains pending PR #4",
        "Main-target dependency-review proof and required-check enforcement remain pending PR #4",
    ):
        assert stale_fact not in status
