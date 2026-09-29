---
id: 2026-09-30-hwpx-h1a-merge
date: 2026-09-30
status: recorded
component: hwpx
issue: null
pr: https://github.com/Han-taz/paruster/pull/18
commit: e33eab112999aebcf91e1593814bbec17bd1ea79
related_ssot:
  - docs/SSOT/migration/status.md
  - docs/SSOT/migration/plans/2026-09-30-hwpx-implementation-plan.md
related_decisions:
  - HWPX package access validates and meters the archive before consumers read members.
  - Ciphertext and decrypted plaintext use separate checked meters at the H1b join.
---

# HWPX H1a merge evidence

## Result

PR [#18](https://github.com/Han-taz/paruster/pull/18) merged by squash as
`e33eab112999aebcf91e1593814bbec17bd1ea79`. It implements the private HWPX
package reader: strict ZIP/ZIP64 central and local extent validation, canonical
and unique member names, the inclusive 500-record ceiling, CRC-checked reads,
shared actual-byte meters, bounded optional-member recovery, and deterministic
spine/numeric section ordering. The H1b bridge meters ciphertext separately
and charges decrypted chunks before append.

## Evidence

- Twelve package/security unit tests and eight H0 integration tests passed.
- Coordinator review added a depth-201 critical content-manifest regression.
- Hosted [CI](https://github.com/Han-taz/paruster/actions/runs/36629983888),
  [Security/CodeQL](https://github.com/Han-taz/paruster/actions/runs/36629983608),
  [Native wheels](https://github.com/Han-taz/paruster/actions/runs/36629983533),
  and [bounded fuzzing](https://github.com/Han-taz/paruster/actions/runs/36629983754)
  passed, including every required gate.

## Outcome

H1a is a merged private substrate. It does not expose HWPX parsing through
Rust, Python, or MCP and does not advance the real-document parity numerator.
H1b waits for the reviewed H2a XML seam before encryption, metadata, and
validation consume these package interfaces.
