# Private PDF text-item scalar normalization

Status: capture-only preparation. No Rust implementation, wire change or public
contract change is included. Authored captures are now frozen in
`crates/kordoc-pdf/tests/fixtures/pdfjs_text_normalization/`; root review is
required before RED/GREEN work begins.

## Source and pipeline boundary

Research used the ignored local oracle at kordoc commit
`bb71f7fb0bf51dd456d27505a8c04772df182144`, PDF.js 4.10.38. Relevant source
pins are:

- `kordoc/src/pdf/text-line.ts`, SHA-256
  `034b6a882eb913b723e21a1729e032bc45147e60acdf88aab655dfd415253072`.
- `kordoc/src/pdf/parser.ts`, SHA-256
  `2eb7018d17bf9bb3d39b7d2cb21145fe812a5257a239331aa13d44da9776bf70`.

The parser's order matters. It calls default `page.getTextContent()` and gets
the unrotated CropBox view at `parser.ts:189-195`. It then obtains the operator
list and performs named-glyph restoration, tracked-spacing restoration,
synthetic-space marking and occlusion detection at `parser.ts:196-210`. Only
then does it call `normalizeItems` at line 211. Hidden/out-of-page filtering,
vertical-column joining and tab-leader removal follow at lines 213-219. CropBox
translation follows annotations at lines 236-244. A DTO-only helper starts
after the operator-list stages and cannot claim full parser parity.

`normalizeItems` itself is much broader than scalar projection. Its per-item
loop at `text-line.ts:157-213` increments sequence before validation, trims
text, rounds position and dimensions, estimates font size from transform column
norms, classifies hidden text, applies targeted text rewrites, splits evenly
spaced strings, and adjusts vertical-text width/x. It then sorts y-descending
and x-ascending (`:215`; stable in the pinned Node runtime), removes fake-bold overlaps (`:217-235`), splits
overlaid runs (`:238-239`), and propagates the nearest space marker
(`:241-260`). Fake-bold and nearest-space scans can have quadratic behavior for
adversarial same-baseline inputs. Operator-derived synthetic-space state is not
part of the current private raw DTO.

## Proposed first slice

Implement only a private, bounded base projection from the already validated
`PdfJsTextItem` DTO to normalized scalar records. The helper should consume the
raw item vector, preserve each original one-based sequence, trim text with the
ECMAScript `String.trim()` WhiteSpace and LineTerminator set (TAB through CR,
SPACE, NBSP, U+1680, U+2000–U+200A, U+2028–U+2029, U+202F, U+205F,
U+3000, and U+FEFF; notably not U+0085), calculate
position/font/dimensions/hidden/orientation fields exactly as the scalar
portion of `normalizeItems`, then sort by descending y, ascending x, and
original sequence. The implementation should use an in-place unstable sort
with this complete key to avoid stable-sort scratch allocation while retaining
the same ordering the pinned Node runtime produces when coordinate keys tie.

Sequence increments for every raw item before text handling, including an
item that trims to the empty string. A whitespace-only item emits no
`PdfBaseTextItem`; the oracle records its position for later `hasSpaceBefore`
propagation, which this first slice intentionally defers. Consequently output
sequence values can contain gaps and must not be renumbered after filtering.

Suggested private type and entry point:

```rust
pub(crate) struct PdfBaseTextItem {
    pub(crate) text: String,
    pub(crate) x: f64,
    pub(crate) y: f64,
    pub(crate) width: f64,
    pub(crate) height: f64,
    pub(crate) font_size: f64,
    pub(crate) font_name: String,
    pub(crate) is_hidden: bool,
    pub(crate) sequence: usize,
    pub(crate) rotated_length: Option<f64>,
}

pub(crate) fn project_base_items(
    items: Vec<PdfJsTextItem>,
) -> Result<Vec<PdfBaseTextItem>, KordocError>;
```

For each item, compute `x/y` with the approved JS `Math.round` helper; compute
`scale_x=hypot(t0,t1)`, `scale_y=hypot(t2,t3)`, and
`font_size=Math.round(max(scale_x,scale_y))`; round raw width and height with
the same JS rule. A raw item is hidden when `font_size == 0`, or when raw width
is zero and trimmed text is nonempty. The vertical test is exactly
`abs(t1) > abs(t0) * 4`; for a vertical item, output width is
`max(1,font_size)`, subtract that width from x only when `t1 > 0`, and retain
`max(1,round(raw_width))` as `rotated_length`. Horizontal items retain rounded
width and have no rotated length. Keep rotation as item orientation only; do
not rotate page coordinates here.

Input normally arrives through `PdfJsTextDocument`, but the private helper may
also be called by Rust tests or future internal code that constructs DTO values
directly. Before any output reservation/copy, it must preflight item count
(100,000), UTF-8 text bytes (64 KiB/item and 2 MiB aggregate), font-name bytes
(128/item and 512 KiB aggregate), and finite raw numeric fields. Use
checked-add counters; reject overflow or cap violations as
`OutputTooLarge`/`ParseError` according to the existing error distinction.
Only after this complete preflight may it reserve output storage fallibly.
Output count cannot exceed input count and trimmed text cannot expand UTF-8
size. Reject malformed derived metrics/coordinates with `ParseError`; do not
clamp or silently drop malformed items.

The coordinate sort should not use `sort_by`, whose stable-sort scratch
allocation is not fallible. Use `sort_unstable_by` with a complete key of
descending y, ascending x, then the unique original sequence. Compare finite
floats numerically (`partial_cmp` after prevalidation), so negative and positive
zero are equal sort keys; do not use `total_cmp`, which distinguishes signed
zero. The sequence key makes the comparator total and preserves Node's stable
tie order without scratch allocation. Sort work is O(N log N), `N <= 100,000`.

The specific block is deliberately not the whole `normalizeItems` contract.
Defer compatibility rewrites (Kangxi radical NFKC, spaced phone strings and
large spaced uppercase labels), `splitEvenSpacedItem`, trailing-NUL width and
space-marker behavior, fake-bold deduplication, overlaid-run splitting,
synthetic-space propagation, hidden/out-of-page filtering, operator-list glyph
restoration, vertical-column joining, and subsequent crop-origin translation.
Do not expose a public page or text API, create IR blocks, or change the KPDF
worker DTO. In particular, sorting this slice does not establish final source
order after later dedup/splitting, and it is not a line or block layout claim.

The existing `PdfJsPage` and `PdfJsTextItem` types live in
`crates/kordoc-pdf/src/v8_runtime/text_document.rs`. The geometry helper is in
`crates/kordoc-pdf/src/geometry.rs` in the prepared geometry branch; it is not
yet part of `origin/main` at this plan's base. Implementation should be based
on the coordinator's joined PDF geometry candidate and own a new private
module plus its narrow module declaration and one focused integration test.
Do not edit the DTO, protocol, supervisor, worker binary, public contracts, or
shared limit definitions.

## Proposed authored regression and pinned capture

The new CC0 one-page PDF is in its own
`crates/kordoc-pdf/tests/fixtures/pdfjs_text_normalization/` directory, apart
from existing probe/text-document fixtures. It places raw text in source order
that differs from y-descending/x-ascending order, with horizontal and positive
90-degree vertical transforms, a rounded-zero font-size item and a zero-width
show. PDF.js emits an empty string item between two visible items and splits
the zero-width show into single-character items. Unicode trim, a literal
nonempty whitespace item and equal sort-key tie ordering are instead captured
as synthetic oracle inputs, because the actual PDF.js DTO does not retain those
raw distinctions. The source text items avoid rewrites/splitting except the
observed zero-width expansion, so the real oracle capture directly exercises
the proposed scalar projection.
The real PDF fixture and its raw worker capture are
`base_items.pdf` and `raw-worker-response.json`. The captured DTO has 20 PDF.js
items. The first visible `Alpha` item has raw sequence 1; the next entry is an
empty string, and the next visible `bottom` has sequence 3 after oracle
normalization. This is an empty PDF.js item, not a nonempty whitespace string.
PDF.js already strips the authored NBSP boundaries from `Alpha` and shifts its
reported x coordinate; this fixture therefore does not evidence Rust-side
Unicode trim. PDF.js expands the zero-width `zero-width` show into ten
single-character raw items, which is the observed behavior and must be recorded
as such. The real fixture captures out-of-order y/x items, a stable source-order
tie only in the synthetic oracle case below, one vertical item, a rounded-zero
font-size item and zero-width characters.

`oracle-scalar-projection.json` is an oracle capture of `normalizeItems` applied
to those actual raw DTO items. A separate synthetic input/output pair in
`oracle-synthetic-vectors.json` directly tests the unavailable raw distinctions:
a literal whitespace-only item at sequence 2 between visible sequence 1 and 3,
same-rounded-x/y visible items to test stable tie ordering, Unicode ECMAScript
trim scalars, and U+0085 which JavaScript `trim` retains. These are authored
synthetic `PdfTextItem` records, not evidence that PDF.js emits those exact
strings. Both actual and synthetic oracle outputs are pinned before RED.

`math-hypot-vectors.json` records real JavaScript `Math.hypot` and `Math.round`
results from V8 crate 152.2.0 alongside Rust `f64::hypot` outputs on
`aarch64-apple-darwin` with rustc 1.97.0. It includes a parity-significant
counterexample: for input bits `a=3fdfffffffffffff` and
`b=3e3ffffffffffff8`, V8 `Math.hypot` returns `a` (below 0.5) and JS font size
is 0, while Rust `f64::hypot` returns exactly 0.5 and Rust rounding is 1. A
subnormal pair also differs by one ULP while producing the same rounded size;
large finite operands demonstrate overflow-safe scaling. The pinned V8 source
`v8/src/builtins/math.tq` SHA-256 is
`60fbee44d8896b6d1e49eb89ab6cc67c2028048b6ef91e6ec297d64e37bb521c`; the
two-argument fast path is `sqrt((abs(a)/max)^2 + (abs(b)/max)^2) * max`.
Therefore direct use of Rust `f64::hypot` is not approved as an exact scalar
parity implementation. Root must review a compatibility strategy before code
starts; tests must not hide the rounded-size mismatch behind an approximate
comparison.

For any eventual RED/GREEN approval, tests should assert the actual authored
worker response hash/schema and exact oracle projection, verify the recipe in a
clean temporary directory, and separately exercise synthetic whitespace/trim/
stable-tie cases. Additional Rust tests should cover Math.round ties and signed
zero through the approved geometry helper; transform column-norm size;
fractional width/height rounding; vertical x/width/rotated-length for both
signs of `t1`; both hidden conditions; -0/+0 numeric sort ties; and nonfinite
computed metrics. Preflight tests must prove item-count, per-item/aggregate
text and font caps reject before output reservation or string copies; use a
private allocator/test hook only if required to observe ordering. Reuse
existing limits; do not weaken them.

## Oracle gaps and follow-up

This first slice can establish only the initial scalar projection and stable
coordinate ordering for an authored ordinary-text PDF. OperatorList-dependent
glyph repair/spacing/occlusion, fake-bold deduplication, overlapping-run split,
space hints, compatibility normalization, vertical columns, hidden filtering,
CropBox shift and full line/layout parity remain unverified. Each requires a
separate bounded implementation and discriminating authored fixture. No broad
PDF parity score or production parser registration follows from this
checkpoint.
