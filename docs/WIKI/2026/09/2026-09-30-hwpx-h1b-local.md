---
id: 2026-09-30-hwpx-h1b-local
date: 2026-09-30
status: local-candidate
component: hwpx
issue: null
pr: null
commit: ffe855bd5de41e35b2d633b36c7d377704ec225d
related_ssot: [docs/SSOT/components/hwpx.md]
related_decisions: []
---

# Local HWPX H1b candidate

## Context

Isolated stacked development continued after the reviewed H1a/H2a join while
GitHub writes were unavailable. No protected integration bypass. Luna owned
package/crypto/metadata/validator; SOL reviewed; coordinator owned shared docs.

## Work performed

Commits a3b7a089d084d732b741953f2eb6ec5cebda273d and
ffe855bd5de41e35b2d633b36c7d377704ec225d add bounded AES-CBC/PBKDF decryption,
preappend metering/PRF rollback, atomic plaintext cache, metadata-only and
ordered structural validation. Review corrected URI validation, metadata
precedence, raw StreamEnd, arbitrary 0..16-byte alignment tails and password
validation errors.

## Evidence

Worker and independent SOL locked Hancom runs passed 72 unit plus 8 integration
tests. Strict Clippy, rustfmt and diff checks passed; final scoped review CLEAN.
Named cases cover password gating, both PRFs, iteration/parameter limits,
shared meter/rollback, atomic install, metadata isolation and validator counts.

The worker disclosed that not all implementation followed red-first TDD.
Targeted real failures exist, but later baseline/counterfactual checks must not
be described as historical red-first execution. This deviation remains in the
record; new review fixes require genuine red/green regressions.

The approved oracle hashes decompressed first-1KiB plaintext; ODF specifies
compressed bytes. The standards gap remains for integration review. Synthetic
zero alignment bytes are not imposed as a runtime padding contract. Unsupported
algorithm lookalikes, including an ellipsis placeholder, are intentionally
rejected rather than accepted through suffix matching.

## Outcome

Locally reviewed private candidate for a later feature PR. No H3/H4 public API,
Python behavior, hosted CI or parser parity completion claim.
