# HWPX component

Status: H1a merged; H2a XML/sections and H1b crypto/metadata/validation are locally reviewed private candidates. Protected hosted gates, H3/H4 public integration and parity remain pending.

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
This guard applies to supplied-cache range expansion; XML-derived evidence is
bounded by the package and XML budgets. Empty supplied cache sections use
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

Tables and images remain H2b work. The candidate's paragraph layout inference
does not yet account for table-internal page splits or suppress the following
mid-page reset after a split table. Those signals must be integrated with H2b
before the final parser parity gate.

The current branch contains 53 crate unit tests across package, XML, sections, and styles, plus 8 HWPX integration tests. H2a-specific coverage includes namespace-local names, DTD/entity rejection, critical XML depth failure, section depth recovery, preserving neighbors around a malformed middle section, spine and numeric ordering, run spans and heading outline, notes, complete versus mixed layout-cache behavior, empty-paragraph page transitions, column transitions, source-page evidence and supplied-cache range bounds. The integration tests presently cover deterministic fixture construction and shape, not production parser registration or public API behavior.

The deterministic fixtures and oracle observations are research evidence only. Over-depth section handling intentionally differs from the current oracle, which returns clipped/empty content; the bounded implementation reports `PARTIAL_PARSE`. Such security divergences are recorded and excluded from parity scoring according to the parity policy. There is no successful production parser oracle capture, Python HWPX test, or full HWPX parity result at this checkpoint.

H2a's final scoped SOL code review is CLEAN and its local checks pass; protected hosted gates remain pending because the current session cannot publish GitHub writes. Later H1b/H2b work, H3 crate integration, H4 coordinator dispatch/Python/golden wiring, and H5 full fuzz and hosted gates remain separate gates. Capability and parity remain pending in the [migration ledger](../migration/status.md).
