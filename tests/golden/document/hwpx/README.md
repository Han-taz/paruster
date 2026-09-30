# HWPX candidate fixtures

All twelve `.hwpx` inputs under `fixtures/` are generated from the CC0-1.0
[Rust H0 recipe source](../../../../crates/kordoc-hancom/tests/support/hwpx_fixture.rs).
They contain no document/oracle input bytes. ZIP records have fixed order,
stored members, UTF-8 XML, and the 1980-01-01 timestamp. The fixed test password
is `fixture-password`; it is not a credential.

The [H0 provenance and input digests](../../../../crates/kordoc-hancom/tests/support/README.md)
and [unchanged full oracle captures](../../../../crates/kordoc-hancom/tests/support/hwpx-oracle-results.jsonl)
remain authoritative evidence. This directory makes generated inputs available
to installed-wheel tests without Rust or the migration oracle at test runtime.

Regenerate the exact inputs from the workspace:

```sh
PARUSTER_HWPX_FIXTURE_EXPORT="$PWD/tests/golden/document/hwpx/fixtures" cargo test -p kordoc-hancom --test hwpx_integration --locked export_generated_fixture_bytes_for_oracle_capture
```

The first seven captures cover ordinary, spine, nested-table, layout/fallback,
and both encryption PRF results. The candidate test compares their entire
recursive wire result with no normalization or omitted dimensions. Malformed
section diagnostic text remains a separately recorded discrepancy. Four
adversarial depth/record cases intentionally are not semantic-parity inputs;
their captured observations and approved security differences are preserved.
No case is added to the protected document parity numerator by this directory.
