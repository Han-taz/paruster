from __future__ import annotations

import re
from pathlib import Path

ROOT = Path(__file__).parents[2]
WORKFLOW = ROOT / ".github" / "workflows" / "security.yml"
POLICY = ROOT / "deny.toml"


def test_security_workflow_and_dependency_policy_are_pinned_and_gated() -> None:
    workflow = WORKFLOW.read_text(encoding="utf-8")
    policy = POLICY.read_text(encoding="utf-8")

    assert all(
        event in workflow for event in ("pull_request:", "push:", "merge_group:")
    )
    assert "schedule:" in workflow and "cron:" in workflow
    assert "permissions:\n  contents: read" in workflow
    assert "branches: [main]" in workflow

    actions = re.findall(r"uses:\s+[^\s@]+@([^\s#]+)", workflow)
    assert actions and all(re.fullmatch(r"[0-9a-f]{40}", ref) for ref in actions)

    for requirement in (
        "security-gate:",
        "cargo audit --file Cargo.lock",
        "cargo-audit 0.22.2",
        "cargo-deny 0.20.2",
        "actionlint 1.7.12",
        "zizmor 1.30.1",
        "Dependency review",
        "rust, python, actions",
    ):
        assert requirement in workflow
    gate_body = workflow.split("  security-gate:", maxsplit=1)[1]
    assert "if: always()" in gate_body
    assert (
        "needs: [codeql, cargo-audit, cargo-deny, dependency-review, actionlint, zizmor]"
        in workflow
    )
    assert 'all(.[]; .result == "success")' in gate_body
    codeql_job = workflow.split("  codeql:", maxsplit=1)[1].split(
        "  cargo-audit:", maxsplit=1
    )[0]
    assert "security-events: write" in codeql_job
    assert workflow.count("security-events: write") == 1
    for language in ("rust", "python", "actions"):
        assert language in codeql_job
    assert "build-mode: none" in codeql_job
    assert "autobuild" not in codeql_job
    assert not re.search(r"cargo\s+install|curl[^\n]*\|\s*(?:sh|bash)", workflow)
    assert workflow.count("checksum: true") == 2
    assert workflow.count("fallback: none") == 2
    assert "sha256sum --check --status" in workflow
    assert (
        "e65324f4430c2717591937edcec90ccbefaf14c174f8ec9415e03ca875b46e1a" in workflow
    )

    dependency_review_job = workflow.split("  dependency-review:", maxsplit=1)[1].split(
        "  actionlint:", maxsplit=1
    )[0]
    assert "github.base_ref == github.event.repository.default_branch" in (
        dependency_review_job
    )
    assert (
        "Dependency review only runs for pull requests targeting the default branch"
        in (dependency_review_job)
    )

    for requirement in (
        "version = 2",
        'unmaintained = "all"',
        'unsound = "all"',
        'yanked = "deny"',
        'wildcards = "deny"',
        'unknown-registry = "deny"',
        'unknown-git = "deny"',
    ):
        assert requirement in policy
    licenses = re.search(r"(?ms)^allow\s*=\s*\[(.*?)\]", policy)
    assert licenses is not None
    assert set(re.findall(r'"([^"]+)"', licenses.group(1))) == {
        "MIT",
        "Apache-2.0",
        "BSD-2-Clause",
        "BSD-3-Clause",
        "ISC",
        "Unicode-3.0",
        "Zlib",
    }
    assert 'crate = "target-lexicon@0.13.5"' in policy
    assert 'allow = ["Apache-2.0 WITH LLVM-exception"]' in policy
    assert 'crate = "syn@2.0.119"' in policy
    assert 'crate = "syn@3.0.6"' in policy
