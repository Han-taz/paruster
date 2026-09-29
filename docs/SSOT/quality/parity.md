# Parity and normalization policy

Parity compares the Rust/Python port with the approved behavioral oracle across normalized recursive IR, Markdown, metadata, outlines, pages, images, warnings, errors, transformations, generated documents, and protocol behavior. The oracle comparison is evidence about behavior; the committed contract snapshots and synthetic fixtures remain runnable without the local oracle directory.

## Permitted normalization

Only these non-semantic differences may be normalized for comparison:

- ZIP entry timestamps.
- XML attribute ordering.

Normalization must leave document content, IR fields, table topology, ordering, paths, image bytes, warning/error identifiers, generated semantics, and protocol envelopes intact. A new normalization requires coordinator review and an explicit documented rationale before use.

The foundation harness uses exact RFC 6901 pointer allowlists. A registered ZIP timestamp is removable only when the pointer's decoded terminal key is `timestamp`; registering an arbitrary semantic field does not authorize deletion. Additional timestamp spellings require a reviewed policy change. Registered XML values may reorder start-tag attributes only: declarations, comments, processing instructions, CDATA, entity spellings, element/text whitespace, text, tails, and child order remain lexically unchanged. Malformed registered XML is an error rather than a normalization opportunity.

## Evaluation integrity

Do not change oracle answers, expected results, benchmark policies, scoring formulas, fixture populations, or corpus membership to improve a port's score. Do not weaken fixtures or acceptance gates. Evaluator or ground-truth changes must be isolated, labeled, and reviewed separately from implementation improvements.

Report the compared dimensions, source and fixture provenance, normalizations applied, and exact commands/results. A missing optional or restricted corpus is reported as unavailable, not silently removed from the denominator. No normalization policy changes runtime status: planned features remain planned until their implementation and parity gates pass.

The document-result harness is separate from the detector manifest. Every case records input and expected hashes, license/provenance, generator, oracle source digest and capture command, options, dimensions, evidence kind, parity-numerator status, and exact normalization pointer allowlists. Source-contract smoke cases run but remain outside oracle parity. At the parser-seam stage successful oracle cases are unavailable and the numerator is zero; future per-case oracle captures do not require weakening this policy.

The six compatibility detection helpers have a separate exact-output matrix captured by executing the pinned TypeScript oracle. Its synthetic magic, malformed-container, deterministic ZIP, and generated CFB recipes are rebuilt in CI; committed hashes bind both inputs and captured answers, and no normalization is applied. This helper evidence does not enter the document-result numerator or imply parser parity.
