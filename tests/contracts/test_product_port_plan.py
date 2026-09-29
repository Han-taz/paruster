from __future__ import annotations

import json
import re
from pathlib import Path

ROOT = Path(__file__).parents[2]
PLAN = (
    ROOT
    / "docs"
    / "SSOT"
    / "migration"
    / "plans"
    / "2026-09-30-product-port-implementation-plan.md"
)


def test_product_port_plan_covers_frozen_public_and_mcp_contracts() -> None:
    text = PLAN.read_text(encoding="utf-8")
    public_api = json.loads(
        (ROOT / "contracts" / "public-api.json").read_text(encoding="utf-8")
    )
    mcp_tools = json.loads(
        (ROOT / "contracts" / "mcp-tools.json").read_text(encoding="utf-8")
    )

    assert "# Rust/Python Full Product Port Implementation Plan" in text
    assert "REQUIRED SUB-SKILL" in text
    assert "Node removal gate" in text
    assert "CPython 3.10-3.14" in text
    assert "64,745" in text
    assert "238" in text
    for entry in public_api["entries"]:
        assert f"`{entry['python_name']}`" in text
    for tool in mcp_tools["tools"]:
        assert f"`{tool['name']}`" in text


def test_product_port_plan_defines_parallel_crate_ownership_and_gates() -> None:
    text = PLAN.read_text(encoding="utf-8")
    required = (
        "crates/kordoc-hancom/",
        "crates/kordoc-pdf/",
        "crates/kordoc-office/",
        "crates/kordoc-transform/",
        "crates/kordoc-render/",
        "crates/kordoc-mcp/",
        "parser seam",
        "oracle parity",
        "cargo +1.97.0 test --workspace --locked",
        "pytest tests/contracts tests/parity tests/python -q",
        "two consecutive full release-candidate workflows",
    )
    assert all(item in text for item in required)


def _task_section(text: str, task: int) -> str:
    match = re.search(
        rf"^### Task P{task}:.*?(?=^### Task P\d+:|^## Completion definition)",
        text,
        flags=re.MULTILINE | re.DOTALL,
    )
    assert match is not None, f"missing P{task} task section"
    return match.group(0)


def test_product_port_plan_encodes_reviewed_dependency_invariants() -> None:
    text = PLAN.read_text(encoding="utf-8")
    for task in range(20):
        assert len(re.findall(rf"^### Task P{task}:", text, flags=re.MULTILINE)) == 1

    for task in (1, 2, 3):
        assert "depends on P7" in _task_section(text, task)
    assert "depends on P1, P4, P7, and P8" in _task_section(text, 10)
    assert "depends on P9-P12 and P14" in _task_section(text, 16)

    crate_owners = {
        8: "crates/kordoc-transform/",
        9: "crates/kordoc-forms/",
        10: "crates/kordoc-roundtrip/",
        11: "crates/kordoc-redact/",
        12: "crates/kordoc-generate/",
    }
    for task, crate in crate_owners.items():
        assert crate in _task_section(text, task)


def test_each_mcp_tool_is_assigned_to_one_implementation_task() -> None:
    text = PLAN.read_text(encoding="utf-8")
    implementation = "\n".join(_task_section(text, task) for task in range(14, 18))
    assignment_lines = "\n".join(
        line
        for line in implementation.splitlines()
        if line.startswith("**Assigned MCP tools:**")
    )
    tools = json.loads(
        (ROOT / "contracts" / "mcp-tools.json").read_text(encoding="utf-8")
    )["tools"]
    for tool in tools:
        assert assignment_lines.count(f"`{tool['name']}`") == 1
