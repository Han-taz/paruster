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


def test_status_separates_verified_local_gates_from_pending_work() -> None:
    status = (SSOT / "migration" / "status.md").read_text(encoding="utf-8")
    for fact in (
        "88.63% line coverage",
        "two 30-second fuzz campaigns",
        "without crashes",
        "Hosted workflows and required-check enforcement remain pending until PR #5 has run",
        "parsers",
        "OCR",
        "transformations",
        "renderers",
        "17 MCP tools",
    ):
        assert fact in status
