# Parity and normalization policy

Parity compares the Rust/Python port with the approved behavioral oracle across normalized recursive IR, Markdown, metadata, outlines, pages, images, warnings, errors, transformations, generated documents, and protocol behavior. The oracle comparison is evidence about behavior; the committed contract snapshots and synthetic fixtures remain runnable without the local oracle directory.

## Permitted normalization

Only these non-semantic differences may be normalized for comparison:

- ZIP entry timestamps.
- XML attribute ordering.

Normalization must leave document content, IR fields, table topology, ordering, paths, image bytes, warning/error identifiers, generated semantics, and protocol envelopes intact. A new normalization requires coordinator review and an explicit documented rationale before use.

## Evaluation integrity

Do not change oracle answers, expected results, benchmark policies, scoring formulas, fixture populations, or corpus membership to improve a port's score. Do not weaken fixtures or acceptance gates. Evaluator or ground-truth changes must be isolated, labeled, and reviewed separately from implementation improvements.

Report the compared dimensions, source and fixture provenance, normalizations applied, and exact commands/results. A missing optional or restricted corpus is reported as unavailable, not silently removed from the denominator. No normalization policy changes runtime status: planned features remain planned until their implementation and parity gates pass.
