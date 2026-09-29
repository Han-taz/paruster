---
id: 2026-09-30-hwpx-h0-merge
date: 2026-09-30
status: recorded
component: hwpx
issue: null
pr: https://github.com/Han-taz/paruster/pull/16
commit: 08654585d58339c9116aa82fd6c87515364a02fe
related_ssot:
  - docs/SSOT/migration/status.md
  - docs/SSOT/migration/plans/2026-09-30-hwpx-implementation-plan.md
related_decisions:
  - HWPX H0 freezes fixtures and private contracts without exposing parser behavior.
  - Deterministic cryptographic values exist only in documented test fixtures.
---

# HWPX H0 merge evidence

## Result

PR [#16](https://github.com/Han-taz/paruster/pull/16) merged by squash as
`08654585d58339c9116aa82fd6c87515364a02fe`. It added 12 deterministic CC0
HWPX recipes, pinned input hashes, full flat oracle captures, source digests,
and the private package/error/order/budget contract. It did not add a public
parser entry point, production registration, Python behavior, or MCP behavior.

## Verification

- Eight focused HWPX integration tests passed with Rust 1.97.
- Formatting, strict crate Clippy, locked tests, and diff checks passed locally.
- A fresh read-only oracle capture matched all 12 committed JSONL records.
- Hosted [CI](https://github.com/Han-taz/paruster/actions/runs/36628087881),
  [Security](https://github.com/Han-taz/paruster/actions/runs/36628087796),
  [Native wheels](https://github.com/Han-taz/paruster/actions/runs/36628087754),
  and [bounded fuzzing](https://github.com/Han-taz/paruster/actions/runs/36628087920)
  passed, including all required gates and language-specific CodeQL jobs.

GitHub's aggregate CodeQL check initially classified four fixed cryptographic
values as critical. Each finding was confined to the deterministic CC0 test
fixture or its independent decryption test; no value was reachable from
production code. Alerts 2-5 were individually reviewed and dismissed with the
official `used in tests` reason and an audit comment pointing to the fixture
provenance. The aggregate CodeQL check then passed; no query or workflow was
disabled or weakened.

## Outcome

H1a package/security and H2a XML/section work may proceed in parallel. HWPX
parsing and the real-document parity numerator remain pending until the later
integration, Python, golden, fuzz, and protected merge gates complete.
