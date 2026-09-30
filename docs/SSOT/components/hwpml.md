# HWPML component

Status: authored fixed-input capture foundation is locally reviewed; private
Rust implementation and public capability remain pending.

The H0 checkpoint contains eight deterministic CC0 XML fixtures and 13 complete
public results from the pinned read-only migration oracle. The
[fixture README](../../../tests/golden/document/hwpml/README.md) records exact
input/capture/generator hashes, oracle source identities and one-time runtime
provenance. Four offline contract tests regenerate the small inputs, pin full
capture/options inventory and stream-hash an oversized in-memory recipe.
Neither Node.js nor the oracle is needed by ordinary tests or delivery.

The captures cover metadata, BOM/namespaces, heading mapping, section pages,
ignored structural text, inline footnote text, nonbreaking spaces, nested and
merged tables, trailing-column options, empty bodies and XML error observations.
Recoverable undefined references produce literal text plus `MALFORMED_XML`;
unclosed XML produces sanitized `PARSE_ERROR`. The escaped DTD input is only a
literal-text control. A separate actual external reference at a harmless
nonexistent URI returns a literal reference and warning; this does not prove
that no file access was attempted. The generated 50 MiB+1 input returns
`DECOMPRESSION_BOMB` and is represented by a recipe rather than a large file.

The [H0 plan](../migration/plans/2026-09-30-hwpml-fixture-plan.md) keeps these
observations separate from later Rust security and semantic implementation.
A private native parser must enforce the 50 MiB outer cap, bounded XML/tree/text
and output allocation, and DTD/custom-entity rejection. HWPML's source table
coordinates and tolerant malformed-XML behavior require dedicated lowering
and bounded recovery; HWPX lowering is not an interchangeable implementation.
No public registration, Python parsing API, HWP3/HWP5 completion or protected
success accounting is advanced by this checkpoint.
