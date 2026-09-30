---
id: 2026-09-30-hwpx-hosted-candidate
date: 2026-09-30
status: recorded
component: hwpx-python
issue: null
pr: https://github.com/Han-taz/paruster/pull/22
commit: adaa106bee630a3401011e6382c6ebf20dcd5250
related_ssot: [docs/SSOT/components/hwpx.md, docs/SSOT/contracts/python-api.md, docs/SSOT/migration/status.md]
related_decisions: []
---

# HWPX hosted candidate evidence

The private bounded HWPX join merged through PR [#21](https://github.com/Han-taz/paruster/pull/21) as `c5cded899b6dd80c99aaad8de6f1145060c77717` after independent SOL review and every required check passed. PR #22 was updated onto that main merge, preserving append-only history. Fresh locked workspace tests and 296 Python/contract/parity tests passed after the merge.

Head `adaa106` passed hosted [CI](https://github.com/Han-taz/paruster/actions/runs/36701020628), [Security/CodeQL](https://github.com/Han-taz/paruster/actions/runs/36701020709), [six native wheel targets](https://github.com/Han-taz/paruster/actions/runs/36701020659), and [six fuzz targets](https://github.com/Han-taz/paruster/actions/runs/36701020615). Retargeting the stacked PR canceled the preceding workflow runs; replacement runs on the same head succeeded. No gate was bypassed.

Independent H4 review approved candidate publication after the meaningful failing-then-passing image-option regressions. The executable candidate exposes bounded HWPX parsing and validation through Python. It does not claim full option or corpus parity: `plain`, `htmlTables`, `scriptTags`, `keepTrailingEmptyCols`, `includeFieldPlaceholders`, malformed-section diagnostics, and the recorded ODF checksum interoperability difference remain follow-ups. The protected manifest's full-parser success numerator stays unchanged.

This entry supplements the [local publication record](2026-09-30-hwpx-h4-publication.md). Later heads must pass all protected checks before squash merge; neither this record nor a green prior head substitutes for that requirement.
