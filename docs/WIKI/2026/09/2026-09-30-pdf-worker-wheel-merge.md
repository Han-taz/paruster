---
id: 2026-09-30-pdf-worker-wheel-merge
date: 2026-09-30
status: recorded
component: pdf
issue: null
pr: https://github.com/Han-taz/paruster/pull/27
commit: 8d565cc6e4a05ffd16c3686a87f4cc4449b3008f
related_ssot: [docs/SSOT/components/pdf.md, docs/SSOT/migration/plans/2026-09-30-pdf-worker-wheel-plan.md, docs/SSOT/migration/status.md]
related_decisions: []
---

# Six-target native worker-wheel checkpoint merged

PR [#27](https://github.com/Han-taz/paruster/pull/27) merged by protected squash
as `8d565cc6e4a05ffd16c3686a87f4cc4449b3008f` on September 30 at 12:20:01 UTC.
Final implementation head `e981f6c91df00e6ea899b107411f8a0de85211d0` passed
[CI](https://github.com/Han-taz/paruster/actions/runs/36713430374),
[Security/CodeQL](https://github.com/Han-taz/paruster/actions/runs/36713430281),
[base wheels](https://github.com/Han-taz/paruster/actions/runs/36713430285),
[bounded fuzzing](https://github.com/Han-taz/paruster/actions/runs/36713430322)
and the [six installed PDF worker-wheel targets](https://github.com/Han-taz/paruster/actions/runs/36713430255).
Linux/macOS/Windows x64 and ARM64 all build the worker, audit wheel architecture
and exact notices, install the wheel cleanly and extract both ASCII and
Hangul/astral PDF inputs. Linux additionally executes at the manylinux 2.28
baseline; Windows ARM64 uses native CPython 3.12.

Independent reviews and 339 isolated macOS installed-wheel API/contract/parity/
tooling tests passed locally. The preceding checkout, isolated-mode UTF-8 and
native newline follow-ups remain intact as append-only records.
No binary enters Git. Default-wheel behavior, public PDF registration,
ordinary PEP 517/sdist worker assembly, OS memory/process-tree containment,
resource factories and full PDF IR/layout/option parity remain pending.

The independent HWPML H0 run's six fuzz jobs all succeed, but its aggregate
runner fails to start after five acquisition attempts. GitHub check annotation
reports that infrastructure failure; it has no steps or test log. After this
merge, HWPML incorporates current main and reruns all mandatory gates on a
fresh head rather than skipping or relaxing the failed gate.
