# Release-candidate process

## Current policy: candidates only, no publication

The release workflow is triggered only by `workflow_dispatch` and is a non-publishing rebuild and verification of one requested source revision. It has no PyPI publish job or credential. A candidate run does not authorize a release, and no package may be uploaded to an index from this workflow.

Release eligibility is blocked until all of the following hold:

- Every compatibility entry in `contracts/public-api.json` has a reviewed disposition other than `planned`.
- All 17 tools in the frozen MCP inventory have implemented runtime handlers and passing contract tests; names listed as `planned` are not implementation evidence.
- Two consecutive complete release-candidate workflow runs pass for the same immutable source commit.

The frozen MCP inventory currently marks all 17 tools as `planned`, so the runtime gate is not met at this foundation stage.

These are cumulative gates. The compatibility manifest and MCP status are reviewed inputs; do not edit them to make a candidate eligible without the required contract-owner approval and corresponding implementation evidence.

## Candidate procedure

1. Select a fully tested commit and dispatch the release-candidate workflow with its full 40-character commit SHA in `ref`. The workflow resolves the selected ref once, records the immutable commit, and rebuilds distributions from that checkout rather than selecting artifacts from an earlier run.
2. Confirm all candidate builds, artifact scans, clean-install smoke tests, and checksum/SBOM/attestation jobs succeeded. Repeat the entire workflow once more for the same source SHA; both runs must complete successfully and consecutively before release eligibility can be considered.
3. Retain and review the exact artifacts and evidence bundle by digest. Any rebuild or source change produces a different candidate and restarts the two-run gate.

The workflow builds six `cp310-abi3` wheels and one source distribution:

| Target | Wheel target | Release-candidate smoke interpreter |
| --- | --- | --- |
| Linux x86_64 | `x86_64-unknown-linux-gnu` | CPython 3.14 |
| Linux AArch64 | `aarch64-unknown-linux-gnu` | CPython 3.14 |
| Windows x86_64 | `x86_64-pc-windows-msvc` | CPython 3.14 |
| Windows ARM64 | `aarch64-pc-windows-msvc` | CPython 3.14 |
| macOS x86_64 | `x86_64-apple-darwin` | CPython 3.14 |
| macOS ARM64 | `aarch64-apple-darwin` | CPython 3.14 |

The release-candidate workflow installs and smoke-tests each target wheel on CPython 3.14. Separately, the native wheels CI smoke-tests CPython 3.10 and 3.14 on target runners where both interpreters are available; Windows ARM64 currently has no 3.10 runner. The full CPython 3.10–3.14 behavior matrix runs on Linux x86_64. The `cp310-abi3` wheel tag plus that matrix is compatibility evidence for intermediate cross-product cells, not direct native execution on every operating-system/architecture/Python-minor combination. In particular, the older Windows ARM64 interpreter is an untested cross-product cell and must remain disclosed in candidate and release notes; do not imply that all target/version combinations were individually exercised.

## Integrity and provenance bundle

For each of the six wheels and the sdist, the candidate produces both an SPDX JSON SBOM and a CycloneDX JSON SBOM. A deterministic SHA-256 manifest covers all seven distributions and all fourteen SBOM files and records the source commit. Verify the manifest before consuming any artifact.

A separate, permission-scoped attestation job attests the immutable artifact digests only after build, scan, smoke-test, SBOM, and checksum prerequisites succeed. Keep the commit SHA, checksums, both SBOM formats, and attestations together; never substitute an artifact rebuilt outside the candidate workflow or identify an artifact only by a mutable filename or tag.

## Future publication

Publishing is intentionally absent. A future reviewed release plan must define and approve the protected `pypi` environment and its PyPI Trusted Publishing configuration before a publication job, credentials, or environment permissions are added. No manual upload, API token, or false workflow guard is an allowed shortcut around that review.
