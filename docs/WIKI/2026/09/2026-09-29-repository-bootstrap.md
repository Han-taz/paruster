---
id: 2026-09-29-repository-bootstrap
date: 2026-09-29
status: recorded
component: repository
issue: null
pr: null
commit: null
related_ssot:
  - docs/SSOT/migration/2026-09-29-rust-python-port-design.md
related_decisions: []
---

# Repository bootstrap

## Context

The target `Han-taz/paruster` repository was empty. The existing TypeScript `kordoc` checkout must remain a local-only migration oracle.

## Work performed

- Created the otherwise empty `main` base commit required for pull requests.
- Activated a repository ruleset requiring pull requests and linear history while preventing deletion and non-fast-forward updates to `main`.
- Started the real repository contents on `bootstrap/repository`.
- Added `/kordoc/` to the root ignore rules.

## Evidence

- Initial base commit: `6555066`
- GitHub ruleset: `protect-main` (`24182744`)

## Outcome

All substantive repository contents can now enter through GitHub Flow.

## Follow-ups

- Add required status checks after the initial CI workflows exist.
- Record the bootstrap PR and merge commit in a follow-up entry rather than rewriting this entry.

