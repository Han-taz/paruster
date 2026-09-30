---
id: 2026-09-30-hwpx-h4-local
date: 2026-09-30
status: local-candidate
component: hwpx-python
issue: null
pr: null
commit: null
related_ssot: [docs/SSOT/components/hwpx.md, docs/SSOT/contracts/python-api.md, docs/SSOT/migration/status.md]
related_decisions: []
---

# Local HWPX H3/H4 integration and allocation follow-up

## Context

Local development continued in writable, isolated branches while the session
could not publish GitHub writes. No direct main push or protected-flow bypass
occurred. The source checkout and its Python environment were not modified.
The ignored migration oracle was research-only, never an implementation,
packaging, test-runtime, or CI dependency.

## Work performed

H3 joined the separately reviewed package/crypto and XML/table/image slices as
private source-neutral parse, metadata and validation entry points. The frozen
candidate is `5dc75440d6f1a94947e9431d3880850250f69bf5`. Selected-page filtering
occurs before image resolution; full parsing also sweeps unreferenced BinData
assets. Core, not Hancom, owns Markdown/page projections.

Coordinator-owned H4 adds the public Hancom facade, HWPX core registry adapter,
strict format-specific detection, Python APIs and immutable validation models.
Native results serialize directly through a capped writer rather than first
duplicating images into an unbounded JSON value. Stable native output failures
remain serializable failures through `try_parse`.

SOL coordinated two disjoint Luna workers on a separate lowering allocation
meter. Section text/spans/notes/outline/layout and table grid/anchors/caption/
nested-cell flattening share an inclusive 256 MiB pre-allocation budget.
Package plaintext, XML, image-output, logical-cell and recursion limits remain
independent. Root review required follow-up for transient numeral/decorated-note
strings and layout/diagnostic allocations; a green semantic test alone was not
accepted as evidence that the memory audit was complete.

## Evidence

The H4 API/model tests had genuine missing-API/model RED runs before their
implementations. Earlier crypto and H3 slices were not universally implemented
red-first; their existing process deviation is preserved, not retroactively
called TDD. Encryption API tests exercised existing crypto rather than claiming
a new crypto RED cycle. The span/newline full-result differences, nested-cell
flattening, borrowed image reference, and reduced-cap allocation regressions
were reproduced before their corresponding follow-up changes.

Before the allocation follow-up, a newly built macOS arm64
`cp310-abi3` wheel was installed into an isolated Python 3.12 environment and
passed 153 Python tests. The imported package was the installed wheel, not the
source package. The latest rebuilt source boundary passed 252 Python/contract
tests. Seven unchanged complete H0 semantic captures now match with no
normalization; a separate test verifies all twelve committed generated inputs
against the original pinned H0 SHA-256 digests. No oracle answer or evaluator
was rewritten. The fixture ZIPs originate entirely in the CC0 Rust recipes.

The Hancom integration test binary path-includes private module unit tests.
Its execution count therefore includes duplicate unit cases: 98 library unit
tests plus 117 integration-binary executions means 98 unit and 19 distinct
integration cases, not 215 distinct tests. Fresh final join/workspace/wheel
verification is required after the last allocation review changes.

## Outcome

Executable local HWPX candidate, not a merged/hosted capability. The protected
document manifest numerator remains zero. Malformed-section diagnostic text
still differs from the captured oracle; approved depth/record security
differences remain explicit. The ODF compressed-versus-oracle-decompressed
checksum convention gap is retained in the earlier H1b record and component
page. Other document formats and all 17 MCP handlers remain pending.

## Follow-ups

Finish the transient-allocation review and independently rerun the workspace,
static/document gates and fresh installed-wheel tests. Publish focused feature
PRs only when GitHub writes are available; require hosted gates, review and
squash merge before capability promotion. This local record is not a substitute
for Python 3.10–3.14 or six-platform hosted CI evidence.
