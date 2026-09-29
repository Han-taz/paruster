from __future__ import annotations

import re
from pathlib import Path

ROOT = Path(__file__).parents[2]
WORKFLOWS = ROOT / ".github" / "workflows"
ANY_ACTION = re.compile(r"uses:\s+[^\s@]+@([^\s#]+)")


def _workflow(name: str) -> str:
    return (WORKFLOWS / name).read_text(encoding="utf-8")


def _assert_actions_are_pinned(text: str) -> None:
    actions = ANY_ACTION.findall(text)
    assert actions, "workflow must use at least one action"
    assert all(re.fullmatch(r"[0-9a-f]{40}", ref) for ref in actions)


def _assert_gate_rejects_non_success(text: str, gate: str, needs: str) -> None:
    gate_body = text.split(f"  {gate}:", maxsplit=1)[1]
    assert "if: ${{ always() }}" in gate_body
    assert f"needs: [{needs}]" in gate_body
    assert '.result == "success"' in gate_body


def test_checkouts_never_persist_credentials() -> None:
    for workflow in WORKFLOWS.glob("*.yml"):
        text = workflow.read_text(encoding="utf-8")
        checkout_count = text.count("uses: actions/checkout@")
        assert text.count("persist-credentials: false") == checkout_count, workflow


def test_ci_workflow_has_stable_required_gate_and_supported_python_matrix() -> None:
    text = _workflow("ci.yml")
    _assert_actions_are_pinned(text)
    assert all(event in text for event in ("pull_request:", "push:", "merge_group:"))
    assert "contents: read" in text
    assert "ci-gate:" in text
    _assert_gate_rejects_non_success(
        text, "ci-gate", "contracts, rust, python, golden, docs, coverage"
    )
    assert "ubuntu-24.04" in text
    assert "maturin-version: v1.15.0" in text
    assert "manylinux: 2_28" in text
    for version in ("3.10", "3.11", "3.12", "3.13", "3.14"):
        assert version in text
    for job in ("contracts", "rust", "python", "golden", "docs", "coverage"):
        assert job in text
    assert "--fail-under-lines 80" in text


def test_wheel_workflow_covers_all_native_targets_without_publishing() -> None:
    text = _workflow("wheels.yml")
    _assert_actions_are_pinned(text)
    assert "wheels-gate:" in text
    _assert_gate_rejects_non_success(text, "wheels-gate", "wheels")
    expected_cells = {
        ("ubuntu-24.04", "x86_64-unknown-linux-gnu"),
        ("ubuntu-24.04-arm", "aarch64-unknown-linux-gnu"),
        ("windows-2025", "x86_64-pc-windows-msvc"),
        ("windows-11-arm", "aarch64-pc-windows-msvc"),
        ("macos-15-intel", "x86_64-apple-darwin"),
        ("macos-15", "aarch64-apple-darwin"),
    }
    for runner, target in expected_cells:
        assert runner in text
        assert target in text
    assert "cp310-abi3" in text
    assert "maturin-version: v1.15.0" in text
    assert "manylinux: auto" not in text
    assert text.count("manylinux: 2_28") == 2
    assert "scripts/check_artifacts.py" in text
    assert "publish" not in text.lower()


def test_fuzz_workflow_is_bounded_and_reports_a_stable_gate() -> None:
    text = _workflow("fuzz.yml")
    _assert_actions_are_pinned(text)
    assert "nightly-2026-09-20" in text
    assert "cargo-fuzz --version 0.13.2" in text
    assert "detect_format" in text
    assert "zip_preflight" in text
    assert "-max_total_time=30" in text
    assert "-max_total_time=900" in text
    assert "fuzz-gate:" in text
    _assert_gate_rejects_non_success(text, "fuzz-gate", "fuzz")
