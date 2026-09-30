---
id: 2026-09-30-pdf-v8-text-document-merge
date: 2026-09-30
status: merged
component: pdf
issue: null
pr: https://github.com/Han-taz/paruster/pull/32
commit: 762ddde4050660abaaa815889ed76da21a589612
related_ssot: [docs/SSOT/components/pdf.md, docs/SSOT/migration/status.md]
related_decisions: [Private bounded PDF.js evidence transfers through Rust-owned V8 process and DTO limits.]
---

# Ordered PDF evidence passes six-target protected merge

PR [#32](https://github.com/Han-taz/paruster/pull/32) squash-merges at
2026-09-30 13:46:16 UTC as `762ddde4050660abaaa815889ed76da21a589612`.
Exact final head `7cebe6543c07dedae6d0b88e495bda8863cd10a4` passes all five
required aggregates, Rust/Python/Actions CodeQL and all six PDF worker wheels.
Each installed worker runs all three existing probes plus the complete new
kind-4 result comparison; Linux also repeats them under manylinux 2.28.
Independent runtime and protocol reviews are clean.

- [CI](https://github.com/Han-taz/paruster/actions/runs/36723004406)
- [Security](https://github.com/Han-taz/paruster/actions/runs/36723004442)
- [Native wheels](https://github.com/Han-taz/paruster/actions/runs/36723004499)
- [Bounded fuzzing](https://github.com/Han-taz/paruster/actions/runs/36723004423)
- [Six PDF worker wheels](https://github.com/Han-taz/paruster/actions/runs/36723004407)

Local evidence includes 233 scoped PDF executions, locked workspace tests,
strict all-feature Clippy/rustdoc, workflow/docs/Python gates, authored offline
recipe/result byte pins and autocrlf preservation. Fresh combined macOS ARM
worker wheel tests pass 354 Python/helper cases plus ten subtests. This private
merge preserves kinds 1/2 and public contracts. XFA/operator/annotation evidence,
Rust layout/IR/corpus parity, OS containment and ordinary worker source-build
assembly remain separate gates; no PDF production parser is registered.
