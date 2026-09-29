---
id: 2026-09-30-pdf-substrate
date: 2026-09-30
status: recorded
component: pdf
issue: null
pr: pending
commit: pending
related_ssot:
  - docs/SSOT/components/pdf.md
  - docs/SSOT/migration/status.md
  - docs/SSOT/migration/plans/2026-09-30-pdf-implementation-plan.md
related_decisions:
  - lopdf 0.45.0 is rejected as the bounded runtime substrate.
  - The private borrowed-source reader must pass every allocation and recursion gate before PDF semantics begin.
---

# PDF bounded object-reader substrate

## Work performed

- Audited `lopdf 0.45.0` source, license, and locked dependency graph and
  rejected its eager document/object-stream path for runtime parsing.
- Added a borrowed-source reader for classic/xref-stream/incremental revisions,
  compressed objects, generation checks, encryption detection, and bounded
  unfiltered/ASCIIHex streams.
- Added checked counters and hostile fixtures for free and normal xref IDs,
  recursion/cycles, decoded bytes, stream lengths, lexical evasions, and
  attacker-proportional allocation paths.
- Added deterministic CC0 PDF generation and per-probe RSS/error evidence.

## Review history

The first green candidate failed SOL exact-diff review because free xref IDs
were inserted without charge and several token/reference arrays allocated
before a budget check. Further review found trailer string/comment encryption
bypasses, stream-payload false references, and an ObjStm index panic path. Each
finding received a failing regression before its fix. The final candidate
passed 15 object, 19 security, and two scaffold tests plus strict Clippy,
formatting, and an independent 500 MiB sparse probe.

## Outcome

Task 0 is ready for protected PR review. Only the private substrate is present;
PDF parsing, metadata, Python, parity, and MCP capability remain pending.
Filtered xref/content streams currently fail closed until later P2a work adds
their bounded decoders.
