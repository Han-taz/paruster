# Bounded HWPX unfinished-XML warning parity

Status: Merged in PR [#26](https://github.com/Han-taz/paruster/pull/26) as `cb17578e6ee2112762b4abdb2d64f0114f3eea6b` after independent review, local verification and every protected hosted gate passed. No shared schema change.

Restore the known frozen malformed-section EOF diagnostic while retaining
transactional recovery, sound neighbors, warning code/page, and hard resource
failures. Only the message currently differs in the original H0 capture.
Original inputs/expected captures and protected scoring remain unchanged.

## Ownership

Luna A owns private `hwpx/{xml,sections}.rs` and focused XML/section/integration
tests. SOL owns read-only safety/parity review. Coordinator owns Python exact
full-result comparison, SSOT/WIKI, branch publication and protected merge.
Other workers do not edit this branch.

## Required behavior and evidence

Record actual RED for the pinned `malformed_section` result before implementation.
At unfinished-tag EOF preserve qualified names in opening order, e.g.
`hs:sec, hp:p`, through a private bounded diagnostic path. Avoid an input-sized
unmetered name copy on normal parsing: prefer borrowed source ranges or
budgeted bounded stack storage. XML input/tree/node/text/depth budgets remain
unchanged. Diagnostic output is metered before allocation and never becomes
an attacker-controlled format string or native/path disclosure.

The recovering section formats the observed Korean ordinal wrapper using the
existing `PARTIAL_PARSE` code and original section/page ordinal. Match the
original complete result exactly, without normalization. Non-EOF malformed
syntax must not receive a fabricated unfinished-tag diagnostic. Existing
resource faults remain hard `DECOMPRESSION_BOMB`; valid neighboring section
state is committed only according to the existing transactional policy.

Use native RED/GREEN and Python source/installed-wheel evidence, original H0
hashes/full-result captures, strict workspace/Clippy/fmt/docs/security gates,
relevant bounded XML/package fuzzing and protected hosted checks. Review
common-case timing/RSS against matched release wheels if names are newly copied.
This closes only the known EOF diagnostic gap; other malformed diagnostics,
ODF checksum interoperability and representative/restricted-corpus parity stay
explicitly pending. Public API/IR/error/MCP shapes do not change.
