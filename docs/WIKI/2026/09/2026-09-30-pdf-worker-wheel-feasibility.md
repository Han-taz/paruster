---
id: 2026-09-30-pdf-worker-wheel-feasibility
date: 2026-09-30
status: locally-verified
component: pdf
issue: null
pr: null
commit: null
related_ssot: [docs/SSOT/components/pdf.md, docs/SSOT/migration/status.md]
related_decisions: [2026-09-30-pdf-v8-decision]
---

# Stage and execute the private PDF worker in native wheels

## Scope and ownership

Luna B owns the separate six-target workflow and wheel helper/regressions;
SOL owns independent review and upstream notice-only assets; the coordinator
owns final integrated verification, SSOT/WIKI and publication. Fresh default
wheels and public PDF registration are unchanged. No executable is committed.

The workflow compiles the optional Rust/V8/PDF.js worker for the six existing
targets, stages it adjacent to the package and includes its notices. ELF,
PE and thin Mach-O machine headers are audited directly; archive mode and
fixed worker path are checked before clean installed execution. Linux builds
inside manylinux 2.28 and repeats installed probes inside a baseline container.
The runner-provided Windows ARM64 CPython version is 3.12; 3.10 there remains
unproven. This gate does not implement PEP 517/sdist worker compilation or
production worker discovery.

## RED and local installed-wheel evidence

The prior default abi3 wheel fails the actual missing-worker audit. A first
worker wheel then fails the newly added missing-notice audit. After staging the
worker and all 33 pinned notices, a macOS arm64 wheel passes both archive and
artifact audits and actual installed execution in clean CPython 3.10.19.

The coordinator rebuilt that wheel after the HWPX #26 merge and reran all
**335** installed tests (320 existing Python/contract/parity plus 15 tooling
regressions), followed by strict frame probes extracting exact
`V8 PDF.js probe` and `한글🧪`. The binary is **61,976,928** bytes with mode
0755. Mach-O load commands declare macOS 11.0 and only libSystem as a dynamic
dependency, matching this local wheel tag. Generated source staging directories
were removed after verification.

## Review and notice provenance

Independent scoped review is CLEAN after fixing non-Linux wheel construction,
platform suffix selection and snake_case frame fields, then exact notice
inventory, duplicate ZIP-member rejection and all installed-notice hash checks.
Fifteen helper regressions, workflow/security contract checks, Ruff, mypy,
actionlint and zizmor pass locally without new exceptions.

The manifest pins 33 notice files totaling **221,177** bytes. The coordinator
independently fetched 32 pinned release/commit URLs and matched their hashes.
The initially recorded PDF.js GitHub URL returned 404 and was corrected to
the published npm tarball member. Its pinned SHA-512 and `package/LICENSE`
SHA-256 match the existing upstream asset manifest and notice bytes. No notice
bytes were changed during that correction. The notice manifest SHA-256 is
`30e668bf0a17ea439761f01e1d0cf3226c8828660fb429a58422b2c757c9ea65`.
The identified native notice bundle is not a blanket Rust dependency audit.

The full staged whitespace advisory reports upstream trailing spaces/blank EOF
in five verbatim notice files. Those bytes intentionally remain unchanged and
match independently fetched source hashes. Authored code, docs and provenance
pass the staged whitespace check. No Git attributes, global check suppression
or existing CI quality gate is changed to hide this upstream data.

## Remaining gates

All six hosted worker-wheel targets must pass before merging this feasibility
checkpoint. OS memory/process-tree containment, calibrated RSS, trusted fixed
production resolver, ordinary source-build staging, PDF resource factories,
metadata/encryption/operators/layout/IR and full corpus/option parity remain
pending. Neither the production capability nor protected success numerator is
promoted by this packaging evidence.
