---
id: 2026-09-30-hwpml-collision-captures
date: 2026-09-30
status: recorded
component: hancom
issue: null
pr: null
commit: null
related_ssot: [docs/SSOT/components/hwpml.md, docs/SSOT/migration/status.md]
related_decisions: [Freeze source ambiguity evidence without changing private attachment policy.]
---

# Separate collision observations

Three authored CC0 inputs and five complete public source results record
unmatched nested-block omission, wrong-cell repeated-text fallback, trailing
column options and unchanged-owner blank collisions. Recipe/input/capture
hashes and provenance are recorded in the new corpus README and indexed plan.
The offline contract independently pins every byte, inventory and case order,
regenerates without the oracle, rejects tampering/extra inputs and checks
structural semantics. Independent review is clean. Native behavior, protected
H0 inputs/answers and scoring remain untouched. Unmatched and decoy source
successes intentionally diverge from private Rust `UNSUPPORTED_FORMAT`;
only the blank control agrees with preserved owner structure.

## Fresh local verification

On 2026-09-30, the collision recipe was made executable and Ruff-formatted;
its updated SHA-256 is
`fe273c1d5792fd109cb363c0963378ecf2fba6522dc053d1194aac618021e9ae`. The
three fixture byte hashes and complete capture JSONL hash remain unchanged.
Under `core.autocrlf=true`, Git reported `i/lf w/lf attr/-text` for all three
XML files, the recipe and the JSONL, and clean-filter object hashes matched the
index blobs.

Fresh Rust gates on the merged branch passed with Rust 1.97.0: workspace tests
(`cargo test --workspace --locked`), all-target/all-feature strict Clippy,
the dedicated PDF worker-tests feature suite, formatting and warning-denied
workspace rustdoc. Python Ruff check/format, all 113 contract tests, and the
documentation link/index check passed. Release base artifacts passed
`scripts/check_artifacts.py`; the CPython 3.10 installed base wheel ran the
complete `tests/` suite with 360 passed and 10 subtests, with one expected
duplicate-ZIP-name warning from the hostile archive test. The CPython 3.10
sdist installed and passed the PDF format-detection smoke check. These are
local macOS arm64 builds, not evidence for other wheel targets.

## Protected metadata mainline integration

Merged main34 `bc61d32` and retained all authored collision bytes. Fresh locked
workspace tests and strict all-feature Clippy pass. A freshly rebuilt base
wheel passes artifact audit and 362 isolated Python/helper cases plus ten
subtests. Ruff, typing and indexed documentation checks pass. Shared table and
Hancom parser source remain byte-identical to main.
