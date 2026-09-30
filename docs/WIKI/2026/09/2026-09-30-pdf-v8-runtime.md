---
id: 2026-09-30-pdf-v8-runtime
date: 2026-09-30
status: private-candidate
component: pdf
issue: null
pr: https://github.com/Han-taz/paruster/pull/23
commit: null
related_ssot: [docs/SSOT/components/pdf.md, docs/SSOT/migration/plans/2026-09-30-pdf-v8-runtime-plan.md, docs/SSOT/migration/status.md]
related_decisions: [docs/SSOT/migration/2026-09-29-rust-python-port-design.md]
---

# Embedded PDF.js runtime feasibility

The SOL manager supervised two Luna workers with disjoint official-assets and runtime boundaries. Root owns dependencies, documentation, verification and PR publication. The user-approved V8 direction supplements the [decision entry](2026-09-30-pdf-v8-decision.md); no pure-Rust semantic backend is continued.

A genuine RED run first failed because the runtime module was absent. Host compatibility failures subsequently exposed missing URLSearchParams, AbortController and a nonfunctional ReadableStream queue; these were fixed through actual PDF.js execution. The bridge now compiles the pinned main/worker ES modules in one fresh isolate, uses the embedded worker handler and a charged binary input buffer, disables PDF.js eval/worker-fetch/font-face/browser image paths, extracts page/text data and destroys the document task.

Fresh `cargo test -p kordoc-pdf --features pdfjs-v8 --locked` passes the existing substrate cases and 8 scoped runtime executions. The exact authored fixture yields one page and `V8 PDF.js probe`. Runtime reports V8 `15.2.124.1-rusty` from crate `152.2.0` and checks PDF.js `4.10.38`. Tests cover actual prefixed-corrupt parsing, initialized-module host-I/O absence, exact JSON-output N/N-minus-one, the same watchdog path with a nonterminating script, allocator inclusive boundary, actual JS over-cap ArrayBuffer rejection and isolate-drop release. The over-cap test ran in its own test process first: catchable `RangeError`, not a fatal abort.

The private limits are 32 MiB input, 200 pages, 4 MiB serialized UTF-8 output, 192 MiB old-generation heap, 128 MiB V8 ArrayBuffer stores and 10 seconds. The custom allocator uses atomic checked charging before allocation and matched Rust allocation/free with a retained Arc lifetime. It includes the copied input buffer. Other native memory and fatal OOM still require process containment.

Independent SOL review found no Critical issue within this private spike. Root reviewed the allocator callbacks, charged input, watchdog lifetime, output boundaries and tests. Strict all-target all-feature Clippy, fmt/diff, ordinary locked workspace tests, offline asset checks, docs links, cargo-deny and the current cargo-audit gate pass. `paste 1.0.15` produces the upstream unmaintained advisory RUSTSEC-2024-0436 as a build-time proc-macro; no warning ignore or policy change is introduced. The noncanonical macOS all-features workspace test combines PyO3 extension-module linkage with embedded Python lib tests and fails at existing unresolved Py symbols; ordinary workspace tests plus separately feature-enabled PDF tests and installed-wheel validation are the required paths.

All 188 selected upstream asset hashes/byte sizes, licenses, exact tree and authored fixture reproducibility pass an offline default-feature test. Upstream selected assets total 5,106,488 bytes. The oracle, Node and network are unnecessary for ordinary PDF tests/runtime. This private feature has no core, Python or MCP registration and changes no shared IR/error schema or parity numerator.

Production follow-ups remain: supervised native-worker fatal-failure/RSS containment, six-target feature-enabled installed-wheel validation, embedded CMap/standard-font factories, complete host clone/stream/URL semantics, complex text/operator/font/metadata and full-result/option parity. Hosted gates on the final published runtime head must pass before protected squash merge.
