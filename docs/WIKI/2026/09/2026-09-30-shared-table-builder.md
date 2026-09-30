---
id: 2026-09-30-shared-table-builder
date: 2026-09-30
status: implementation-candidate
component: normalization
issue: null
pr: null
commit: null
related_ssot: [docs/SSOT/architecture/workspace.md, docs/SSOT/components/normalization.md, docs/SSOT/migration/plans/2026-09-30-shared-table-builder-plan.md]
related_decisions: [Extract shared table construction into an IR-only sibling; core retains projection policy.]
---

# Share table placement without a format-to-core cycle

The HWPML text candidate deliberately rejects selected tables. Its next slice
needs the existing source-neutral grid placement, but Hancom cannot import the
core builder: core already depends on Hancom. The coordinator approves an
IR-only `kordoc-tables` sibling and keeps Markdown, classification and legacy
layout-table flattening in core. Native implementation belongs to Luna A;
root owns workspace/dependencies, exact inventory, documentation and final
integration; SOL reviews the completed candidate independently.

The scaffold first fails two architecture contracts (missing exact workspace
member and missing sibling manifest); three existing contracts pass. After
registering the sibling and exact consumer versions, all five contracts pass
and an offline crate check succeeds. The contract enforces an IR-only sibling
manifest and keeps Hancom free of a core dependency. No fixture/scoring or
security limit changes.

The native RED then fails at the absent approved builder APIs. Existing core
cell/options structs and strict limits remain unchanged. New explicit metered
paths charge before fallible reserves; consumed rich cells transfer nested
payload ownership without deep clones. Cumulative logical cells are a separate
budget based on built geometry. Input payload allocation remains the caller's
responsibility. HWPML adapter integration and source-specific clamping remain
separate work; this record does not establish parser parity.

## Independent review and local native evidence

Root comparison against the exact previous core implementation matches full
serialized IR or exact error code/message over 10,000 deterministic small
grids with Unicode, blanks, collisions, spans, addressed/implicit cells and
options. This temporary verification harness is outside product/runtime and
does not depend on the migration oracle. Existing core tests remain intact.

Review catches blank collisions discarding unchanged owner blocks and the new
outer table exceeding the renderer depth boundary. Both reproduce RED and are
fixed before publication; an actual core Markdown test proves the deepest
accepted rich output and rejection of the next level. Independent SOL re-review
is clean. Six sibling and 110 core unit tests plus 22 other core executions
pass; root locked workspace tests pass.

Moved table code stays inside the unchanged required 80% coverage gate by
adding the sibling to the measured packages. Root's exact selected gate passes
91.75% line coverage (sibling 91.56%). Initial local reporting fails because
Homebrew Rust lacks llvm-profdata; using matching installed rustup LLVM tools
resolves the environment issue. A report without package selection measures
unexercised format dependencies and is not this gate. No exclusion or threshold
change is used. Both workspace lockfiles add only the local sibling, with no
external dependency upgrades.

Before candidate commit, root strict all-target/all-feature workspace Clippy,
warning-free rustdoc, fmt, Ruff (52 files), actionlint and documentation checks
also pass. The previously built resource-worker wheel passes all 345 current
Python API/contracts/parity/tooling cases on this base; a fresh candidate wheel
and latest-main integration remain before PR publication.
