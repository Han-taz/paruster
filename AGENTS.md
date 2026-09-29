# paruster agent guide

paruster is a Rust document-processing core exposed as a Python 3.10+ library and an MCP server. The local `kordoc/` directory is a migration oracle only and must never be committed, packaged, or required at runtime.

## Read first

- Current project truth and navigation: `docs/SSOT/README.md`
- Approved migration design: `docs/SSOT/migration/2026-09-29-rust-python-port-design.md`
- Historical work log: `docs/WIKI/README.md`

## Rules

- Use GitHub Flow. Never push implementation or documentation directly to `main`.
- Every change goes through a focused branch, PR, required CI, review, and squash merge.
- Update relevant SSOT pages in the same PR as behavior or architecture changes.
- WIKI entries are append-only. Correct history with a later entry instead of rewriting it.
- Shared contracts (`IR`, errors, Python API, MCP schemas) require coordinator approval.
- Keep feature ownership boundaries explicit so parallel workers do not edit the same files.
- Do not weaken fixtures, scoring, security limits, or quality gates to make a test pass.

## Temporary migration oracle

- `kordoc/` is ignored from the repository root.
- It may be read locally for parity research.
- New code, tests, packaging, and CI must not depend on its presence.
- No file beneath it may appear in Git, wheels, sdists, or CI artifacts.
