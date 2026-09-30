---
id: 2026-09-30-pdf-v8-text-document
date: 2026-09-30
status: implementation-candidate
component: pdf
issue: null
pr: null
commit: null
related_ssot: [docs/SSOT/components/pdf.md, docs/SSOT/migration/status.md, docs/SSOT/migration/plans/2026-09-30-pdf-v8-text-document-plan.md]
related_decisions: [User-approved embedded PDF.js with Rust runtime and product ownership; private text evidence before full layout qualification.]
---

# Transfer bounded ordered page items into Rust

The coordinator approves a private raw-text DTO rather than public parser
registration. Luna B owns V8 streaming, shared-budget strict Rust DTO and a
new authored fixture; SOL owns kinds 3/4 and shared subprocess lifecycle;
Luna A independently reviews the protocol, and SOL independently reviews B.
Root owns installed-wheel helpers, workflows, byte contracts and SSOT/WIKI.
Existing kinds 1/2, fixtures, asset pins, notices and public contracts stay
unchanged. The initial focused extraction test fails against the absent DTO
API before implementation.

The new default streaming route matches pinned getTextContent for ordinary
non-XFA pages. Raw item sequence, geometry, page view/rotation and seven fixed
Info fields survive without layout normalization. U+FB03 produces ffi under
default normalization and differs when normalization is disabled. Two authored
pages include Hangul/astral text, reversed stream/x order and rotated nonzero
CropBox. The deterministic recipe SHA is
`25047936b3aa20d8fbb87f503164d627bc75c60862f4dc389b5825a027e4f993`;
2256-byte PDF SHA is
`6420221a8fbbfe4386caa18705fe79b3f450ec5893852297461fa793522c4a6b`;
1764-byte complete expected result SHA is
`24da296d2ec1b231d55c3171530c79bcb9a5847283428ed8dee469670f331f7e`.
Root's isolated offline regeneration and autocrlf=true index checkout prove
all three byte pins. New narrow -text attributes preserve recipe/result bytes
without weakening whitespace checks.

## Boundary review

Reviews find and fix UTF-16 preflight missing before UTF-8 scans, permissive
string fallback, document-wide decoder aggregation, nested extra-item traversal,
overconservative JSON boundary accounting, spoofable quota-message detection,
and a recognized six-byte kind-3 prefix paired to an old error after truncated
length. Exact inclusive JSON byte accounting, rejecting seeds, internal sticky
quota state and anchored custom decoder markers now preserve typed failures.
Kind-3 prefix truncation reproduces a real executable RED before the pairing
fix; independent re-review is clean. Old kind-1/kind-2 bytes remain covered.
Escaped JSON/key scratch may transiently allocate within the capped 4 MiB
frame; no zero-allocation claim is made.

Final scoped native feature suites include 20 DTO, 35 executable-worker and
40 supervisor cases; strict scoped Clippy/fmt pass and both independent reviews
are clean. Root helper/offline checks pass 26 tests and ten subtests; full Ruff
checks/format and documentation checks pass. The declared mypy runner is used
for the full typing gate after a packaging-only environment lacks mypy.
Fresh worker-wheel and latest-main/hosted evidence remains before publication.
Operator/font-object/annotation evidence, XFA, Rust layout/IR, options/corpus
parity and OS-calibrated process containment remain separate gates.


## Fresh local installed worker evidence

Root full default workspace tests, strict all-target/all-feature workspace
Clippy, warning-free rustdoc, fmt and eight-source mypy pass. Manager reruns
233 scoped PDF feature executions across eleven groups and builds a fresh
release arm64 worker. Its SHA is
`3361ff9ee40bed25990429bb6fbe12e3b1ca34bde2f85648c923a9a9a3c4b837`.
The fresh staged macOS ARM abi3 wheel passes worker architecture, strict 36-notice
inventory and artifact audits. Clean isolated CPython 3.10 installation passes
all four worker smokes and 351 full Python/helper tests; root also repeats the
complete kind-4 installed result comparison. Temporary package stage inputs
are preserved under ignored build storage. Latest-main and all six hosted
worker targets remain required before publication.


## Latest-main verification

The branch merges protected HWPML PR #30. Squashed resource ancestry produces
merge conflicts, resolved by a three-way merge using the reviewed identical
resource head as the synthetic base. Runtime/protocol/workflow source is
unchanged by the mainline merge; HWPML fixture/source changes are preserved.
Root repeats merged locked workspace tests, strict all-feature Clippy/rustdoc,
fmt, Ruff (53 files), actionlint, zizmor and documentation validation. A fresh
macOS ARM worker wheel passes all four installed probes and 352 isolated tests
plus ten subtests. Wheel and base sdist audits pass; the three HWPML fixture
copies retain exact bytes. Private PDF worker sources are outside the base
sdist dependency closure, as before; ordinary worker build/assembly remains a
separate production gate. No full-source-sdist worker support is claimed.


## Shared-table mainline join

Protected PR #31 is incorporated before final hosted review. Only additive
SSOT/WIKI merge conflicts need resolution; native PDF source is unchanged.
Merged locked workspace tests, all-feature Clippy/rustdoc and fmt pass.
A freshly rebuilt combined worker wheel passes 354 isolated Python/helper
cases and ten subtests; the exact installed kind-4 probe still passes.
Both artifact audits pass. Final-head six-platform gates remain pending.
