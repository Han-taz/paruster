from __future__ import annotations

import re
from pathlib import Path

ROOT = Path(__file__).parents[2]
OPERATIONS = ROOT / "docs" / "SSOT" / "operations"
DEVELOPMENT = OPERATIONS / "development.md"
RELEASE = OPERATIONS / "release.md"
INDEX = OPERATIONS / "README.md"


def _read_required(path: Path) -> str:
    assert path.is_file(), (
        f"required operations page is missing: {path.relative_to(ROOT)}"
    )
    return path.read_text(encoding="utf-8").casefold()


def test_development_guide_has_reproducible_bootstrap_and_full_local_gate() -> None:
    development = _read_required(DEVELOPMENT)
    required = (
        "uv python install 3.10",
        "uv sync --all-groups --locked --python 3.10",
        "pyo3_python",
        "export pyo3_python=",
        "abi3-py310",
        "uv run --python 3.10 maturin develop",
        "cargo +1.97.0 fmt --all -- --check",
        "cargo +1.97.0 clippy --workspace --all-targets --all-features --locked -- -d warnings",
        "cargo +1.97.0 test --workspace --locked",
        "cargo +1.97.0 doc --workspace --no-deps --locked",
        "cargo-llvm-cov 0.9.1",
        "cargo +1.97.0 install cargo-llvm-cov --version 0.9.1 --locked",
        "--fail-under-lines 80",
        "cargo-fuzz 0.13.2",
        "cargo +nightly-2026-09-20 install cargo-fuzz --version 0.13.2 --locked",
        "nightly-2026-09-20",
        "-max_total_time=30",
        "ruff check python tests scripts",
        "ruff format --check python tests scripts",
        "mypy python/kordoc scripts",
        "pytest tests/contracts tests/parity tests/python -q",
        "maturin build --release",
        "scripts/check_artifacts.py",
        "scripts/check_docs.py",
        "actionlint",
        "actionlint 1.7.12",
        "zizmor",
        "zizmor 1.30.1",
        "cargo deny",
        "cargo-deny 0.20.2",
        "cargo audit --file cargo.lock",
        "cargo-audit 0.22.2",
    )
    assert all(item in development for item in required)
    assert "zip_preflight" in development and "detect_format" in development


def test_release_guide_states_nonpublishing_compatibility_and_evidence_gates() -> None:
    release = _read_required(RELEASE)
    required = (
        "non-publishing",
        "workflow_dispatch",
        "immutable",
        "commit sha",
        "compatibility manifest",
        "planned",
        "all 17 tools",
        "mcp inventory",
        "currently marks all 17 tools as `planned`",
        "two consecutive",
        "cp310-abi3",
        "linux x86_64",
        "linux aarch64",
        "windows x86_64",
        "windows arm64",
        "macos x86_64",
        "macos arm64",
        "untested cross-product",
        "sha-256",
        "spdx",
        "cyclonedx",
        "attestation",
        "protected `pypi` environment",
        "trusted publishing",
        "same immutable source commit",
        "40-character commit sha",
        "release-candidate workflow installs and smoke-tests each target wheel on cpython 3.14",
        "native wheels ci smoke-tests cpython 3.10 and 3.14",
    )
    assert all(item in release for item in required)
    assert (
        "no pypi publish job" in release
        or "publishing is intentionally absent" in release
    )
    assert release.count("| cpython 3.14 |") == 6
    assert "| cpython 3.10 and 3.14 |" not in release


def test_operations_index_links_each_page_exactly_once() -> None:
    index = _read_required(INDEX)
    destinations = re.findall(r"\]\(([^)]+)\)", index)
    for page in ("development.md", "release.md"):
        assert destinations.count(page) == 1
