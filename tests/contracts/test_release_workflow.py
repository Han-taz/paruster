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
    assert "guard:" in text
    assert "outputs:" not in text.split("  guard:", maxsplit=1)[1].split(
        "  build-wheels:", maxsplit=1
    )[0]
    assert "github.event.inputs.ref" not in text
    assert 'test "$GITHUB_REF" = refs/heads/main' in text
    assert "needs.resolve.outputs.commit" not in text
    assert "outputs.commit" not in text
    assert not re.search(r"(?m)^\s+ref:", text)
    assert text.count("uses: actions/checkout@") >= 3
    for immutable_commit_use in (
        "name: rc-wheel-${{ matrix.id }}-${{ github.sha }}",
        "name: rc-sdist-${{ github.sha }}",
        "name: ${{ matrix.artifact }}-${{ github.sha }}",
        "name: sbom-${{ matrix.artifact }}-${{ github.sha }}",
        "SOURCE_COMMIT: ${{ github.sha }}",
        "name: rc-checksums-${{ github.sha }}",
    ):
        assert immutable_commit_use in text
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
    sdist_job = text.split("  build-sdist:", maxsplit=1)[1].split(
        "  sboms:", maxsplit=1
    )[0]
    assert "args: --out dist" in sdist_job
    assert "args: --locked" not in sdist_job
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
    assert "needs: [guard, build-wheels, build-sdist, sboms, checksums]" in (
        attestation_job
    )
    assert "actions/attest@" in attestation_job
