# HWPX component

Status: H1a package/security merged; H2a XML and section lowering is an implementation candidate under code review. This page records the candidate's private behavior and review boundary. It does not mark the HWPX parser, public API, or parity capability complete.

## Package boundary

The crate-local package reader is the sole member access path. It validates the ZIP central directory before member access, counts directory and file records toward an inclusive 500-record ceiling, rejects unsafe or ambiguous member paths, and charges actual logical plaintext against a shared inclusive 268,435,456-byte budget. Required package faults remain hard errors. No section parser may bypass package metering. The broader ZIP preflight and its generic limits run before format parsing.

These package/security rules merged in H1a, PR [#18](https://github.com/Han-taz/paruster/pull/18), as `e33eab1`. They are crate-private and do not register an HWPX parser in production dispatch.

## XML and sections candidate

The H2a candidate in `kordoc-hancom::hwpx` uses a bounded event-driven XML reader to build a compact private tree. It matches element names by namespace-local name, requires UTF-8, rejects DTDs and custom entity declarations, and limits XML input to 64 MiB, text to 16 MiB, nesting to 200 levels, node and attribute counts to 100,000 each, and estimated tree storage to 32 MiB. Predefined and numeric character references are decoded without external resolution. XML allocation/resource exhaustion maps to `DECOMPRESSION_BOMB`; malformed critical XML maps to `CORRUPTED`.

Section lowering is transactional. Each section accumulates blocks, outline items, and page state in a temporary delta; that delta is committed only after the entire section parses and lowers. Malformed or over-depth section XML yields one `PARTIAL_PARSE` warning and leaves neighboring section deltas available. XML resource-budget violations remain hard `DECOMPRESSION_BOMB` errors. A malformed required manifest/header is a package-level `CORRUPTED` failure, not a recoverable section warning.

Section order follows the content spine when present and valid; otherwise numeric `Contents/sectionN.xml` order is used. Paragraph/run spans, heading outline, footnote and endnote text, character and paragraph style references, page evidence, and recursive page assignment are lowered into source-neutral IR. The crate does not create Markdown; shared core projections own Markdown and page rendering.

XML layout page evidence is used only when every section has top-level paragraphs and each such paragraph has line-layout records. If any section lacks usable layout evidence, page numbering falls back consistently to section order, recursively assigning the section page to nested blocks. Page selection applies after choosing layout or section fallback. Empty paragraphs omitted from the emitted blocks still retain their page transitions. Page evidence includes pages crossed within the final paragraph, and the next section starts after the preceding section's terminal page.

Inline footnote/endnote markers inherit the section's number type, user character, prefix, and suffix, with note-local decorations taking precedence. A present number format with no suffix emits an empty suffix; the default `)` applies only when the format itself is absent.

Omitted options remain distinct from explicit `false`; the candidate keeps empty paragraphs only when `keep_empty_paragraphs == Some(true)`. No final Markdown operation or public Python progress callback is introduced by this crate-local work.

## Candidate evidence and limits

The current branch contains 44 crate unit tests across package, XML, sections, and styles, plus 8 HWPX integration tests. H2a-specific coverage includes namespace-local names, DTD/entity rejection, critical XML depth failure, section depth recovery, preserving neighbors around a malformed middle section, spine and numeric ordering, run spans and heading outline, notes, complete versus mixed layout-cache behavior, empty-paragraph page transitions, and recursive fallback. The integration tests presently cover deterministic fixture construction and shape, not production parser registration or public API behavior.

The deterministic fixtures and oracle observations are research evidence only. Over-depth section handling intentionally differs from the current oracle, which returns clipped/empty content; the bounded implementation reports `PARTIAL_PARSE`. Such security divergences are recorded and excluded from parity scoring according to the parity policy. There is no successful production parser oracle capture, Python HWPX test, or full HWPX parity result at this checkpoint.

H2a awaits final code review and protected hosted gates. Later H1b/H2b work, H3 crate integration, H4 coordinator dispatch/Python/golden wiring, and H5 full fuzz and hosted gates remain separate gates. Capability and parity remain pending in the [migration ledger](../migration/status.md).
