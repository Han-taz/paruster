# Migration status

This page is the live capability ledger. A capability is complete only after its implementation, contract/parity evidence, and required quality gates have merged. Contract inventory alone never counts as implementation. Local verification is recorded separately from hosted CI results and repository branch-protection enforcement.

## Merged foundation

- Rust workspace and Python 3.10+ abi3 packaging scaffold
- recursive foundation IR and the 13 stable error codes
- bounded content detection for HWP3, ZIP-based HWPX/Office formats, CFB-based HWP/XLS, PDF, HWPML, PNG, JPEG, and WebP
- checked single-disk ZIP/ZIP64 preflight with input, entry-count, uncompressed-size, and local-extent bounds
- foundation dispatch that reports empty, oversized, security-rejected, and unsupported inputs without claiming parser completion
- internal acyclic `ParsedDocument`/options seam, private injectable registry, parser panic containment, and successful-result assembly tests
- immutable Python `Document` projection, bounded serializable option validation, and six legacy detection/refinement compatibility helpers with pinned runtime-oracle evidence
- provenance-checked full-result harness with source-smoke versus oracle-capture accounting

The parser seam has only injected/synthetic success evidence; the production registry remains empty and the document harness reports zero successful oracle captures. The earlier 88.63% line coverage and two 30-second fuzz campaigns without crashes describe the initial merged foundation baseline. PR #9 added fresh hosted coverage and fuzz evidence for the parser seam. None of these foundation checks demonstrate document-output parity.

## Merged P7 shared projections (PR #11)

The merged `kordoc-core` implementation provides source-neutral Markdown,
page and structural chunk projections, bounded table construction and opt-in
classification, label/visual policy, and a single shared Markdown table-unit
reader. The merged Rust/Python public mappings are
`blocks_to_markdown`, `blocks_to_pages`, `blocks_to_chunks`,
`kordoc.tables.classify_table_tree`, and the `ChunkOptions`, `DocChunk`, and
`PageMarkdown` models. Other table API exports remain planned. The default
page renderer detaches from the GIL; the optional synchronous Python callback
receives deeply immutable blocks and propagates its original exception.

Six generated-IR projection cases have captured answers from the pinned,
read-only runtime oracle; all six pass exact comparison without normalization.
This proves pure projection behavior only. The document-parser oracle-success
numerator remains **0**, and the production parser registry remains empty; no
real parser or MCP tool is claimed complete. P7 also has two bounded
proptests, dedicated Markdown-unit and projection fuzz targets, and local
Rust/Python quality-gate evidence recorded in the implementation entry.

PR [#11](https://github.com/Han-taz/paruster/pull/11) merged by squash as
`d473bae79f1de6f1aced29c59609a2cb49f0aa90` after all 33 checks succeeded.
The hosted [CI](https://github.com/Han-taz/paruster/actions/runs/36622144159),
[Fuzz](https://github.com/Han-taz/paruster/actions/runs/36622144214),
[Wheels](https://github.com/Han-taz/paruster/actions/runs/36622144316), and
[Security/CodeQL](https://github.com/Han-taz/paruster/actions/runs/36622144308)
runs passed. P7 is now a merged shared capability; P1-P6 are unblocked to
implement format parsers against its stable projections and add real-document
fixtures. Their parser parity remains pending until those fixtures pass.
Detailed merge evidence is in the append-only
[P7 merge record](../../WIKI/2026/09/2026-09-30-ir-projections-merge.md).

PR #5 implementation head `a9df485` passed the hosted foundation gates: [CI](https://github.com/Han-taz/paruster/actions/runs/36600464167), [Security](https://github.com/Han-taz/paruster/actions/runs/36600464228), [Native wheels](https://github.com/Han-taz/paruster/actions/runs/36600464274), and [Bounded fuzzing](https://github.com/Han-taz/paruster/actions/runs/36600464235). PR #5 documentation head `ad86e43` also passed all four hosted gates: [CI](https://github.com/Han-taz/paruster/actions/runs/36601581589), [Security](https://github.com/Han-taz/paruster/actions/runs/36601581506), [Native wheels](https://github.com/Han-taz/paruster/actions/runs/36601581485), and [Bounded fuzzing](https://github.com/Han-taz/paruster/actions/runs/36601581555). PR #4 remediation head `31212275` passed all four hosted checks: [CI](https://github.com/Han-taz/paruster/actions/runs/36603823278), [Security](https://github.com/Han-taz/paruster/actions/runs/36603823358), [Native wheels](https://github.com/Han-taz/paruster/actions/runs/36603823436), and [Bounded fuzzing](https://github.com/Han-taz/paruster/actions/runs/36603823386). The Security run includes a successful main-target Dependency review, confirming the pytest 9.0.3 remediation. Active protect-main ruleset `24182744` strictly requires `ci-gate`, `security-gate`, `wheels-gate`, `fuzz-gate`, and CodeQL, limits PR merges to squash-only, and states that later heads must pass the required checks. Product parity remains pending as detailed below.

PR #9 merged the parser seam as `770ab8c` after [CI](https://github.com/Han-taz/paruster/actions/runs/36612367072), [Security and CodeQL](https://github.com/Han-taz/paruster/actions/runs/36612367082), [Native wheels](https://github.com/Han-taz/paruster/actions/runs/36612367121), and [Bounded fuzzing](https://github.com/Han-taz/paruster/actions/runs/36612367219) passed. The wheel matrix covered six native targets and the Python matrix covered CPython 3.10 through 3.14.

## Pending product parity

Full real-document oracle parity is pending. All document parsers, OCR pipelines, transformations, comparisons, redaction, form operations, patching/generation flows, renderers, and MCP handlers remain pending. P7's six synthetic pure-projection captures do not advance parser parity. None is complete merely because its contract has been frozen; each stays pending until representative and adversarial oracle/golden fixtures pass in Rust/Python.

The first parser wave is specified by focused [HWPX](plans/2026-09-30-hwpx-implementation-plan.md) and [PDF](plans/2026-09-30-pdf-implementation-plan.md) execution plans. These plans authorize parallel implementation only after their planning PR and the protected parser-wave scaffold PR merge; they do not advance either parser's capability or parity status. HWPX keeps the generic strict ZIP security gate and allows only bounded part-local recovery after a structurally valid archive is opened. The September 30 user decision supersedes pure-Rust PDF semantic extraction: the active [V8/PDF.js plan](plans/2026-09-30-pdf-v8-runtime-plan.md) embeds the pinned upstream modules without Node.js. The private runtime probe precedes IR integration, supervised native-process containment, and six-target wheel gates. PDF capability and parity remain pending. PDFium remains behind the optional raster/OCR gate.

The parser-wave scaffold registers `kordoc-hancom` and `kordoc-pdf` as root
workspace members with dependency-direction tests and no public parser entry
points. Neither crate is connected to the production registry or Python API;
`parse_hwpx`, `validate_hwpx`, and `parse_pdf` remain planned, and the
real-document oracle-success numerator remains zero.

PR [#14](https://github.com/Han-taz/paruster/pull/14) merged the protected
parser-wave scaffold as
`86e4b78ce0fd2f40a238910eca46733fa84d317a`. All required CI, security,
six-target wheel, bounded-fuzz, and CodeQL checks passed. HWPX H0 and PDF P2a
may now execute from that merge; this merge does not advance either parser's
capability or parity status. Detailed hosted evidence is in the append-only
[scaffold merge record](../../WIKI/2026/09/2026-09-30-parser-wave-scaffold-merge.md).

HWPX H0 merged in PR [#16](https://github.com/Han-taz/paruster/pull/16) as
`08654585d58339c9116aa82fd6c87515364a02fe`. The merged evidence freezes 12
deterministic CC0 recipes, their hashes and complete oracle observations, and
the private package/error/order/budget contract. It exposes no parser entry
point and does not advance HWPX capability or parity. H1a package/security and
H2a XML/section implementation are now unblocked. See the append-only
[H0 merge record](../../WIKI/2026/09/2026-09-30-hwpx-h0-merge.md).

HWPX H1a merged in PR [#18](https://github.com/Han-taz/paruster/pull/18) as
`e33eab112999aebcf91e1593814bbec17bd1ea79`. The private package reader now
enforces the reviewed ZIP/ZIP64, path, record-count, CRC, member-extent,
plaintext/ciphertext, recovery, and section-order boundaries. It remains
unregistered and exposes no parser capability. H2a is still under review; H1b
starts only after their interface join. See the append-only
[H1a merge record](../../WIKI/2026/09/2026-09-30-hwpx-h1a-merge.md).

PDF P2a Task 0 has a reviewed private bounded-reader candidate. The pinned
`lopdf 0.45.0` eager loader failed the pre-allocation and cumulative-budget
gate, so runtime object access uses the in-crate borrowed-source substrate
described in the [PDF component page](../components/pdf.md). This checkpoint
does not parse pages or glyphs, expose a public function, or advance PDF parity;
those remain pending until the rest of P2a and its protected merge complete.

All 17 MCP tools remain pending and must preserve their frozen names, schemas, descriptions, envelopes, limits, security policy, and stdio discipline when implemented.

## Evidence policy

Detector-only and generated-IR projection goldens prove deterministic foundation behavior, not document-output parity. The parity policy in [`../quality/parity.md`](../quality/parity.md) governs allowed normalization; first-difference evidence and fixture provenance are required before any capability moves from pending to verified.
