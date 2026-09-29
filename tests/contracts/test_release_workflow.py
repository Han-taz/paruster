from __future__ import annotations

import re
from pathlib import Path

ROOT = Path(__file__).parents[2]
RELEASE = ROOT / ".github" / "workflows" / "release.yml"
ACTION_REF = re.compile(r"uses:\s+[^\s@]+@([^\s#]+)")


def _workflow() -> str:
    return RELEASE.read_text(encoding="utf-8")


def test_release_candidate_is_manual_non_publishing_and_reproducible() -> None:
    text = _workflow()
    assert "workflow_dispatch:" in text
    assert "pull_request:" not in text
    assert "\n  publish:" not in text
    assert "contents: read" in text
    assert "maturin-version: v1.15.0" in text
    assert "syft-version: v1.52.0" in text
    assert "resolve:" in text
    assert "git rev-parse HEAD" in text
    assert "github.event.inputs.ref" not in text
    assert "ref: ${{ github.sha }}" in text
    assert 'test "$GITHUB_REF" = refs/heads/main' in text
    assert text.count("ref: ${{ needs.resolve.outputs.commit }}") >= 3
    assert "publish" not in "\n".join(
        line for line in text.splitlines() if re.match(r"^  [a-z].*:$", line)
    )


def test_release_candidate_rebuilds_and_verifies_every_distribution() -> None:
    text = _workflow()
    for runner, target in (
        ("ubuntu-24.04", "x86_64-unknown-linux-gnu"),
        ("ubuntu-24.04-arm", "aarch64-unknown-linux-gnu"),
        ("windows-2025", "x86_64-pc-windows-msvc"),
        ("windows-11-arm", "aarch64-pc-windows-msvc"),
        ("macos-15-intel", "x86_64-apple-darwin"),
        ("macos-15", "aarch64-apple-darwin"),
    ):
        assert runner in text
        assert target in text
    assert "command: sdist" in text
    assert "scripts/check_artifacts.py" in text
    assert "cp310-abi3" in text
    assert "spdx-json" in text
    assert "cyclonedx-json" in text
    assert "SHA256SUMS" in text
    assert "bundle/SHA256SUMS\n            bundle/SOURCE_COMMIT" in text
    assert "expected_sboms" in text
    assert "len(distributions) == 7" in text
    assert "sha256sum -c SHA256SUMS" in text
    assert "$RUNNER_TEMP/SHA256SUMS" in text
    for artifact in (
        "rc-wheel-linux-x86_64",
        "rc-wheel-linux-aarch64",
        "rc-wheel-windows-x86_64",
        "rc-wheel-windows-aarch64",
        "rc-wheel-macos-x86_64",
        "rc-wheel-macos-aarch64",
        "rc-sdist",
    ):
        assert text.count(artifact) >= 2


def test_release_actions_and_elevated_permissions_are_hardened() -> None:
    text = _workflow()
    refs = ACTION_REF.findall(text)
    assert refs
    assert all(re.fullmatch(r"[0-9a-f]{40}", ref) for ref in refs)
    assert "attestations: write" in text
    assert "id-token: write" in text
    assert "artifact-metadata: write" in text
    attestation_job = text.split("  attest:", maxsplit=1)[1]
    before_attestation = text.split("  attest:", maxsplit=1)[0]
    assert "attestations: write" not in before_attestation
    assert "id-token: write" not in before_attestation
    assert "artifact-metadata: write" not in before_attestation
    assert "needs: [resolve, build-wheels, build-sdist, sboms, checksums]" in (
        attestation_job
    )
    assert "actions/attest@" in attestation_job
