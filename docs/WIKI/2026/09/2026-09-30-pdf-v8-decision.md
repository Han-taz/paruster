---
id: 2026-09-30-pdf-v8-decision
date: 2026-09-30
status: recorded
component: pdf
issue: null
pr: null
commit: null
related_ssot:
  - docs/SSOT/migration/plans/2026-09-30-pdf-v8-runtime-plan.md
  - docs/SSOT/components/pdf.md
related_decisions:
  - docs/SSOT/migration/2026-09-29-rust-python-port-design.md
---

# User-approved embedded V8/PDF.js direction

## Context

The user explicitly required V8 for PDF and clarified that PDF.js must be embedded in V8 for parsing and text extraction. This supersedes the earlier pure-Rust semantic-backend plan. The merged private object-reader checkpoint remains historical research; it does not establish PDF capability.

## Work performed

The coordinator opened `feature/pdf-v8-runtime` and assigned a SOL manager with two Luna workers: official assets/provenance and private runtime/tests. Root owns dependencies, lockfile, SSOT/WIKI and publication. The optional `pdfjs-v8` feature pins `v8 = 152.2.0`; upstream PDF.js assets use version 4.10.38, matching the oracle package without copying the ignored checkout. Current design and P2 navigation now reflect this decision.

## Evidence

`cargo check -p kordoc-pdf --features pdfjs-v8` succeeded on macOS arm64 with the pinned V8 prebuilt archive. `cargo deny check` passed advisories, duplicate-version bans, licenses and sources without policy changes. These establish dependency feasibility only; they do not prove PDF extraction or native-wheel support.

## Outcome

Runtime implementation and meaningful extraction/termination/limit tests are in progress. The module remains private and feature-gated; no PDF Python, MCP or production-registry capability is registered.

## Follow-ups

Verify real embedded extraction, host API absence, output/deadline boundaries, upstream licenses and reproducible checksums. Production registration still requires source-neutral IR integration, full-result parity, supervised native-process containment, resource factories and six-target installed-wheel evidence.
