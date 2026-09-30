---
id: 2026-09-30-hwpx-h4-publication
date: 2026-09-30
status: candidate-review
component: hwpx-python
issue: null
pr: null
commit: null
related_ssot: [docs/SSOT/components/hwpx.md, docs/SSOT/contracts/python-api.md, docs/SSOT/migration/status.md]
related_decisions: []
---

# HWPX coordinator candidate publication

## Work performed

Recovered the H4 core adapter, strict detector-before-parser boundary, Python
parse/validation APIs and immutable models into a permanent feature worktree.
Native success serialization writes directly into a capped buffer. Added
source-neutral HWPX package/XML fuzz entry points and deterministic CC0 seeds.

Fresh review reproduced `images=False` being ignored by all three Python parse
entry points. Three failing regressions preceded the core postfilter fix.
The result image collection and recursive image payloads are now omitted after
Markdown/page projection; image placeholders and projections stay unchanged.
Nested children, table cells and captions have a core regression. The pinned
oracle defines `tables=False` as PDF-only; an HWPX regression preserves table
structure rather than introducing an incompatible table-flattening behavior.

## Evidence

Fresh source Python/contract/parity verification passes 296 tests, including
seven unchanged complete oracle result captures compared without normalization.
All twelve generated input hashes remain pinned to the original H0 recipes.
Ruff, formatting, mypy, strict workspace Clippy, documentation checks and
locked workspace tests pass. The ordinary macOS Rust test command sets the
matching Python base prefix for embedded PyO3 tests; extension-module linking
is validated through the separately built wheel.

A freshly built macOS arm64 `cp310-abi3` wheel was installed into a separate
CPython 3.10 environment. Its import path points into that environment's
site-packages, and all 296 Python/contract/parity cases pass against the
installed artifact. Wheel and sdist forbidden-path inspections pass.

Independent H4 SOL review found no remaining dispatch, serialization or
Python-model blocker for candidate publication. Full option parity remains
pending for `plain`, `htmlTables`, `scriptTags`, `keepTrailingEmptyCols`, and
`includeFieldPlaceholders`. The full document manifest and its protected
success numerator are unchanged; the seven candidate comparisons are not
misreported as a merged whole-parser capability.

Package fuzzing additionally asserts that a core-rejected ZIP cannot produce a
successful direct Hancom parse, with checked image-byte and page-evidence
bounds. XML fuzzing retains fixed sound neighbors, and a failed middle section
cannot leak partial blocks. Both strengthened targets completed local
30-second smoke campaigns without a crash. The existing four campaigns also
passed on the private prerequisite checkpoint. These are smoke checks; longer
campaigns and protected hosted gates remain required.

## Outcome

Reviewable executable HWPX Python candidate. Option gaps, malformed-section
diagnostic differences, ODF checksum interoperability and restricted-corpus
evidence remain explicit. Required hosted checks and protected squash merge
are separate checkpoints. No other parser or MCP handler is promoted here.
