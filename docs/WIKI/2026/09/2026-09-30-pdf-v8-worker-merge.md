---
id: 2026-09-30-pdf-v8-worker-merge
date: 2026-09-30
status: merged
component: pdf
issue: null
pr: https://github.com/Han-taz/paruster/pull/25
commit: 43c30fd2239fb762eba4530e61bb46802176a663
related_ssot: [docs/SSOT/components/pdf.md, docs/SSOT/migration/status.md]
related_decisions: [2026-09-30-pdf-v8-decision]
---

# Private PDF native-worker checkpoint merged

PR [#25](https://github.com/Han-taz/paruster/pull/25) merged by protected squash
on September 30 at 11:25:17 UTC as
`43c30fd2239fb762eba4530e61bb46802176a663`. Final implementation head
`473ed9045abfdd75c50a77fc7f0abc2d23c7fd72` passed all required checks:
[CI](https://github.com/Han-taz/paruster/actions/runs/36707432905),
[Security/CodeQL](https://github.com/Han-taz/paruster/actions/runs/36707432819),
[six default native wheels](https://github.com/Han-taz/paruster/actions/runs/36707432850),
and [bounded fuzzing](https://github.com/Han-taz/paruster/actions/runs/36707432843).
Independent scoped review and local verification are recorded in the earlier
[worker gate entry](2026-09-30-pdf-v8-worker-gates.md).

The private Rust worker contains native crashes and enforces its framed I/O,
deadline and concurrency protocol around the embedded V8/PDF.js probe.
Default wheels do not include this executable. OS memory containment,
six-target installed worker-wheel execution, resource factories, source-neutral
IR/layout and full PDF result parity remain pending. No public PDF capability
or protected parity numerator is promoted.

The user subsequently reconfirmed the language boundary: PDF.js remains
JavaScript executed inside embedded V8; execution management and the remainder
of the port are Rust. JavaScript used for upstream PDF parsing is intentional
under that explicit direction.
