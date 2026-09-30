# Private bounded HWPML text lowering

Status: coordinator-approved under P5; depends on H0 capture checkpoint PR #28.
No shared contract, registry, core, Python or public parser export change.

## Ownership

Luna A owns new `crates/kordoc-hancom/src/hwpml.rs` and `src/hwpml/tests.rs`.
The coordinator owns the private module declaration and the narrow HWPX
re-exports `parse_xml_critical`, `XmlContent`, `XmlNode`, plus SSOT/WIKI.
Existing HWPX parser behavior and XML policies are unchanged. SOL independently
reviews semantic ownership, transactional output and allocation boundaries.

## First implementation checkpoint

Start with meaningful failing unit expectations against a private unsupported
stub. Implement only valid bounded HWPML text, metadata, headings and section
page selection, comparing exact IR facts to the unchanged H0 normal/empty
captures. Public result formatting remains in shared projections.

Check the inclusive 50 MiB outer input cap before any clone or XML processing.
Normalize `&nbsp;` to same-length `&#160;` only when present, with bounded
fallible allocation; do not add entity expansion. Reuse strict XML input,
depth, node/attribute, text and tree limits. DTD/custom declarations and fatal
syntax remain hard strict errors; tolerant recovery is a separate future
checkpoint and these strict results are not claimed to match the legacy
malformed/DTD captures.

Traverse ordered `XmlContent` with namespace-local names. Map only direct
DOCSUMMARY TITLE/AUTHOR/DATE and HEAD/MAPPINGTABLE/PARASHAPELIST Outline
PARASHAPE definitions. Outline source Level 0 maps to 1 and clamps at 6.
Within paragraphs, collect ordered CHAR descendant text, ignoring
TABLE/PICTURE/SHAPEOBJECT/AUTONUM content; inline footnotes contribute their CHAR text. Entire
HEADER/FOOTER sections are ignored. Section traversal otherwise recurses
through wrappers, including PICTURE/SHAPEOBJECT/AUTONUM, as observed in the
pinned oracle; paragraph-only suppression is not extended to that traversal. Trim Unicode whitespace once and omit
empty paragraphs. BODY's direct SECTION children use 1-based page ordinals;
page selection retains original ordinals and metadata. Do not invent
page_count or source page evidence absent from captured results.

Structural tables are intentionally unsupported in this slice: return a hard
existing unsupported error rather than discard them and claim a complete
document. Dedicated HWPML table coordinates/nesting and partial XML recovery
are required before public qualification. Preserve all frozen captures.

Reuse `LoweringBudget` precharging every retained/transient copy, vector push,
shape mapping and duplicated outline text. Avoid unmetered tree text helpers
and descendant-vector collection. Validate exact inclusive/threshold-1 output
budget, oversized input preflight, XML resource limits, wrong root, DTD/entity
rejection, table rejection and skipped structures without mutating the shared
limits or security gates. The private parser is transactional on failure.

Rust unit tests, Hancom strict Clippy/fmt, unchanged HWPX regression suite,
workspace/core/Python/artifact gates and independent review must pass before
publication. HWPML recovery, table lowering, options, Python registration,
HWP3/HWP5 and corpus parity remain separate work.
