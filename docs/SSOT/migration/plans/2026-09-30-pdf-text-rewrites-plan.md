# Private PDF text rewrite parity checkpoint

Status: private implementation candidate. The scoped helper and tests are
implemented; no worker wire, public API, IR, or parser-registration change is
included. `unicode-normalization = 0.1.25` is an optional `kordoc-pdf`
dependency enabled only by `pdfjs-v8`.

## Corrected pipeline boundary

The implemented helper is pure text transformation, independent of coordinates,
sort order, sequence, page structure, or the existing scalar projection:

```rust
fn rewrite_text(text: &str, rounded_font_size: f64) -> Result<String, KordocError>;
```

It receives a text field and the already rounded font size needed by the
uppercase-label rule, first applies ECMAScript `String.trim()`, and returns
only transformed text. This checkpoint does not wire it into
`PdfBaseTextItem`, `normalizeItems`, a parser, or any final layout stage.
Text-only vectors from non-splitting `normalizeItems` calls are the focused
parity evidence. The numeric-versus-Hangul split example is kept separately
as a source-order observation for a later pipeline integration; it does not
establish full-field parity for this helper.

This separation corrects the prior draft's pipeline claim. Current Rust base
projection performs orientation metrics and a global sort, while oracle
`normalizeItems` performs per-item rewrite, then split, then orientation and
sorting. The future text-only helper avoids falsely presenting the current
base-item helper as a full `normalizeItems` stage. A later integration must
reorder the stages explicitly so text rewrites precede splitting and page
orientation/sort. It is outside this checkpoint.

## Pinned source behavior

The ignored local oracle is commit
`bb71f7fb0bf51dd456d27505a8c04772df182144`, package `kordoc` 4.16.3. The source
`kordoc/src/pdf/text-line.ts` has SHA-256
`034b6a882eb913b723e21a1729e032bc45147e60acdf88aab655dfd415253072`.
`normalizeItems` at lines 190–213 applies these text operations in order:

1. ECMAScript `String.trim()` removes leading and trailing ECMAScript
   whitespace/line terminators, including BOM U+FEFF, NBSP, U+2000–U+200A and
   U+2028–U+2029. It does not remove U+0085.
2. Only each code point in U+2F00–U+2FD5 is individually normalized with
   `.normalize("NFKC")`. Adjacent fullwidth Latin, circled digits, out-of-range
   compatibility characters, and astral values are not NFKC-normalized by
   this rule.
3. The numeric rewrite matches
   `/^[\d\s\-().·,☎]+$/`, requires at least one ASCII `\d` and one literal
   U+0020, then removes only U+0020 with `/ /g`. JavaScript `\d` is ASCII-only;
   ECMAScript `\s` includes Unicode whitespace but not U+0085. A matching tab,
   line break, NBSP, or ideographic space remains after ordinary spaces are
   removed.
4. At rounded `fontSize >= 14`, the uppercase rewrite matches
   `/^[A-Z0-9?!&'’](?: [A-Z0-9?!&'’]){2,}$/` and deletes its U+0020 separators.
   It accepts three or more single ASCII groups with exactly one space between
   them; non-ASCII letters and repeated spaces are near misses.
5. Only after those rules does the oracle call `splitEvenSpacedItem`.

Lone UTF-16 surrogate halves are excluded because the current private worker's
JSON string decoder represents valid UTF-8/Rust strings only. Captured astral
scalars prove that neighboring supplementary characters survive the targeted
radical replacement unchanged.

## NFKC compatibility evidence

The authored `radical-inputs.json` enumerates all 214 scalars in the exact
U+2F00–U+2FD5 range. `oracle-radical-mappings.json` contains 214 direct
evaluations of the exact per-character source expression
`String.fromCodePoint(cp).normalize("NFKC")`. The source's radical regex
applies that expression independently to each matched character, so these
records pin that branch without invoking later item-level processing. The 20
text rewrite vectors are actual non-splitting `normalizeItems` captures. Every
mapping changes the input scalar; each produces exactly one output scalar,
with a maximum three-byte UTF-8 output. The separate mixed-string vector
`Ａ⼀①⿕⿖ -> Ａ一①龠⿖` proves that only the targeted block changes, including
the inclusive U+2FD5 and excluded U+2FD6 boundary.

The workspace pins `unicode-normalization = 0.1.25` as an optional `kordoc-pdf`
dependency enabled by `pdfjs-v8` (checksum
`5fd4f6878c9cb28d874b009da9e8d183b5abc80117c40bbd187a1fde336be6e8`). The
crate's `UNICODE_VERSION` is `(17, 0, 0)`. Oracle capture runtime is Node
v22.22.0, V8 12.4.254.21-node.33,
ICU 78.2, Unicode 17.0, so the Unicode table versions align. A separate offline
Rust 0.1.25 probe applying `char.nfkc()` to each radical matched all 214 pinned
Node outputs; its maximum compatibility decomposition and NFKC output lengths
were both one scalar.

The crate's `UnicodeNormalization` trait implements `nfkc()` directly for
`char` as an iterator. Its `Recompositions` and `Decompositions` buffers are
`TinyVec` with four-element inline storage. Since each of these 214 scalar
normalizations has a single-scalar compatibility decomposition and a
single-scalar output, this exact restricted use stays within those inline
buffers. The helper calls `try_reserve_exact(text.len())` once, streams
non-radical characters unchanged, and streams each selected `char.nfkc()` into
that output. All observed replacements are three-byte UTF-8 scalars, so this
range does not expand the source byte count. The helper uses this pinned
optional dependency.

## Implemented helper and test scope

- The private `rewrite_text(text, rounded_font_size)` helper is covered by
  text-only tests; it is not wired into scalar items or a parser.
- Compare each non-splitting authored case's one returned `text` value from
  the pinned `normalizeItems` capture.
- Compare all 214 radical outputs by exact Unicode scalar/UTF-8 string.
- Assert mixed fullwidth/circled/out-of-range and astral neighbors remain
  byte-for-byte intact while in-range radicals normalize.
- Cover numeric positive/negative classes, ASCII-only digits, ECMAScript
  whitespace versus literal-space deletion, U+0085, and uppercase 13/14
  rounded-font-size gate with positive and negative patterns.
- Keep `split_order_evidence` separate and do not assert it through the pure
  helper. It exists for review of a future text-rewrite-before-split
  integration.
- Use fallible output reservation and guarantee linear scanning with no
  unbounded regular-expression backtracking or output expansion. For this
  single-item helper, enforce the 64 KiB raw/output text cap and reject a
  non-finite rounded font size before copying. It cannot enforce the document
  item-count or aggregate 2 MiB text cap; those remain the upstream DTO/caller's
  responsibility until a later integration.

## Authored captures and limits

`crates/kordoc-pdf/tests/fixtures/pdf_text_rewrites/` contains CC0 authored
synthetic input vectors, an offline deterministic generator, text-only oracle
captures for 20 non-split cases, separate split/order evidence, and all 214
radical mappings. `README.md` records exact byte counts, SHA-256 values,
versions, source pin, temporary capture script/loader hashes, and capture
method. The temporary loader stubs only unrelated `line-detector.js` imports;
the source `normalizeItems` function executes unchanged. The loader and capture
script are research-only and are neither checked in nor used by tests,
packaging, or runtime.

The split/order evidence uses raw sequence 1 `1 2 3` at lower y and sequence 2
`가 나 다` at higher y. Numeric rewriting produces one `123`; the Hangul
string reaches the later splitter and becomes `가`, `나`, `다` at sequences
2, 2.001, 2.002. The complete oracle function then coordinate-sorts the
fragments before `123`. Later fake-bold deduplication, overlap splitting, and
space propagation run in the oracle but are no-ops for this input. This is
evidence about future pipeline order only, not a required assertion for
`rewrite_text`.

No new test, generator, or runtime path may depend on the ignored oracle's
presence. The capture script's external source path is research provenance,
not a checked-in dependency.
