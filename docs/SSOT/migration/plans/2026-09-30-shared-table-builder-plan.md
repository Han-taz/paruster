# Shared table builder extraction plan

Status: coordinator-approved private extraction; no IR wire or parser contract change.

## Goal and boundary

Extract source-neutral rectangular table placement from `kordoc-core` into a small sibling crate while preserving the existing core call surface and every current table-builder result. The new crate depends only on `kordoc-ir` and `std`. It must not depend on `kordoc-core`, Hancom, Python, or any format crate. This keeps the graph acyclic: `kordoc-ir <- kordoc-tables <- kordoc-hancom <- kordoc-core`, with `kordoc-core` also depending directly on `kordoc-tables`.

This change does not change IR types or wire fields, HWPML parsing, HWPX source parsing, Markdown/table policy, or caller-specific placement semantics. In particular, the existing core placement rejects anchors and spans that exceed 200 columns; no clamp mode is added until HWPML parity captures approve it.

## Files and ownership

Coordinator owns workspace membership, both consumer dependencies, `Cargo.lock`, dependency-direction contract test, and SSOT/WIKI integration. Shared-table implementation owns:

- `crates/kordoc-tables/src/lib.rs` and focused new crate source/tests (coordinator owns the new crate manifest)
- `crates/kordoc-core/src/table/builder.rs`, retaining the existing core-facing facade and policy-only `LegacyLayoutFlattening` code
- focused crate/core tests

The extraction deliberately leaves `crates/kordoc-hancom/src/hwpx/tables.rs` and all HWPML files unchanged. A later HWPML table-lowering slice can consume the rich API after separate source-capture review.

## API

Move the existing `CellContext` and `BuildTableOptions` definitions without adding fields, preserving current struct literals. Keep the core helper signature through a facade:

```rust
pub(crate) fn build_table(
    rows: &[Vec<CellContext>],
    options: BuildTableOptions,
) -> Result<IrTable, KordocError>;
```

The sibling crate exposes the same two-argument `build_table` as a compatibility convenience and a metered borrowed form for future parser callers:

```rust
pub trait AllocationBudget {
    fn charge_bytes(&mut self, bytes: usize) -> Result<(), KordocError>;
}

pub struct TableCellBudget { used: usize, limit: usize }
impl Default for TableCellBudget { /* inclusive 2,000,000 cells */ }

pub fn build_table_with_budget<B: AllocationBudget>(
    rows: &[Vec<CellContext>],
    options: BuildTableOptions,
    cells: &mut TableCellBudget,
    allocations: &mut B,
) -> Result<IrTable, KordocError>;
```

For nested IR blocks, preserve `CellContext` exactly and add a separate rich owned input:

```rust
pub struct RichCellContext {
    pub cell: CellContext,
    pub blocks: Option<Vec<IrBlock>>,
}

pub fn build_table_with_blocks<B: AllocationBudget>(
    rows: Vec<Vec<RichCellContext>>,
    options: BuildTableOptions,
    cells: &mut TableCellBudget,
    allocations: &mut B,
) -> Result<IrTable, KordocError>;
```

The rich form consumes inputs and transfers ordinary cell strings and valid block vectors into the output, avoiding deep cloning. Its attachment policy is intentionally conservative and source-compatible: attach a source's nested blocks only to the cell at its source coordinate when the built anchor remains unclaimed and the built text/spans match the original context; never merge rich blocks merely because grid collision merged text. HWPML source `parser.ts:368-390` uses this coordinate-first / text-and-span fallback / claimed-target behavior, so collided contexts must not receive guessed blocks. The first extraction implements placement result mappings sufficient for this rule but does not implement HWPML parsing.

`kordoc-core/src/table/builder.rs` reexports/aliases the original names and delegates the original function to the sibling compatibility builder. `LegacyLayoutFlattening` and `flatten_layout_tables` remain in core because they encode core-only layout policy, not generic grid construction. Existing tests and call sites stay in their current file and continue calling the same facade.

## Allocation and placement requirements

The metered path runs placement preflight before constructing the rectangular IR grid. All arithmetic is checked. It preserves existing limits: 200 columns, 10,000 rows (or lower `max_rows` option), 2,000,000 logical cells, positive spans, and current `OutputTooLarge`/`ParseError` classes and message behavior where tested.

Before every material allocation, charge conservative bytes through `AllocationBudget`; use `try_reserve_exact` or fallible map reservation. Account for input placement records, occupancy/owner data, placement rows, output row vectors, all default/filler `IrCell`s, and retained cell-text copies/appends. Use the same preflight geometry for both borrowed and rich forms. If accounting fails, return before reserving the rejected allocation. The old convenience entry point uses an internal no-op allocation budget to preserve compatibility, but still uses checked arithmetic, fallible reserves, and grid limits. A later Hancom caller can implement `AllocationBudget` for its existing `LoweringBudget` without making this crate depend on Hancom.

`TableCellBudget` is a separately shared logical-grid count. Charge actual computed `rows * cols`, checked and inclusive, before allocating each result matrix; nested callers share one budget across all produced child tables. It is not based only on declared dimensions and does not change column policy. `with_limit` belongs under tests or a test-only constructor so production callers cannot weaken the fixed default cap.

The existing occupancy policy remains unchanged: first anchor owns a slot; collision text appends in source order with a newline; spans clip around existing occupied cells; default mode trims textless trailing columns; `keep_anchored_empty_cols` retains only trailing columns with real anchors; spans are clipped to the effective final width; `has_header` remains `row_count > 1`. No new rich-block behavior may alter these fields or the core output.

## Test-first sequence

Before implementation, add tests in the new crate against the approved signatures and verify the expected failures are real compiler/API or budget failures, not fixture changes. Keep the current core behavior tests untouched and use them as post-extraction compatibility checks.

1. A budget implementation that rejects any charge returns `OutputTooLarge` before a non-empty grid reserve; a recording budget demonstrates grid/occupancy allocations are charged. Then an inclusive N/N-1 test for a small 1x1 grid: exact computed bytes succeeds, one byte less fails before final grid allocation.
2. Rich-input test constructs one anchored cell with a paragraph and nested table block, and asserts ownership reaches `IrCell.blocks` with table text intact; a separate overlapping-cell case proves text collision still appends `a\nb` while blocks from the collision are not blindly attached to the first source's owner.
3. Inject a reduced `TableCellBudget` for exact inclusive limit and one over. A geometry-only preflight seam verifies 2,000,000 accepted / 2,000,001 rejected without allocating a 2M grid in the test.
4. At the core facade, run all current `builder_*` tests unchanged: addressed-span collision, anchored trailing empty columns, huge span rejection, mixed implicit/explicit addresses, all-empty grid geometry, and layout-flatten policy. Run the existing Markdown table builder test too.
5. A depth test creates nested `IrBlock` trees on rich inputs: deepest accepted supported boundary remains owned, the next level returns the existing `OutputTooLarge`; no recursion cap is increased. Later HWPML integration will separately assert the source depth-8 structures and flattened text behavior.

No HWPML capture or fixture is added in this extraction. The H0 `nested_table.xml` assertions remain for the later adapter PR: default and false produce 2x2; true produces 2x3 with the anchored empty final column; default clips the second-row merged span from 2 to 1 while true keeps 2; outer cell flat text remains `nested cell\ninner cell` and nested blocks remain paragraph then inner table. HML-specific column clamping is out of scope until pinned oracle evidence and separate approval.

## Gates

Run focused new crate tests; `cargo test -p kordoc-core --lib table::builder --locked`; `cargo test -p kordoc-core --lib markdown::tests --locked` (or the exact table-specific Markdown test); `cargo test -p kordoc-tables --locked`; `cargo fmt --check -p kordoc-tables -p kordoc-core`; strict all-target Clippy for both crates; then `cargo test --workspace --locked`. Also run the coordinator-owned dependency-direction contract test and review `cargo tree -p kordoc-tables` to show only `kordoc-ir` and std dependencies.

## Scaffold evidence

The coordinator first runs the exact workspace inventory and sibling layering
contracts against the old workspace: two expected failures (missing member and
manifest), three passing existing contracts. Registering the IR-only sibling,
exact consumer versions and lock entry makes all five contracts pass; the
new crate also passes an offline Cargo check. Rich payloads are caller-owned
and already allocated; the consuming builder meters its new allocations and
any copying/appending, and documents caller ownership of input payload budgets.

Moved table construction remains included in the required 80% coverage gate: add
`-p kordoc-tables` to the existing IR/core coverage command rather than removing
its code from the measured set. The matching contract first fails against the
old workflow, then passes after the extension. Keep both workspace lockfiles
consistent with the new source dependency.
