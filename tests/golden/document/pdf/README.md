# PDF Task 0 fixtures and probe evidence

`generate.py` is a deterministic, Python-stdlib-only generator for original
fixtures released under CC0-1.0. Run `python3 tests/golden/document/pdf/generate.py`
from the repository root to regenerate them. Inputs and outputs are restricted
to this directory; no migration-oracle files are used.

## Bounded-reader evidence

The free-xref regressions were first run against the implementation from
`d44d4a6` (with the new regression tests applied locally): both
`rejects_more_than_one_million_free_classic_xref_ids` and
`rejects_more_than_one_million_free_xref_stream_ids` failed because reader
construction incorrectly returned `Ok`. After charging all distinct IDs
before map insertion, both assert `Budget(Objects)` and pass. The remaining
rows are post-fix measurements and record the parser result asserted by each
test.

Measurements were collected on macOS with one test process at a time using:

```sh
/usr/bin/time -l cargo test --manifest-path crates/kordoc-pdf/Cargo.toml \
  --test TARGET --locked TEST -- --exact --test-threads=1
```

The table records `/usr/bin/time -l`'s maximum resident set size (RSS) and
peak memory footprint in bytes for each complete `cargo test` invocation,
including its test harness. RSS varies with allocator/runtime state and is
evidence, not a portable resource guarantee. Parser outcomes are asserted in
the named test; `Ok` means the deliberately supported path completed.

| Probe (`TARGET::TEST`) | Asserted parser result | Max RSS (bytes) | Peak footprint (bytes) |
| --- | --- | ---: | ---: |
| `pdf_security::rejects_reader_stream_limit_before_copying_payload` | `Budget(StreamBytes)` | 69,615,616 | 26,837,520 |
| `pdf_security::enforces_cumulative_decode_limit_across_repeated_stream_access` | `Budget(DecodedBytes)` | 102,875,136 | 24,494,608 |
| `pdf_security::rejects_reader_reference_chain_at_depth_65` | `Budget(ObjectDepth)` | 44,433,408 | 22,692,296 |
| `pdf_security::rejects_million_and_first_xref_object_before_map_growth_past_cap` | `Budget(Objects)` | 45,416,448 | 23,691,744 |
| `pdf_security::rejects_more_than_one_million_free_classic_xref_ids` | `Budget(Objects)` | 130,498,560 | 22,708,680 |
| `pdf_security::rejects_more_than_one_million_free_xref_stream_ids` | `Budget(Objects)` | 44,613,632 | 22,888,928 |
| `pdf_security::rejects_large_explicit_xref_index_without_range_vector_expansion` | `Budget(Objects)` | 44,482,560 | 22,708,704 |
| `pdf_security::bounds_objstm_member_copy_before_allocating_it` | `Budget(DecodedBytes)` | 103,038,976 | 24,658,448 |
| `pdf_security::rejects_large_filter_array_without_materializing_filter_names` | `UnsupportedFilter` | 44,335,104 | 22,577,632 |
| `pdf_security::rejects_extreme_objstm_member_index_without_arithmetic_panic` | `Corrupted` | 45,318,144 | 23,593,464 |
| `pdf_security::compressed_object_container_cycles_terminate_before_stack_growth` | `ReferenceCycle` | 44,351,488 | 22,626,760 |
| `pdf_security::sparse_500_mib_file_is_rejected_from_metadata_before_reading` | exact 500 MiB accepted; 500 MiB + 1 rejected as `Budget(SourceBytes)` | 45,793,280 | 24,166,856 |
| `pdf_security::parses_valid_near_cap_sparse_source_without_an_internal_clone` | valid sparse source parsed | 466,583,552 | 24,216,104 |
| `pdf_security::unwinds_object_and_form_recursion_after_errors` | `Budget(ObjectDepth)` and `Budget(FormDepth)` | 46,202,880 | 24,494,608 |
| `pdf_security::ignores_long_unrecognized_classic_xref_line_without_token_vec` | `Ok` | 44,400,640 | 22,643,144 |
| `pdf_objects::rejects_malformed_stream_length_and_reports_encryption_without_secret_data` | `Corrupted`; `Encrypted` | 44,400,640 | 22,561,296 |
| `pdf_objects::detects_encrypt_key_after_dictionary_close_text_in_a_literal_string` | `Encrypted` | 47,579,136 | 25,903,680 |
| `pdf_objects::detects_encrypt_key_after_comment_between_trailer_and_dictionary` | `Encrypted` | 44,695,552 | 23,003,664 |
| `pdf_objects::does_not_scan_stream_payload_for_object_references` | `Ok` | 47,235,072 | 25,526,824 |
| `pdf_objects::rejects_xref_stream_field_widths_that_do_not_fit_machine_integer` | `Corrupted` | 47,890,432 | 26,165,800 |
| `pdf_objects::rejects_generation_mismatch_and_reference_cycles` | `GenerationMismatch`; `ReferenceCycle` | 47,415,296 | 25,838,120 |

The near-cap probe creates a valid 500 MiB sparse file, reads it into the one
caller-owned source `Vec`, and passes that slice to the reader. Its measured
maximum RSS was 466,583,552 bytes; macOS reports this variably for sparse,
zero-filled pages, so it is not a strict memory bound. The code path retains a
borrowed source slice and the observed peak has no second 500 MiB resident
increment. This probe does not claim that a general caller can avoid its own
input allocation. The separate metadata-only probe verifies the cap and
cap-plus-one decision without reading sparse-file contents.
