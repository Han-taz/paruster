---
id: 2026-09-30-pdf-worker-windows-checkout
date: 2026-09-30
status: locally-verified
component: pdf
issue: null
pr: https://github.com/Han-taz/paruster/pull/27
commit: null
related_ssot: [docs/SSOT/components/pdf.md, docs/SSOT/migration/status.md]
related_decisions: []
---

# Preserve pinned worker assets during Windows checkout

The first [six-target worker workflow](https://github.com/Han-taz/paruster/actions/runs/36711328481)
passes both Linux and both macOS targets, including actual installed worker
probes. Both Windows targets fail at the first helper test, before Rust build:
Abseil's source notice hash differs after Git converts LF to CRLF. The gate
correctly fails; neither Windows target is skipped.

The helper restores CRLF to LF only when the restored bytes match the exact
same pinned SHA-256. It stages those verified bytes, never a relaxed hash or
modified notice. Positive newline and negative tamper regressions pass.

Coordinator checkout reproduction also finds that Git changes the ASCII
Helvetica fixture, shifting xref offsets: **596 → 629** bytes. Abseil's notice
changes **11,361 → 11,564** bytes. New targeted `-text` attributes preserve
PDF runtime assets and PDF document inputs as exact bytes. The original legacy
`tests/golden/fixtures/*.pdf binary` rule is retained. This changes checkout
line-ending conversion only; it does not disable diff/whitespace inspection
of the new upstream notices. Their recorded whitespace advisory remains.

With `core.autocrlf=true`, all **36** relevant files (33 notices, two probe PDFs,
legacy minimal PDF) now check out byte-identically. A new helper unit test pins
that policy using Git attributes. `PYTHONUTF8=1` keeps Windows probe logs capable
of printing Hangul/astral text. Independent SOL review, **18** helper tests,
Ruff, actionlint and authored diff checks pass. Required fixtures, answers,
hashes, inventory, caps and every platform/gate remain unchanged.

This is a follow-up to the initial
[wheel feasibility record](2026-09-30-pdf-worker-wheel-feasibility.md), which
correctly recorded no new attributes at its first publication. Actual Windows
worker build/execution is still a hosted verification gate after this fix.

The final isolated macOS installed environment also passes **338** existing
API/contract/parity and current tooling tests after the checkout fix. No native
code, fixture, notice bytes or source manifest hash changes in this follow-up.
