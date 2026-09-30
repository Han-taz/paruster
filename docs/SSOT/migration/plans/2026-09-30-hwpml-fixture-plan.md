# HWPML fixed-input oracle capture foundation

Status: fixture/capture work under approved P5; no Rust parser capability,
registry, Python API, protected manifest or scoring promotion in this slice.

The first HWPML checkpoint freezes small authored CC0 input bytes and exact
public legacy results before private Rust implementation. The approved
[product plan](2026-09-30-product-port-implementation-plan.md) requires normal
text, namespaces/BOM, styles/headings, nested tables, page selection, empty
body, malformed partial recovery and hostile XML/resource boundaries.

## Ownership

Luna A owns new Hancom HWPML support/generator/capture files and small golden
inputs with provenance README. SOL reviews semantic/security capture scope.
The coordinator owns offline provenance/inventory tests, SSOT/WIKI and protected
publication. No existing worker edits these new paths. Private parser/core/
Python wiring follows separately with coordinator approval.

## Evidence requirements

Generate small deterministic UTF-8 fixtures without copying migration-oracle
source or fixtures. Include ordinary text/metadata, BOM/namespaces, heading
mapping and ignored header/footer/shape/autonumber text, empty bodies/sections,
page options, nested/merged tables and trailing empty column options,
recoverable malformed XML, predefined/legacy nonbreaking-space behavior and
DTD/entity-reference observations without claiming universal resolution safety. Do not commit a 51 MiB size-limit fixture;
use a later bounded generator or reduced-cap native regression for that limit.

Use the pinned read-only local legacy snapshot for one-time result capture;
record snapshot and owned parser/shared-XML/table source hashes, input hashes,
exact options and complete result/error JSON. No normalization or fabricated
Rust parity. Failed/unavailable captures are recorded honestly. The oracle is
never a runtime, packaging, CI or ordinary test dependency. Offline tests verify
regenerated input bytes, fixture/capture inventories and hashes. Preserve all
existing protected fixtures, expected results, security limits and success
accounting.

## Later implementation boundaries

The existing HWPX event reader can supply ordered local-name/UTF-8 XML content
under input, depth, node, text and tree budgets after its narrow crate visibility
is approved. HWPML has an outer 50 MiB input cap. Strict parsing is insufficient
for the legacy malformed-DOM partial result: private bounded recovery must be
specified and tested before public qualification. HWPML tables have different
source tags/coordinates and legacy skip limits; reusing HWPX table lowering
without a mapping is incorrect. Full P5 includes these behaviors and HWP3,
not merely the first valid-XML text slice.

## Local checkpoint evidence

Eight byte-pinned CC0 XML fixtures and 13 complete public oracle captures are
frozen with the pinned source/runtime provenance in the
[fixture README](../../../../tests/golden/document/hwpml/README.md).
Four offline contract tests regenerate all small inputs, verify exact
capture/options inventory, stream-hash the 50 MiB+1 recipe and distinguish
recoverable entity warnings from unclosed-XML errors. The final capture SHA-256
is `aba0dd587753d2c898340be9fe10e4dfd79a535c9f1356c4c83f0ede250551c6`.

The escaped DTD case is a literal-text control. The separate actual entity
reference at a harmless nonexistent file URI returns literal text and a
`MALFORMED_XML` warning in the oracle. This observation alone does not prove
zero attempted file I/O. Rust DTD/entity refusal remains a separate hard
security boundary for the later parser. A fatal unclosed XML input returns the
sanitized public `PARSE_ERROR`. The generated size-limit observation returns
`DECOMPRESSION_BOMB`; no 51 MiB file is committed.

The JS capture helper exists only under ignored `build/oracle-capture/`.
Ordinary verification uses Python stdlib and frozen observations, and requires
neither the oracle nor Node.js. This checkpoint changes no native parser,
Python API, IR, registry, protected fixture/result or scoring inventory.
