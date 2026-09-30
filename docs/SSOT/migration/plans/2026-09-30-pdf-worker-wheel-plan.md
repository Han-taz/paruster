# Private PDF worker wheel feasibility

Status: CI-only implementation, independent review, notice provenance/audit
and local macOS installed execution pass. All six hosted targets precede merge. No public
PDF registration or ordinary source-build support is introduced.

The user reconfirmed PDF.js inside V8 with Rust execution management and all
other implementation in Rust. This slice packages the private worker only in
its separate feasibility workflow. Fresh default wheels remain unchanged.

## Ownership

Luna B owns the new six-target workflow and wheel stage/audit/smoke tooling,
including focused tooling regressions. SOL owns independent review and a
notice-only upstream-provenance bundle. The coordinator owns SSOT/WIKI,
publication, final checks and protected merge. Generated native executables
must never enter Git. The ignored migration oracle remains absent from all
build/runtime/test/artifact paths.

## Required evidence

Build the native binary for each existing x86_64/aarch64 Linux, Windows MSVC
and macOS target with pinned Rust and the optional embedded V8 feature.
Linux compilation occurs in Maturin's manylinux 2.28 container before wheel
construction. Preserve executable mode, include the fixed adjacent worker
path and its upstream license notices, and audit actual ELF/PE/Mach-O
architecture, wheel tags, exact worker member and notice hashes. Do not infer
architecture from filename alone.

Install each wheel in a clean native environment and execute the installed
worker using strict private binary framing. Both unchanged authored Helvetica
and ToUnicode Hangul/astral fixtures must extract their exact expected text.
Repeat Linux installed-worker probes under a manylinux 2.28 runtime to catch
native dynamic-library incompatibility. The independent workflow gate must
fail if any target or supporting audit fails.

First prove the previous default wheel fails the missing-worker audit; then
prove the newly staged local macOS arm64 wheel succeeds at archive audit and
actual isolated CPython 3.10 execution. Keep all existing default CI/security/
coverage/fuzz/wheel gates. The six-target matrix itself is evidence only after
its hosted runs pass, not merely after the YAML is written.

## Limits and later work

Official Windows ARM64 runners currently offer ARM64 Python 3.12–3.14; that
target's feasibility smoke uses 3.12. The cp310-abi3 extension tag is audited,
but native CPython 3.10 execution on that target remains unproven. Other five
targets smoke on 3.10. See the official
[runner toolset](https://github.com/actions/runner-images/blob/main/images/windows/toolsets/toolset-win-11-arm64.json).

Staging a binary in CI does not make ordinary PEP 517 or sdist installation
build that worker. Fixed trusted sibling discovery, OS memory/process-tree
containment and measured RSS, PDF resource factories/operator/layout/IR,
options/encryption/metadata and full corpus parity remain separate gates.
No protected parser success numerator or production capability is promoted.

The feasibility checkpoint merged in PR [#27](https://github.com/Han-taz/paruster/pull/27)
as `8d565cc6e4a05ffd16c3686a87f4cc4449b3008f` after all six installed-worker
targets and required protected gates passed. Production assembly/containment
and PDF semantic qualification remain separate.
