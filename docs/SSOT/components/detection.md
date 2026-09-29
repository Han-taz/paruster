# Bounded format detection

`kordoc-core` owns content-based format detection. It reuses `kordoc-ir::FileType`; no core-local duplicate enum is allowed. Detection accepts bounded in-memory bytes and never extracts archive members.

## Limits and errors

- Raw input is limited to 524,288,000 bytes. The limit itself is accepted; a larger length returns `OUTPUT_TOO_LARGE` before container parsing or copying.
- ZIP preflight accepts at most 100,000 central-directory entries and at most 1,073,741,824 declared uncompressed bytes, inclusive.
- ZIP preflight uses checked `u64` arithmetic and bounded scans without per-entry allocation. It supports single-disk ZIP64 EOCD/locator records and ZIP64 size/offset extra fields so the entry-count limit is meaningful above 65,535. Every multidisk representation is rejected.
- Forged or unsafe central-directory metadata—including out-of-bounds spans, malformed variable lengths, inconsistent counts, missing required ZIP64 values, arithmetic overflow, and a limit excess—returns `ZIP_BOMB`. A structurally valid empty or unrelated ZIP returns `unknown`.
- A malformed or unrelated CFB/OLE container returns `unknown`; detection does not claim document corruption before a format is established.

## Recognition order

Top-level magic recognition is ordered: HWP3, ZIP, OLE2/CFB, PDF, HWPML XML, then image. PNG uses its full eight-byte signature, JPEG uses its SOI plus following marker prefix, and WebP requires the twelve-byte `RIFF....WEBP` form. This deliberately permits safe recognition from the minimal discriminating PNG/JPEG prefixes even though the former oracle helper required a twelve-byte buffer.

ZIP names are matched as exact, case-sensitive ASCII paths. After successful preflight, precedence is:

1. `xl/workbook.xml` -> `xlsx`
2. `word/document.xml` -> `docx`
3. `ppt/presentation.xml` -> `pptx`
4. `Contents/content.hpf`, `mimetype`, or an entry below `Contents/` -> `hwpx`

CFB recognition considers stream entries, never storage names. A stream whose basename is exactly `Workbook` or `Book` selects `xls` first. Otherwise exact `FileHeader`, exact `DocInfo`, or `BodyText/Section` followed only by decimal digits selects `hwp`. Storage lookalikes and substring matches remain `unknown`.

## Foundation dispatch

Rust `try_parse` returns `Result<ParseSuccess, ParseDispatchError>`. The core-local error carries the detected `file_type`, stable `code`, and safe `message`; it does not change the shared `KordocError` wire contract. Empty input is `EMPTY_INPUT` with `unknown`, detection safety errors retain their code with `unknown`, and every detected or unknown format is `UNSUPPORTED_FORMAT` until its parser lands while preserving the detected type. The Python boundary converts this error into the frozen serializable failure result.
