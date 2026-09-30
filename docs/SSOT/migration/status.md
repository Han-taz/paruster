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

The merged parser-seam baseline has only injected/synthetic success evidence and
an empty production registry. The local HWPX candidate below now registers one
real adapter, but does not change the protected document harness's zero
successful-capture count. The earlier 88.63% line coverage and
two 30-second fuzz campaigns without crashes describe the initial merged foundation baseline.
PR #9 added fresh hosted coverage and fuzz evidence for the parser seam. None
of these foundation checks demonstrates document-output parity.

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

The merged parser-wave scaffold registers `kordoc-hancom` and `kordoc-pdf` as
root workspace members with dependency-direction tests and no public parser
entry points. The local HWPX integration candidate now extends that baseline
with a core adapter and Python bindings. `parse_hwpx`, `validate_hwpx`, and
`parse_pdf` remain planned in the protected capability inventory, and the
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
unregistered and exposes no parser capability. The locally reviewed H2a/H1b/H2b/H3
join below remains a separate candidate. See the append-only
[H1a merge record](../../WIKI/2026/09/2026-09-30-hwpx-h1a-merge.md).

H2a XML and section lowering is present as a crate-local candidate on
`feature/parse-hwpx-xml` and passed its final scoped SOL review. The candidate has
bounded XML parsing, transactional section lowering, styles/notes/page
evidence, and focused unit coverage. Review regressions now cover mixed layout
fallback, note suffix inheritance, and omitted-empty-paragraph page transitions.
H2a hosted gates remain pending. GitHub publication is now available through the
authenticated CLI. The XML slice will ship with the aggregate lowering budget
from H3, rather than as an unbounded standalone section lowerer. See the
[HWPX component page](../components/hwpx.md) and append-only
[H2a candidate record](../../WIKI/2026/09/2026-09-30-hwpx-h2a.md).

The reviewed H1b candidate adds crypto/metadata/validation and passes 72 unit
plus 8 integration tests; [evidence and process/standards caveats](../../WIKI/2026/09/2026-09-30-hwpx-h1b-local.md)
are recorded. H2b tables/images and H3 private composition are present in the
recovered `feature/hwpx-private` candidate. Fresh independent SOL review found
no Critical or Important private Hancom issue. The reviewed aggregate allocation
guards cover nested note text and repeated inherited note prefixes before copying.
Fresh Hancom verification passes 103 library tests and 122 integration-binary
executions: 103 repeated unit cases and 19 distinct integration cases. The private join merged in PR [#21](https://github.com/Han-taz/paruster/pull/21) as `c5cded899b6dd80c99aaad8de6f1145060c77717` after all required hosted gates and independent review passed. H4 candidate wiring is published separately below.

H4 currently adds a real HWPX core dispatch adapter, strict format-specific
preflight, Python `parse_hwpx`/`validate_hwpx`, immutable validator models, and
direct capped native result serialization. A freshly built macOS arm64 abi3 wheel
installed into an isolated CPython 3.10 environment passes 296 Python/contract
tests after the lowering-budget and image-option follow-ups. Independent review
and fresh locked workspace verification pass locally. The seven ordinary/encrypted complete-result captures are
compared separately without normalization; their initial failures exposed
unstyled-span emission and nested-cell newline differences. The malformed
section EOF warning diagnostic is restored by the focused candidate below;
no frozen oracle result, scoring or security boundary is changed to hide it. Generated
fixture input hashes are checked against all twelve unchanged H0 pins.

PR [#22](https://github.com/Han-taz/paruster/pull/22) merged this executable candidate as `e10a2df2dd57d1b69f55afeb0a07295e0c29db7f` after final-head protected gates passed. Head `adaa106` passed hosted [CI](https://github.com/Han-taz/paruster/actions/runs/36701020628), [Security/CodeQL](https://github.com/Han-taz/paruster/actions/runs/36701020709), [six native wheels](https://github.com/Han-taz/paruster/actions/runs/36701020659), and [six bounded fuzz targets](https://github.com/Han-taz/paruster/actions/runs/36701020615). Independent SOL review passed for candidate publication. Final head `b5ceec2` passed the required checks before protected squash merge. H4 candidate evidence
must not be interpreted as support for the other parsers or any MCP handler.
Fresh H4 review found no remaining dispatch, serialization, or Python-model
blocker for candidate publication after the `images=false` regression fix.
That option omits image collections and recursive image payloads while retaining
Markdown, page projections, and image placeholders. HWPX intentionally ignores
the PDF-only `tables` option. The focused [option follow-up](plans/2026-09-30-hwpx-option-parity-plan.md)
merged in PR [#24](https://github.com/Han-taz/paruster/pull/24) as
`0cd5b261371aaf697c11db825bb22500db369782` after final-head CI, security,
six native wheels, six bounded fuzz targets and independent review passed. It implements `plain`, `htmlTables`, `scriptTags`, `keepTrailingEmptyCols`, and
`includeFieldPlaceholders` through existing Rust/Python options. Five pinned
helper observations and authored native/Python cases cover default/false/true,
Unicode, nested cells/fields and ordering. Independent review identified and
fixed transient character/column allocations and repeated suffix rescans.
The focused [unfinished-XML warning restoration](plans/2026-09-30-hwpx-warning-parity-plan.md)
merged in PR [#26](https://github.com/Han-taz/paruster/pull/26) as
`cb17578e6ee2112762b4abdb2d64f0114f3eea6b` after all protected gates passed.
It adds the unchanged eighth full-result capture, including complete warning text,
code/page and retained neighboring sections. The original extension first fails
on all three public parse paths at `/warnings/0/message`. Qualified source-name
ranges are collected only after the primary parser reports unfinished-tag EOF;
non-EOF syntax faults retain their prior diagnostic and new message appends
remain allocation-metered. Full HWPX capability remains pending for other
malformed diagnostics, checksum interoperability and representative/restricted
corpus parity; the protected success numerator stays 0.
The preceding H2a candidate had 53 unit tests and 8 integration tests, including
deterministic synthetic fixture checks. These do not establish public parser
registration, Python behavior, or real-document parity. HWPX capability stays
pending; the parser-oracle success numerator remains zero.

PDF P2a Task 0 has a reviewed private bounded-reader candidate. The pinned
`lopdf 0.45.0` eager loader failed the pre-allocation and cumulative-budget
gate, so runtime object access uses the in-crate borrowed-source substrate
described in the [PDF component page](../components/pdf.md). This checkpoint
does not parse pages or glyphs, expose a public function, or advance PDF parity;
those remain pending until the rest of P2a and its protected merge complete.

The active private V8/PDF.js spike merged in PR [#23](https://github.com/Han-taz/paruster/pull/23). Fresh local feature tests execute V8 `15.2.124.1-rusty` with PDF.js `4.10.38` and extract the authored one-page text exactly. Eight scoped executions, offline asset integrity, strict all-feature Clippy, normal locked workspace tests and docs checks pass. This demonstrates embedded runtime feasibility only. Source-neutral IR/layout/table integration, OS-calibrated process resource containment, full result/option parity and feature-enabled six-target installed-wheel gates remain pending; no PDF production capability or manifest success numerator is promoted.

The private [native PDF.js worker](plans/2026-09-30-pdf-v8-worker-plan.md)
adds bounded one-document framing, two-slot supervision and kill/wait/I/O
cleanup around the existing V8 probe. Actual supervised Helvetica and
ToUnicode Hangul/astral probes compare exactly to in-process extraction. This
checkpoint merged in PR [#25](https://github.com/Han-taz/paruster/pull/25) as
`43c30fd2239fb762eba4530e61bb46802176a663` after all protected gates passed.
It does not register PDF or package a worker in base wheels; OS-specific resource containment, six-target worker wheels, factories and
full-result IR/layout/option parity remain pending.

The separate [PDF worker-wheel gate](plans/2026-09-30-pdf-worker-wheel-plan.md)
now has a locally reviewed CI-only candidate for six targets. Actual macOS
arm64 installed execution and 339 Python/tooling cases pass with pinned
upstream notices included. The six hosted worker targets are still pending;
default wheels, ordinary PEP 517/sdist worker builds and public PDF registration
are unchanged. Windows ARM64 uses CPython 3.12 for this feasibility gate and
minimum-version native execution remains unproven there.

The private [PDF resource-factory checkpoint](plans/2026-09-30-pdf-v8-resources-plan.md)
is locally reviewed. An authored CID/Helvetica PDF first fails to retain Korean
text, then passes with real CMap/font callback counts; 182 embedded mappings
match the unchanged provenance. Bounded callback arguments/request/item/total
bytes and strict fallback-denial behavior are verified. The next worker-wheel
inventory has 36 notices and a third mandatory installed resource probe.
Hosted publication, production containment/assembly and full PDF IR parity
remain pending; this does not promote public capability or protected scoring.

All 17 MCP tools remain pending and must preserve their frozen names, schemas, descriptions, envelopes, limits, security policy, and stdio discipline when implemented.

## Evidence policy

Detector-only and generated-IR projection goldens prove deterministic foundation behavior, not document-output parity. The parity policy in [`../quality/parity.md`](../quality/parity.md) governs allowed normalization; first-difference evidence and fixture provenance are required before any capability moves from pending to verified.
