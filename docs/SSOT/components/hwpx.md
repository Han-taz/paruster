# HWPX component

Status: H1a merged; H2a, H1b, and H2b are locally reviewed private candidates. H3 composes private crate entry points and locally bounded IR/metadata allocation on a feature branch. Coordinator-owned public dispatch/Python wiring, fuzzing review, and protected hosted gates remain separate pending gates.

## Package boundary

The crate-local package reader is the sole member access path. It validates the ZIP central directory before member access, counts directory and file records toward an inclusive 500-record ceiling, rejects unsafe or ambiguous member paths, and charges actual logical plaintext against a shared inclusive 268,435,456-byte budget. Required package faults remain hard errors. No section parser may bypass package metering. The broader ZIP preflight and its generic limits run before format parsing.

These package/security rules merged in H1a, PR [#18](https://github.com/Han-taz/paruster/pull/18), as `e33eab1`. They are crate-private and do not register an HWPX parser in production dispatch.

## XML and sections candidate

The H2a candidate in `kordoc-hancom::hwpx` uses a bounded event-driven XML reader to build a compact private tree. It matches element names by namespace-local name, requires UTF-8, rejects DTDs and custom entity declarations, and limits XML input to 64 MiB, text to 16 MiB, nesting to 200 levels, node and attribute counts to 100,000 each, and estimated tree storage to 32 MiB. Predefined and numeric character references are decoded without external resolution. XML allocation/resource exhaustion maps to `DECOMPRESSION_BOMB`; malformed critical XML maps to `CORRUPTED`.

Section lowering is transactional. Each section accumulates blocks, outline items, and page state in a temporary delta; that delta is committed only after the entire section parses and lowers. Malformed or over-depth section XML yields one `PARTIAL_PARSE` warning and leaves neighboring section deltas available. XML resource-budget violations remain hard `DECOMPRESSION_BOMB` errors. A malformed required manifest/header is a package-level `CORRUPTED` failure, not a recoverable section warning.

Section order follows the content spine when present and valid; otherwise numeric `Contents/sectionN.xml` order is used. Paragraph/run spans, heading outline, footnote and endnote text, character and paragraph style references, page evidence, and recursive page assignment are lowered into source-neutral IR. The crate does not create Markdown; shared core projections own Markdown and page rendering.

XML layout page evidence is used only when every section has top-level paragraphs and each such paragraph has line-layout records. If any section lacks usable layout evidence, page numbering falls back consistently to section order, recursively assigning the section page to nested blocks. Page selection applies after choosing layout or section fallback. Empty paragraphs omitted from the emitted blocks still retain their page transitions. Page evidence includes pages crossed within the final paragraph, and the next section starts after the preceding section's terminal page.

Inline footnote/endnote markers inherit the section's number type, user character, prefix, and suffix, with note-local decorations taking precedence. A present number format with no suffix emits an empty suffix; the default `)` applies only when the format itself is absent.

Paragraph layout inference compares vertical and horizontal coordinates to
distinguish a rightward column transition from a new page. Empty initial pages
and malformed sections retain source-page evidence. An explicitly supplied
cache preserves pages through its maximum page number and rejects range
expansion above 100,000 entries before allocation with `DECOMPRESSION_BOMB`.
The same 100,000-entry cap and a pre-insertion allocation charge apply to
XML-derived and page-selection evidence. Empty supplied cache sections use
section fallback instead of selecting a layout with no source pages.

Omitted options remain distinct from explicit `false`; the candidate keeps empty paragraphs only when `keep_empty_paragraphs == Some(true)`. No final Markdown operation or public Python progress callback is introduced by this crate-local work.

## Candidate evidence and limits

### H1b crypto, metadata and validation

The bounded manifest reader accepts exact reviewed AES-256-CBC, SHA256
password-start-key, PBKDF2 and checksum identifiers/aliases, not arbitrary
suffixes. Parameter sizes, encrypted path uniqueness/existence, inclusive
1,000,000 iterations per entry and 4,000,000 summed iterations are checked
before derivation. SHA1 PRF precedes SHA256 fallback. Raw deflate requires
StreamEnd; up to 16 arbitrary CBC alignment bytes may follow. An 8 KiB buffer
charges the shared package plaintext budget before append; failed attempts
roll back charges before retry. Plaintext is installed only as a complete
verified batch, and encrypted paths cannot be read before that installation.
Missing/wrong passwords produce safe distinct `ENCRYPTED` diagnostics.

Constant-time SHA256 comparison hashes the first 1,024 **decompressed** bytes,
matching the approved oracle/fixture contract. ODF specifies compressed bytes;
this standards interoperability gap remains recorded, not silently changed.
Zero-only or PKCS7 padding assumptions are not introduced.

Metadata does not read section content. OPF precedes optional Dublin Core;
title-or-author stops fallback, description wins over subject, first duplicate
values win and FILETIME-zero dates are omitted. Critical XML/resource faults
are hard; optional malformed metadata can be skipped. The validator preserves
ordered issues, files-only entry count and the same typed password failures.
H1b passes 72 unit plus 8 integration tests and strict local gates; independent
SOL scoped review is CLEAN. Its red-first TDD process deviation is recorded in
[WIKI](../../WIKI/2026/09/2026-09-30-hwpx-h1b-local.md).

### H2b tables and images

Table lowering retains ordered text, nested tables, image blocks, captions,
merged spans, header cells, and trailing empty cells. It enforces inclusive
200-column, 2,000,000 aggregate logical-cell, and 64-level logical-recursion
limits before grid allocation. Table-internal page splits advance the source
page once and suppress the following mid-page reset. A malformed section rolls
back its staged table/image state; package/resource limits remain hard errors.
Image references are path-checked and resolved only through the shared package
reader. Retained cache/list/block copies and image metadata share an inclusive
256 MiB private image-output meter; exceeding it is `OUTPUT_TOO_LARGE` rather
than a silent truncation. The H2b scoped SOL review and local gates are clean.

### H3 private composition

The private `parse_hwpx`, `parse_hwpx_metadata`, and `validate_hwpx` entry points
compose bounded package, crypto, critical XML, metadata, styles, section
lowering, and validation. The parser returns source-neutral `ParsedDocument`
without Markdown. Missing required container/content/header data is hard
`CORRUPTED`; an all-unusable section set is `NO_SECTIONS`. After central
validation, isolated damaged section members produce `BROKEN_ZIP_RECOVERY`
and retain their original section ordinals so neighboring page numbers do not
shift. Malformed section XML remains `PARTIAL_PARSE`.

Image placeholders preserve source order through page-mode selection and page
filtering, including nested table cells/captions. Only retained image refs are
then resolved; excluded-page image members are neither read nor returned,
warned, charged, or assigned filenames. A full parse additionally sweeps
unreferenced `BinData` images through the same cache/output meter, returning
their assets and appending image IR blocks. A page-selected parse does not run
that sweep. `metadata.pageCount` and `ParsedDocument.pageCount` use full source
page evidence, not the count remaining after selection. Metadata-only reads
package metadata and section paths, never section contents; plaintext metadata
remains available when section members are encrypted, without decrypting them.
Metadata-only first parses the bounded critical encryption manifest and marks
declared encrypted paths before any metadata member read. An encrypted OPF or
optional metadata member fails closed with typed `ENCRYPTED`, even with a
password; selective metadata-only decryption is not implemented.
Plain paragraph-only table cells and captions omit redundant optional nested
block arrays, while structural nested blocks remain ordered.

The local private integration cases cover the three entry points, deterministic
fixtures, both encrypted PRFs, section isolation, critical/package failures,
page-selected nested assets, unreferenced sweep, full source page count, and
plain-versus-structural table fields. These do not constitute public Python or
full-wire parity evidence.

The deterministic fixtures and twelve captured oracle results are research evidence, not by themselves a hosted full-wire result. Over-depth section handling intentionally differs from the current oracle, which returns clipped/empty content; the bounded implementation reports `PARTIAL_PARSE`. Such security divergences are recorded and excluded from parity scoring according to the parity policy. Coordinator-owned H4 public dispatch/Python checks remain separate from this crate-private evidence.

### H3 allocation follow-up

One private, inclusive 256 MiB lowering-allocation budget is shared across
sections. Text is copied from XML leaves through charged appends, avoiding
recursive `text_content` amplification. Paragraph spans, nested cell text and
captions, note markers, outline strings, image placeholders, transient
paragraph/layout collections, IR block/span/cell/outline structures, and page
evidence are charged before retained copies or collection growth. Zero-length
paragraphs are not free because their IR structures count. Discarded malformed
sections roll back only their own charge; `OUTPUT_TOO_LARGE` escapes as a hard
error. The existing ZIP plaintext, 256 MiB image-output, XML tree, 200-column,
2,000,000-cell, and 64-depth limits remain independent. Page selection occurs
before retained image resolution; no excluded image is charged to the image
output meter.

Metadata extraction uses a separate inclusive 256 MiB budget across OPF and
optional Dublin Core fallback. It streams leaf text before each append and
charges retained values and keyword copies. Unknown OPF fields do not trigger
text materialization; metadata precedence and malformed-optional behavior are
unchanged. Budget failures from optional metadata are hard `OUTPUT_TOO_LARGE`,
not skipped as malformed XML. Reduced-limit tests cover nested text,
N/N-minus-one boundaries, OPF-to-fallback accumulation, and section/table
amplification without allocating large test payloads.

H2a, H1b, and H2b scoped SOL reviews and local gates passed; the H3 allocation
follow-up passed a fresh independent private Hancom review with no Critical or
Important issue. Fresh focused checks pass 103 library unit tests and 122
integration-binary executions (103 repeated units and 19 distinct integration
cases). Publication through the authenticated GitHub CLI is available;
the private checkpoint merged in PR [#21](https://github.com/Han-taz/paruster/pull/21) as `c5cded899b6dd80c99aaad8de6f1145060c77717` after all protected hosted gates passed. The recovered private checkpoint includes the
lowering meter, so unbounded note-prefix repetition from the earlier standalone
H2a candidate is not published separately. H4 now provides the reviewed core/Python candidate in PR [#22](https://github.com/Han-taz/paruster/pull/22). It merged as `e10a2df2dd57d1b69f55afeb0a07295e0c29db7f` after the final head passed CI, security, six native wheels, six fuzz targets and independent candidate review. Representative-corpus parity remains pending. Capability and parity
stay pending in the [migration ledger](../migration/status.md).


## Existing option restoration

The [focused option plan](../migration/plans/2026-09-30-hwpx-option-parity-plan.md)
restores `keepTrailingEmptyCols` and `includeFieldPlaceholders` during bounded
section/table lowering. Default trimming removes only trailing all-empty
columns; explicit preservation requires a real anchor in that column. The
untrimmed logical grid and all retained span copies still count against the
existing budgets.

CLICK_HERE tracking uses a stack and does not emit parameter metadata as body
text. Dirty fields and differing user-filled values remain visible. Direction
parameters precede encoded Command fallback; Command lengths use UTF-16 code
units, and escaped dollar signs are compared using the source convention.
Guide text remains in flat IR text and becomes a `placeholder` span only when
it matches. Nested already-marked guides prevent an outer raw-text match.
Only tables with direct `pos treatAsChar="1"` use inline-table policy.

Multi-column pipe and HTML cell rendering filters marked spans without losing
IR text or nested content. The source's one-column Markdown branch intentionally
reads flat cell text and can still display a marked guide. Empty Direction
parameters do not fall back to a Command guide. A Command length that splits a
UTF-16 surrogate pair is malformed: the port retains the original body text
unmarked because Rust/public IR requires valid Unicode scalars. It does not
synthesize an invalid half-surrogate or silently discard the text.

Shared core owns the remaining `scriptTags`, `plain` and `htmlTables` transforms;
see [normalization](normalization.md). Original H0 fixture hashes, seven complete
ordinary/encrypted answers and protected parity accounting are unchanged.
Malformed-section diagnostics, compressed-versus-decompressed ODF checksum
interoperability and representative/restricted-corpus evidence remain explicit
full-parser gaps.
