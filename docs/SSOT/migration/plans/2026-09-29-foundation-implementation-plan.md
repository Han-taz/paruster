# Rust/Python Foundation Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Freeze kordoc compatibility contracts and deliver an installable Rust/PyO3 Python 3.10+ foundation with bounded format detection, typed errors, a golden parity harness, and mandatory cross-platform CI.

**Architecture:** `kordoc-ir` owns serializable document and error contracts, `kordoc-core` owns byte-oriented detection and security limits, and `kordoc-python` exposes the Rust core as `kordoc._native`. The pure-Python facade normalizes path/bytes/file-like inputs and exposes typed objects without duplicating parsing logic. Checked-in synthetic golden cases are independent of the ignored local TypeScript oracle.

**Tech Stack:** Rust 1.97, Cargo workspace, serde 1.0, serde_json 1.0, thiserror 2.0, zip 8.x with minimal features, cfb 0.15, PyO3 0.29 with `abi3-py310`, maturin 1.15, CPython 3.10-3.14, pytest 8.4, Ruff 0.16, mypy 1.19, GitHub Actions.

---

## Execution and ownership

This is the first independently testable sub-project of the full migration. Execute it through three focused PRs:

1. `contract/freeze-foundation`: Tasks 1-2
2. `feature/rust-python-foundation`: Tasks 3-7 after PR 1 merges
3. `ci/foundation-gates`: Tasks 8-10 after PR 2 merges

The coordinator alone changes `docs/SSOT/contracts/`, compatibility manifests, shared IR names, error codes, Python public names, and MCP schemas. `contracts/*.json` is the machine-readable canonical source; Rust, Python, and prose inventories must be generated from it or checked against it. A SOL manager reviews each PR; Luna workers may edit only the files listed for their task. No worker may edit the ignored `kordoc/` directory.

## File map

- `docs/SSOT/contracts/*.md`: normative human-readable contracts
- `contracts/*.json`: machine-readable compatibility inventories consumed by tests
- `crates/kordoc-ir/`: recursive IR, metadata, result, warning, and error wire types
- `crates/kordoc-core/`: detection, input limits, and parse dispatch boundary
- `crates/kordoc-python/`: PyO3 extension only
- `python/kordoc/`: Python exceptions, models, input normalization, and public facade
- `tests/contracts/`: inventory drift tests
- `tests/golden/`: licensed synthetic fixtures and expected normalized JSON
- `tests/parity/`: deterministic normalization and comparison
- `tests/python/`: installed-package behavior
- `.github/workflows/`: required PR, security, wheel, and release workflows

### Task 1: Freeze machine-readable error and MCP inventories

**Files:**
- Create: `contracts/errors.json`
- Create: `contracts/mcp-tools.json`
- Create: `contracts/mcp-protocol.json`
- Create: `tests/contracts/test_contract_inventory.py`
- Create: `docs/SSOT/contracts/errors.md`
- Create: `docs/SSOT/contracts/mcp.md`
- Create: `.github/pull_request_template.md`
- Modify: `docs/SSOT/README.md`

- [ ] **Step 1: Write the failing inventory tests**

```python
# tests/contracts/test_contract_inventory.py
import json
from pathlib import Path

ROOT = Path(__file__).parents[2]

ERROR_CODES = {
    "EMPTY_INPUT", "UNSUPPORTED_FORMAT", "ENCRYPTED", "DRM_PROTECTED",
    "CORRUPTED", "DECOMPRESSION_BOMB", "ZIP_BOMB", "IMAGE_BASED_PDF",
    "NO_SECTIONS", "PARSE_ERROR", "MISSING_DEPENDENCY",
    "OUTPUT_TOO_LARGE", "FILE_NOT_FOUND",
}

MCP_TOOLS = [
    "parse_document", "detect_format", "parse_metadata", "parse_pages",
    "parse_table", "compare_documents", "parse_chunks", "parse_form",
    "fill_form", "place_seal", "patch_document", "redact_document",
    "render_document", "crop_regions", "extract_tables",
    "extract_profile", "generate_document",
]

def load(name: str) -> object:
    return json.loads((ROOT / "contracts" / name).read_text(encoding="utf-8"))

def test_error_inventory_is_exact() -> None:
    assert set(load("errors.json")["codes"]) == ERROR_CODES

def test_mcp_inventory_is_exact_and_ordered() -> None:
    tools = load("mcp-tools.json")["tools"]
    assert [tool["name"] for tool in tools] == MCP_TOOLS
    assert all(tool["status"] == "planned" for tool in tools)
    assert all(tool["source"].startswith("src/mcp/") for tool in tools)

def test_mcp_protocol_covers_every_tool_and_shared_limit() -> None:
    inventory = load("mcp-tools.json")["tools"]
    protocol = load("mcp-protocol.json")
    assert {tool["name"] for tool in inventory} == set(protocol["input_schemas"])
    assert {tool["name"] for tool in inventory} == set(protocol["output_envelopes"])
    assert protocol["transport"] == "stdio"
    assert protocol["limits"] == {
        "document_bytes": 524_288_000,
        "metadata_bytes": 52_428_800,
        "response_characters": 200_000,
    }
```

- [ ] **Step 2: Run the tests and verify the files are missing**

Run: `uv run --python 3.10 --with pytest==8.4.2 pytest tests/contracts/test_contract_inventory.py -q`

Expected: failures for the three missing contract files.

- [ ] **Step 3: Add exact inventories**

`contracts/errors.json` contains `schema_version: 1` and the 13 codes from `ERROR_CODES` in the test. `contracts/mcp-tools.json` contains the 17 ordered objects below; each object also records `status: "planned"`.

```json
{
  "schema_version": 1,
  "tools": [
    {"name":"parse_document","source":"src/mcp/tools-parse.ts","status":"planned"},
    {"name":"detect_format","source":"src/mcp/tools-parse.ts","status":"planned"},
    {"name":"parse_metadata","source":"src/mcp/tools-parse.ts","status":"planned"},
    {"name":"parse_pages","source":"src/mcp/tools-parse.ts","status":"planned"},
    {"name":"parse_table","source":"src/mcp/tools-parse.ts","status":"planned"},
    {"name":"compare_documents","source":"src/mcp/tools-parse.ts","status":"planned"},
    {"name":"parse_chunks","source":"src/mcp/tools-parse.ts","status":"planned"},
    {"name":"parse_form","source":"src/mcp/tools-form.ts","status":"planned"},
    {"name":"fill_form","source":"src/mcp/tools-form.ts","status":"planned"},
    {"name":"place_seal","source":"src/mcp/tools-form.ts","status":"planned"},
    {"name":"patch_document","source":"src/mcp/tools-form.ts","status":"planned"},
    {"name":"redact_document","source":"src/mcp/tools-form.ts","status":"planned"},
    {"name":"render_document","source":"src/mcp/tools-render.ts","status":"planned"},
    {"name":"crop_regions","source":"src/mcp/tools-render.ts","status":"planned"},
    {"name":"extract_tables","source":"src/mcp/tools-render.ts","status":"planned"},
    {"name":"extract_profile","source":"src/mcp/tools-generate.ts","status":"planned"},
    {"name":"generate_document","source":"src/mcp/tools-generate.ts","status":"planned"}
  ]
}
```

Capture `mcp-protocol.json` from the exact 17 handler registrations and shared helpers in the ignored oracle. For each tool it records the complete JSON input schema (required/optional fields, enums, defaults, numeric/string bounds and descriptions), success/error content envelope, file/path/output behavior, and response truncation behavior. It also records stdio framing/logging rules, root confinement and symlink-safe output rules, extension allowlists, sanitized-error behavior, and the exact 500 MiB/50 MiB/200,000-character shared limits. Record source file SHA-256 values and a deterministic extraction/review command; checked-in tests must validate full tool coverage and schema uniqueness without reading the ignored directory.

Document in `errors.md` that codes are stable protocol values and that unimplemented codes are reserved protocol values, not claims of reachable behavior. Document in `mcp.md` that all captured schemas, envelopes, security limits, and stdio behavior are compatibility requirements while runtime implementation status remains `planned`; names may not be marked implemented merely because the future server lists them. Link both pages from the SSOT index. Add a PR template requiring scope, tests, parity evidence, SSOT impact, WIKI link (allowed to say `pending` while the PR is a draft), security impact, and ownership declaration so every PR in this sequence has the same review evidence.

- [ ] **Step 4: Run the inventory tests**

Run: `uv run --python 3.10 --with pytest==8.4.2 pytest tests/contracts/test_contract_inventory.py -q`

Expected: `3 passed`.

- [ ] **Step 5: Commit**

```bash
git add contracts tests/contracts docs/SSOT/contracts docs/SSOT/README.md .github/pull_request_template.md
git commit -m "docs: freeze error and MCP inventories"
```

### Task 2: Freeze Python translation and parity policy

**Files:**
- Create: `contracts/public-api.json`
- Create: `contracts/oracle-public-exports.json`
- Create: `contracts/ir-schema.json`
- Create: `docs/SSOT/contracts/ir.md`
- Create: `docs/SSOT/contracts/python-api.md`
- Create: `docs/SSOT/contracts/compatibility-manifest.md`
- Create: `docs/SSOT/quality/parity.md`
- Modify: `tests/contracts/test_contract_inventory.py`
- Modify: `docs/SSOT/README.md`
- Create: `docs/WIKI/2026/09/2026-09-29-contract-freeze.md`
- Modify: `docs/WIKI/README.md`

- [ ] **Step 1: Add a failing public API classification test**

```python
def test_every_api_entry_has_a_disposition() -> None:
    public_api = load("public-api.json")
    entries = public_api["entries"]
    type_entries = public_api["type_entries"]
    oracle = load("oracle-public-exports.json")
    assert entries
    assert len({entry["source_name"] for entry in entries}) == len(entries)
    assert {entry["source_name"] for entry in entries} == set(oracle["named_exports"])
    assert {entry["source_name"] for entry in type_entries} == set(oracle["type_exports"])
    assert len({entry["source_name"] for entry in type_entries}) == len(type_entries)
    assert {entry["disposition"] for entry in entries} <= {
        "foundation", "planned", "removed-node-surface", "internal",
    }
    assert {entry["source_name"] for entry in entries} >= {
        "parse", "parseHwpx", "parseHwp", "parseHwp3", "parsePdf",
        "parseXlsx", "parseXls", "parseDocx", "parseHwpml", "parseImage",
        "fillForm", "compare", "markdownToHwpx", "patchHwpx", "patchHwp",
        "validateHwpx", "redactText", "redactMarkdown", "blocksToChunks",
        "renderDocument", "detectFormat", "blocksToMarkdown", "blocksToPages",
    }
```

- [ ] **Step 2: Verify the test fails**

Run: `uv run --python 3.10 --with pytest==8.4.2 pytest tests/contracts/test_contract_inventory.py::test_every_api_entry_has_a_disposition -q`

Expected: failure because `contracts/public-api.json` does not exist.

- [ ] **Step 3: Create the classified manifest and normative pages**

Capture `oracle-public-exports.json` once from the exact named value and type exports in the ignored oracle `src/index.ts`; record its SHA-256, extraction command, named exports, type exports, and source path. Capture `ir-schema.json` from the oracle type declarations with every public wire type, field, optionality, enum value, and protocol casing. These snapshots are inert JSON evidence and must not make tests depend on the ignored directory. Populate `public-api.json` with exactly one classified value entry for every captured named export and one classified `type_entries` record for every captured type export, linked to its Python model, protocol-only representation, or explicit removal disposition. Map the ten format parsing functions to Python snake-case names with `disposition: "planned"`; map `parse` to `parse` with `disposition: "foundation"` and explicitly document that it is an API shell returning unsupported until parsers land; mark the Node CLI and Node package entry machinery `removed-node-surface`; keep `filePath` as `internal` rather than a public Python option. Every remaining library behavior stays `planned`, not removed. The contract test asserts exact value/type export-set equality, unique source and Python names where applicable, allowed dispositions, required mapping fields, and internal consistency of the IR schema.

The normative pages state:

```text
parse(input, options=None) -> ParseResult        # raises KordocError on failure
try_parse(input, options=None) -> TryParseResult # serializable success/failure
input := path-like | bytes | bytearray | memoryview | BinaryIO
```

`ir.md` defines recursive IR as the wire SSOT and requires JSON roundtrip for blocks, nested cell blocks, caption blocks, spans, images, pages, metadata, outline, quality, warnings, and errors. `parity.md` permits normalization only for ZIP timestamps and XML attribute order; it forbids changing oracle answers, benchmark policy, or population to gain score. Link the new contract and quality pages from the SSOT index.

- [ ] **Step 4: Run the complete contract test**

Run: `uv run --python 3.10 --with pytest==8.4.2 pytest tests/contracts -q`

Expected: `4 passed`.

- [ ] **Step 5: Commit, open a draft PR, and append its WIKI evidence**

```bash
git add contracts tests/contracts docs/SSOT
git commit -m "docs: freeze Python and parity contracts"
git push -u origin contract/freeze-foundation
gh pr create --draft --base main --head contract/freeze-foundation --title "docs: freeze foundation compatibility contracts" --body-file .github/pull_request_template.md
```

Create the append-only contract-freeze WIKI entry with the new PR URL, exact oracle digest, tests, and decisions; add it to the WIKI index, push that commit, update the PR body with the WIKI link, and mark the PR ready for review. After squash merge, run `git fetch origin`, fast-forward local `main` with `git merge --ff-only origin/main`, and create `feature/rust-python-foundation` from `origin/main`. Never continue implementation on the merged branch.

Do not begin Task 3 until this PR is reviewed and squash-merged.

### Task 3: Scaffold the locked Cargo and Python workspace

**Files:**
- Create: `Cargo.toml`
- Create: `LICENSE`
- Create: `rust-toolchain.toml`
- Create: `crates/kordoc-ir/Cargo.toml`
- Create: `crates/kordoc-ir/src/lib.rs`
- Create: `crates/kordoc-core/Cargo.toml`
- Create: `crates/kordoc-core/src/lib.rs`
- Create: `crates/kordoc-python/Cargo.toml`
- Create: `crates/kordoc-python/src/lib.rs`
- Create: `pyproject.toml`
- Create: `python/kordoc/__init__.py`
- Create: `python/kordoc/py.typed`
- Create: `tests/python/test_import.py`

- [ ] **Step 1: Write the failing import test**

```python
# tests/python/test_import.py
import kordoc

def test_package_imports_native_version() -> None:
    assert kordoc.__version__ == "0.1.0"
    assert kordoc.native_version() == "0.1.0"
```

- [ ] **Step 2: Verify the package is absent**

Run: `uv run --python 3.10 --with pytest==8.4.2 pytest tests/python/test_import.py -q`

Expected: collection error `ModuleNotFoundError: No module named 'kordoc'`.

- [ ] **Step 3: Create the workspace manifests**

Use resolver 3 and central workspace dependencies:

```toml
# Cargo.toml
[workspace]
members = ["crates/kordoc-ir", "crates/kordoc-core", "crates/kordoc-python"]
resolver = "3"

[workspace.package]
version = "0.1.0"
edition = "2024"
license = "MIT"
rust-version = "1.97"

[workspace.dependencies]
pyo3 = "0.29.2"
serde = { version = "1.0.229", features = ["derive"] }
serde_json = "1.0"
thiserror = "2.0.21"
zip = { version = "8", default-features = false, features = ["deflate"] }
cfb = "0.15.0"
proptest = "1.11.0"
```

`rust-toolchain.toml` pins `1.97.0` with `rustfmt` and `clippy`; this is also the declared and tested Rust minimum. Each crate inherits workspace package fields. `kordoc-core` uses workspace `proptest` only as a dev-dependency. `kordoc-python` uses `cdylib` and `rlib`, depends on both internal crates, enables `abi3-py310` on its PyO3 dependency, and declares a crate feature `extension-module = ["pyo3/extension-module"]` with no default. `[tool.maturin]` selects that feature for extension builds; ordinary `cargo test --workspace` does not, preventing macOS libpython link failures.

`LICENSE` contains the MIT license for the project. `pyproject.toml` uses `maturin==1.15.0`, declares `requires-python = ">=3.10"`, module name `kordoc._native`, Python source `python`, and `license-files = ["LICENSE"]`. Define `[dependency-groups].dev` with exact versions `maturin==1.15.0`, `pytest==8.4.2`, `ruff==0.16.9`, and `mypy==1.19.1` so every documented `uv sync --all-groups` and `uv run` command is reproducible.

- [ ] **Step 4: Add the minimal native version function and facade**

```rust
// crates/kordoc-python/src/lib.rs
use pyo3::prelude::*;

#[pyfunction]
fn native_version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

#[pymodule]
fn _native(module: &Bound<'_, PyModule>) -> PyResult<()> {
    module.add_function(wrap_pyfunction!(native_version, module)?)?;
    Ok(())
}
```

```python
# python/kordoc/__init__.py
from ._native import native_version

__version__ = "0.1.0"
__all__ = ["__version__", "native_version"]
```

- [ ] **Step 5: Build and run the import test**

Run: `uv sync --python 3.10 --all-groups && uv run maturin develop`

Expected: development wheel installs successfully.

Run: `uv run pytest tests/python/test_import.py -q`

Expected: `1 passed`.

- [ ] **Step 6: Generate and commit lockfiles**

Run: `cargo generate-lockfile && uv lock`

Expected: `Cargo.lock` and `uv.lock` exist and `cargo metadata --locked --no-deps` exits 0.

- [ ] **Step 7: Commit**

```bash
git add Cargo.toml Cargo.lock rust-toolchain.toml LICENSE crates pyproject.toml uv.lock python tests/python
git commit -m "build: scaffold Rust and Python workspace"
```

### Task 4: Implement typed IR and stable errors with serde

**Files:**
- Create: `crates/kordoc-ir/src/document.rs`
- Create: `crates/kordoc-ir/src/error.rs`
- Modify: `crates/kordoc-ir/src/lib.rs`
- Create: `crates/kordoc-ir/tests/ir_contract.rs`

- [ ] **Step 1: Write failing recursive roundtrip and error-code tests**

```rust
// crates/kordoc-ir/tests/ir_contract.rs
use kordoc_ir::{ErrorCode, ImageData, IrBlock, IrBlockType, IrCell, IrSpan, IrTable};

#[test]
fn recursive_ir_roundtrips_without_loss() {
    let block = IrBlock {
        kind: IrBlockType::Table,
        table: Some(IrTable {
            rows: 1,
            cols: 1,
            has_header: false,
            cells: vec![vec![IrCell {
                text: "값".into(),
                col_span: 1,
                row_span: 1,
                blocks: Some(vec![IrBlock::paragraph("중첩")]),
                is_header: None,
            }]],
            caption: Some("표 1".into()),
            caption_blocks: Some(vec![IrBlock::paragraph("캡션")]),
        }),
        spans: Some(vec![IrSpan { text: "값".into(), bold: Some(true), ..Default::default() }]),
        image_data: Some(ImageData { data: vec![0, 1, 2], mime_type: "image/png".into(), filename: None }),
        ..IrBlock::default()
    };
    let json = serde_json::to_string(&block).unwrap();
    assert_eq!(serde_json::from_str::<IrBlock>(&json).unwrap(), block);
}

#[test]
fn error_codes_serialize_to_protocol_names() {
    assert_eq!(serde_json::to_string(&ErrorCode::EmptyInput).unwrap(), "\"EMPTY_INPUT\"");
    assert_eq!(ErrorCode::ALL.len(), 13);
}
```

- [ ] **Step 2: Run the tests and verify compilation fails**

Run: `cargo test -p kordoc-ir --test ir_contract`

Expected: unresolved imports for the not-yet-defined IR and error types.

- [ ] **Step 3: Implement the complete foundation wire types**

In `document.rs`, derive `Debug`, `Clone`, `PartialEq`, `Serialize`, and `Deserialize`; use `#[serde(rename_all = "camelCase")]` for fields and explicit enum renames for protocol strings. Implement `IrBlockType`, `IrSpan`, `ImageData`, `BoundingBox`, `InlineStyle`, `IrBlock`, `IrTable`, `IrCell`, `DocumentMetadata`, `ParseWarning`, `OutlineItem`, `PageMarkdown`, `ExtractedImage`, `PageQuality`, and `DocumentQualitySummary`. Optional fields use `#[serde(skip_serializing_if = "Option::is_none")]`; byte arrays remain JSON integer arrays in the foundation wire format.

`IrBlock::paragraph` sets `kind`, `text`, and defaults all other fields. `Default` leaves every optional field absent. The type inventory is checked against `contracts/ir-schema.json` captured in Task 2 and includes existing fields such as block quote/indent/list depth/link/footnote data and table classification/render/source/region data; fields may not be silently omitted from the foundation schema. Add exact wire-key, enum-string, optional omission, success/failure result, and warning-code tests in addition to the roundtrip. `error.rs` defines the exact 13-code `ErrorCode` enum and `ALL` array, plus `KordocError { code, message }` using `thiserror`; tests compare every serialized code against canonical `contracts/errors.json`, not only its count.

- [ ] **Step 4: Run crate tests and rustfmt**

Run: `cargo fmt --all -- --check && cargo test -p kordoc-ir --locked`

Expected: all `kordoc-ir` tests pass.

- [ ] **Step 5: Commit**

```bash
git add crates/kordoc-ir
git commit -m "feat(ir): add serializable document and error contracts"
```

### Task 5: Implement bounded format detection and failure dispatch

**Files:**
- Create: `crates/kordoc-core/src/detect.rs`
- Create: `crates/kordoc-core/src/limits.rs`
- Modify: `crates/kordoc-core/src/lib.rs`
- Create: `crates/kordoc-core/tests/detect_contract.rs`
- Create: `crates/kordoc-core/tests/detect_properties.rs`

- [ ] **Step 1: Write failing detection tests**

```rust
// crates/kordoc-core/tests/detect_contract.rs
use kordoc_core::{detect_format, try_parse, FileType};
use kordoc_ir::ErrorCode;

#[test]
fn detects_magic_bytes() {
    assert_eq!(detect_format(b"HWP Document File V3.00\x1a\x01").unwrap(), FileType::Hwp3);
    assert_eq!(detect_format(b"%PDF-1.7\n").unwrap(), FileType::Pdf);
    assert_eq!(detect_format(b"\x89PNG\r\n\x1a\n").unwrap(), FileType::Image);
    assert_eq!(detect_format(b"\xff\xd8\xff\xe0").unwrap(), FileType::Image);
    assert_eq!(detect_format(b"RIFF\0\0\0\0WEBP").unwrap(), FileType::Image);
}

#[test]
fn empty_and_unknown_inputs_return_stable_errors() {
    assert_eq!(try_parse(&[]).unwrap_err().code, ErrorCode::EmptyInput);
    assert_eq!(try_parse(b"not a document").unwrap_err().code, ErrorCode::UnsupportedFormat);
}
```

- [ ] **Step 2: Verify the test fails to compile**

Run: `cargo test -p kordoc-core --test detect_contract`

Expected: unresolved imports for detection and dispatch functions.

- [ ] **Step 3: Write container-security and property tests before implementation**

Create synthetic in-memory archives in the tests. Assert a PPTX container will detect as `Pptx`, while empty and unrelated ZIPs and a generic OLE container will detect as `Unknown`. Specify the preflight validator through tests with synthetic central-directory metadata at the exact boundary and one beyond it, then add compact malformed ZIP cases whose forged entry count, directory span, or declared size must return `ZIP_BOMB` without allocating the declared payload. Test the input-length validator directly at `MAX_INPUT_BYTES` and `MAX_INPUT_BYTES + 1` so the suite requires `OUTPUT_TOO_LARGE` behavior without allocating 500 MiB. The SSOT explicitly defines this historically named code as covering bounded input or output payloads; decompression limits retain `DECOMPRESSION_BOMB`/`ZIP_BOMB`.

Add property tests using the workspace `proptest` dependency: arbitrary bounded byte slices never panic; detection is deterministic; any successful central-directory preflight has entry count, span, and uncompressed total within declared limits. Run both test binaries and verify they fail because the container preflight, limits, and refinement behavior do not exist. Preserve every property-test failure seed as a fixed regression case before implementing.

- [ ] **Step 4: Implement magic detection and bounded container refinement**

`detect.rs` implements `FileType` with protocol strings `hwpx`, `hwp`, `hwp3`, `hwpml`, `pdf`, `xlsx`, `xls`, `docx`, `pptx`, `image`, and `unknown`. `detect_format` returns `Result<FileType, KordocError>` so security-limit failures remain distinguishable from unknown content. Reject inputs over `MAX_INPUT_BYTES = 524_288_000` before container parsing. Detect HWP3, ZIP, OLE2, PDF, HWPML XML, PNG, JPEG, and WebP in that order.

For ZIP, first preflight the EOCD and central-directory records with checked integer arithmetic and no per-entry allocation. Reject more than `MAX_ARCHIVE_ENTRIES = 100_000`, a central directory outside the input, malformed name/extra/comment lengths, or declared uncompressed totals over `MAX_UNCOMPRESSED_BYTES = 1_073_741_824` before constructing `ZipArchive`. After preflight, inspect names only: `xl/workbook.xml` is XLSX, `word/document.xml` is DOCX, `ppt/presentation.xml` is PPTX, and `Contents/content.hpf`, `mimetype`, or a `Contents/` section is HWPX. An unrelated or empty ZIP is `Unknown`, never HWPX. OLE refinement uses `cfb` only after the input cap, recognizes `Workbook` or `Book` streams as XLS, recognizes `FileHeader`, `DocInfo`, or body `Section` streams as HWP, and returns unknown for unrelated OLE containers. All container readers operate on `Cursor<&[u8]>` and never extract members during detection.

`try_parse` validates input and propagates detection security errors, then returns `UNSUPPORTED_FORMAT` for every recognized but not-yet-implemented parser and for unknown input; empty input returns `EMPTY_INPUT`. PPTX is detected but always unsupported. This avoids falsely advertising parser completion. The Python `detect_format` contract intentionally returns the refined container type, unlike the oracle's coarse synchronous `detectFormat` plus separate refiners; record this as an approved surface translation in `compatibility-manifest.md` and test both the refined values and unknown generic containers.

- [ ] **Step 5: Run focused and workspace tests**

Run: `cargo test -p kordoc-core --locked && cargo test --workspace --locked`

Expected: all tests pass.

- [ ] **Step 6: Commit**

```bash
git add crates/kordoc-core
git commit -m "feat(core): add bounded document format detection"
```

### Task 6: Expose Python models, input normalization, and typed failures

**Files:**
- Modify: `crates/kordoc-python/src/lib.rs`
- Create: `python/kordoc/_api.py`
- Create: `python/kordoc/_errors.py`
- Create: `python/kordoc/_models.py`
- Modify: `python/kordoc/__init__.py`
- Create: `tests/python/test_parse_inputs.py`
- Create: `tests/python/test_error_mapping.py`

- [ ] **Step 1: Write failing Python contract tests**

```python
# tests/python/test_parse_inputs.py
from io import BytesIO
import kordoc

def test_detect_format_accepts_all_memory_inputs() -> None:
    pdf = b"%PDF-1.7\n"
    assert kordoc.detect_format(pdf) == "pdf"
    assert kordoc.detect_format(bytearray(pdf)) == "pdf"
    assert kordoc.detect_format(memoryview(pdf)) == "pdf"
    assert kordoc.detect_format(BytesIO(pdf)) == "pdf"

def test_try_parse_is_serializable() -> None:
    result = kordoc.try_parse(b"")
    assert result.to_dict() == {
        "success": False,
        "fileType": "unknown",
        "error": "빈 버퍼이거나 유효하지 않은 입력입니다.",
        "code": "EMPTY_INPUT",
    }
```

```python
# tests/python/test_error_mapping.py
import pytest
import kordoc

def test_parse_raises_typed_error() -> None:
    with pytest.raises(kordoc.EmptyInputError) as caught:
        kordoc.parse(b"")
    assert caught.value.code == "EMPTY_INPUT"
```

- [ ] **Step 2: Build and verify the tests fail**

Run: `uv sync --python 3.10 --all-groups && uv run maturin develop && uv run pytest tests/python/test_parse_inputs.py tests/python/test_error_mapping.py -q`

Expected: failures because the public functions and exception class are absent.

- [ ] **Step 3: Expose native byte-oriented functions**

PyO3 exposes `detect_format_bytes(data: &[u8]) -> PyResult<&'static str>` and `try_parse_bytes(py, data)`; wrap CPU work in `py.detach`. Security-limit failures from detection become the same stable failure dict returned by `try_parse_bytes`. The direct detector raises `ValueError(code, message)` with exactly two string arguments; the Python facade catches only that shape, maps `args[0]` through the stable exception registry, and preserves `args[1]` as the message. Tests assert both native argument shape and public typed translation. Return a Python dict for the temporary native boundary, preserving camel-case protocol keys. Do not accept paths or file-like objects in Rust.

- [ ] **Step 4: Implement the Python facade**

`_api.py` normalizes `str | os.PathLike | bytes | bytearray | memoryview | BinaryIO` to immutable bytes. Memory inputs are length-checked before copying; paths are `stat`-checked and then read through a bounded reader; file-like objects are read with `MAX_INPUT_BYTES + 1` when supported and rejected if they return text or exceed the cap. It delegates only bounded bytes to `_native`. `_models.py` defines frozen, slotted `TryParseResult` with `to_dict` and the complete success/failure field contract documented in SSOT. `_errors.py` defines `KordocError` and subclasses for every stable error code; `parse` converts a failure result to the mapped exception while `try_parse` never raises for document failures. Tests cover a recognized PDF returning `UNSUPPORTED_FORMAT`, oversize memory/path/file-like inputs, and both success-independent failure shapes.

- [ ] **Step 5: Run Python tests, typing, and lint**

Run: `uv run pytest tests/python -q`

Expected: all Python tests pass.

Run: `uv run ruff check python tests && uv run ruff format --check python tests && uv run mypy python/kordoc`

Expected: all commands exit 0.

- [ ] **Step 6: Commit**

```bash
git add crates/kordoc-python python tests/python
git commit -m "feat(python): add typed foundation API"
```

### Task 7: Add a deterministic foundation golden harness

**Files:**
- Create: `tests/golden/README.md`
- Create: `tests/golden/manifest.json`
- Create: `tests/golden/fixtures/minimal.pdf`
- Create: `tests/golden/expected/minimal-pdf.json`
- Create: `tests/__init__.py`
- Create: `tests/parity/__init__.py`
- Create: `tests/parity/normalize.py`
- Create: `tests/parity/compare.py`
- Create: `tests/parity/test_golden.py`

- [ ] **Step 1: Write the failing golden test**

```python
# tests/parity/test_golden.py
import json
from pathlib import Path
import kordoc
from tests.parity.compare import compare_json
from tests.parity.normalize import normalize

ROOT = Path(__file__).parents[1]

def test_committed_golden_cases() -> None:
    manifest = json.loads((ROOT / "golden/manifest.json").read_text())
    for case in manifest["cases"]:
        raw = (ROOT / "golden" / case["input"]).read_bytes()
        actual = {"fileType": kordoc.detect_format(raw)}
        expected = json.loads((ROOT / "golden" / case["expected"]).read_text())
        assert compare_json(normalize(actual), normalize(expected)) is None
```

- [ ] **Step 2: Verify the manifest is missing**

Run: `uv run pytest tests/parity/test_golden.py -q`

Expected: failure for missing `tests/golden/manifest.json`.

- [ ] **Step 3: Add a licensed synthetic PDF and expected result**

Generate `minimal.pdf` from literal bytes in a one-time test fixture generator, then commit the resulting small PDF. `manifest.json` records `id`, input, expected, license `CC0-1.0`, generator, SHA-256, and covered contract. Expected JSON is `{"fileType":"pdf"}`.

`README.md` states that no government corpus or ignored-oracle file may be copied into this directory without recorded redistribution rights and checksum.

- [ ] **Step 4: Implement reusable normalization and comparison**

`normalize.py` recursively sorts object keys, strips ZIP timestamp fields only when their JSON pointer is registered, and canonicalizes XML attribute order only for explicitly tagged XML values. It must not normalize text, block order, table dimensions, page numbers, warnings, errors, scores, or whitespace. `compare.py` reports the first JSON pointer with unequal expected and actual values. Add direct tests for permitted normalization and for every forbidden semantic change. This foundation harness is detector-level golden evidence, not parser parity; `docs/SSOT/migration/status.md` must keep full oracle parity pending until real parser outputs run through it.

- [ ] **Step 5: Run the complete local gate**

Run: `cargo test --workspace --locked && uv run pytest -q && uv run ruff check python tests && uv run mypy python/kordoc`

Expected: all commands exit 0.

- [ ] **Step 6: Commit, open a draft foundation PR, and append WIKI evidence**

```bash
git add tests/golden tests/parity
git commit -m "test: add synthetic foundation golden harness"
git push -u origin feature/rust-python-foundation
gh pr create --draft --base main --head feature/rust-python-foundation --title "feat: establish Rust and Python foundation" --body-file .github/pull_request_template.md
```

Open this PR as a draft, create `docs/WIKI/2026/09/2026-09-29-rust-python-foundation.md` with its URL, exact verification output, artifact checksum, and implemented/pending boundary, index and push the log, update the PR body, then mark it ready. After squash merge, fetch and fast-forward local `main`, then create `ci/foundation-gates` from `origin/main`.

### Task 8: Add required lint, test, documentation, and artifact checks

**Files:**
- Create: `.github/workflows/ci.yml`
- Create: `.github/workflows/wheels.yml`
- Create: `.github/workflows/fuzz.yml`
- Create: `fuzz/Cargo.toml`
- Create: `fuzz/fuzz_targets/detect_format.rs`
- Create: `fuzz/fuzz_targets/zip_preflight.rs`
- Create: `scripts/check_artifacts.py`
- Create: `scripts/check_docs.py`
- Create: `tests/contracts/test_forbidden_paths.py`
- Create: `tests/contracts/test_artifacts.py`
- Create: `tests/contracts/test_docs.py`
- Modify: `.github/pull_request_template.md`

- [ ] **Step 1: Write the forbidden-content test**

```python
# tests/contracts/test_forbidden_paths.py
import subprocess
from pathlib import Path

ROOT = Path(__file__).parents[2]

def test_kordoc_oracle_is_not_tracked() -> None:
    tracked = subprocess.check_output(["git", "ls-files", "-z"], cwd=ROOT)
    assert not any(path.startswith(b"kordoc/") for path in tracked.split(b"\0"))
```

- [ ] **Step 2: Implement the artifact scanner**

First write `test_artifacts.py` with temporary valid wheel and sdist archives plus one case for every forbidden member class. Verify the tests fail because `scripts.check_artifacts` is absent. Then implement `scripts/check_artifacts.py`: it accepts wheel or sdist paths, opens ZIP/tar safely, rejects absolute paths and `..` traversal, and exits nonzero if any normalized member contains `node_modules`, ends in `.ts`, or matches the ignored oracle layout `(^|/)kordoc/src/`, `(^|/)kordoc/package.json`, or `(^|/)kordoc/package-lock.json`. It must allow the real Python package members under `python/kordoc/` and wheel members under `kordoc/`. For a wheel it also requires `.dist-info/METADATA`, a packaged license, and the native extension; for an sdist it requires `LICENSE` and `pyproject.toml`. The tests must pass after implementation.

- [ ] **Step 3: Implement the documentation checker test-first**

Write `test_docs.py` against a temporary Markdown tree with one valid relative link, one missing relative link, and one external link. Verify it fails because `scripts.check_docs` is absent. Implement `scripts/check_docs.py` to scan tracked Markdown, ignore fenced code blocks and external/anchor-only URLs, resolve relative links safely, and fail on missing local targets. It also validates that every non-template WIKI entry appears once in `docs/WIKI/README.md` and every current SSOT page appears in the nearest SSOT index. Run the focused tests until green.

- [ ] **Step 4: Create the PR CI workflow**

`ci.yml` triggers on `pull_request`, pushes to `main`, and `merge_group`; sets top-level `contents: read`; pins every action to a full commit SHA; cancels superseded runs. It ends with one stable, always-reported `ci-gate` aggregate job that depends on all required jobs and fails if any dependency failed or was unexpectedly skipped. The ruleset requires only this aggregate name rather than matrix-generated names. Required jobs are:

```text
contracts: Python 3.10, contract and forbidden-path tests
rust: fmt, clippy -D warnings, workspace tests, rustdoc -D warnings
python: ubuntu-24.04 matrix 3.10, 3.11, 3.12, 3.13, 3.14; maturin develop then pytest/Ruff/mypy
golden: installed wheel against committed golden cases
docs: `scripts/check_docs.py` link and index drift checks
coverage: `cargo-llvm-cov` for `kordoc-ir` and `kordoc-core`, line coverage >= 80%
```

Keep the existing PR template synchronized with the required scope, tests, parity evidence, SSOT impact, WIKI link, security impact, and ownership declaration fields; do not replace it with a different checklist.

- [ ] **Step 5: Add bounded fuzz campaigns**

Create two `cargo-fuzz 0.13.2` targets: arbitrary bytes through `detect_format` and arbitrary central-directory bytes through the allocation-free ZIP preflight exposed only under a non-default `fuzzing` crate feature. Seed both with checked-in synthetic fixtures and assert only safety invariants—no panic, abort, out-of-bounds read, or uncontrolled allocation. `fuzz.yml` pins `nightly-2026-09-20` and the cargo-fuzz version, runs each target for 30 seconds on PR/push/merge-group events, runs each for 15 minutes weekly, uploads crash artifacts, and ends with an always-reported `fuzz-gate` on required triggers. Any crash input becomes a deterministic regression test before the fix is merged. Coverage uses pinned `cargo-llvm-cov 0.9.1`, uploads HTML/LCOV artifacts, and enforces 80% line coverage for the two foundation logic crates; generated PyO3 glue is excluded and the threshold may only change in a reviewed policy PR.

- [ ] **Step 6: Create native wheel smoke workflow**

`wheels.yml` triggers on `pull_request`, pushes to `main`, `merge_group`, and `workflow_dispatch`, and uses `PyO3/maturin-action` pinned to a full SHA. It ends with a stable `wheels-gate` aggregate job that is always reported for those triggers. Matrix runners and targets are:

```text
ubuntu-24.04       x86_64-unknown-linux-gnu
ubuntu-24.04-arm   aarch64-unknown-linux-gnu
windows-2025       x86_64-pc-windows-msvc
windows-11-arm     aarch64-pc-windows-msvc
macos-15-intel     x86_64-apple-darwin
macos-15           aarch64-apple-darwin
```

Each job sets up explicit supported CPython interpreters on the host, builds one `cp310-abi3` wheel, audits the wheel tag, creates clean virtual environments, installs that exact wheel, runs import/detection/error/golden smoke, invokes `scripts/check_artifacts.py`, and uploads the wheel with the source commit SHA in artifact metadata. Linux explicitly uses a supported manylinux policy and treats the maturin build container separately from the host interpreter used for install testing. The full 3.10-3.14 behavior matrix runs on Linux x64; every target runner tests the oldest and newest interpreter natively available there, with 3.14 mandatory and 3.10 mandatory wherever an official runner interpreter exists. The `cp310-abi3` audit plus the full Linux minor matrix is the recorded evidence for intermediate target/version combinations that hosted runners cannot supply (notably older Windows ARM interpreters). Release notes must disclose any untested cross-product cell. No job publishes.

- [ ] **Step 7: Validate locally**

Run: `uv run pytest tests/contracts tests/parity tests/python -q && uv run python scripts/check_docs.py && cargo llvm-cov -p kordoc-ir -p kordoc-core --fail-under-lines 80 && cargo +nightly-2026-09-20 fuzz run detect_format -- -max_total_time=30 && cargo +nightly-2026-09-20 fuzz run zip_preflight -- -max_total_time=30 && uv run maturin build --release && uv run python scripts/check_artifacts.py target/wheels/*.whl`

Expected: tests pass and the locally built wheel passes the scanner.

- [ ] **Step 8: Commit and open the draft CI PR**

```bash
git add .github scripts tests/contracts
git commit -m "ci: add foundation quality and wheel gates"
git push -u origin ci/foundation-gates
gh pr create --draft --base main --head ci/foundation-gates --title "ci: enforce foundation quality gates" --body-file .github/pull_request_template.md
```

### Task 9: Add security and release-candidate workflows

**Files:**
- Create: `.github/workflows/security.yml`
- Create: `.github/workflows/release.yml`
- Create: `deny.toml`
- Create: `docs/SSOT/operations/development.md`
- Create: `docs/SSOT/operations/release.md`

- [ ] **Step 1: Add dependency and license policy**

`deny.toml` denies known advisories, unknown registries/git sources, duplicate wildcard dependencies, and licenses outside the reviewed allowlist `MIT`, `Apache-2.0`, `BSD-2-Clause`, `BSD-3-Clause`, `ISC`, `Unicode-3.0`, and `Zlib`. Any additional native-engine license requires its own reviewed PR.

- [ ] **Step 2: Add scheduled security workflow**

`security.yml` runs CodeQL for Rust, Python, and Actions; `cargo audit --locked`; `cargo deny check`; dependency review on PRs; actionlint; and zizmor. It runs weekly and on `pull_request`, pushes to `main`, and `merge_group`, with read-only permissions except the documented CodeQL `security-events: write` job. Every CLI tool is installed through a full-SHA-pinned action or verified release artifact at a version recorded in the workflow; floating `cargo install` and unverified curl pipes are forbidden. A stable `security-gate` aggregate job is always reported on required triggers. Before making CodeQL or dependency review required, verify repository visibility and GitHub Code Security entitlement; if unavailable, keep the local open-source scanners required and record the hosted-feature limitation in SSOT rather than leaving an impossible required check.

- [ ] **Step 3: Add a non-publishing release candidate workflow**

`release.yml` initially triggers only via `workflow_dispatch` with `publish` fixed to false. To avoid trusting or ambiguously selecting artifacts from another workflow run, it rebuilds the six wheels and sdist from the checked-out requested commit inside the release workflow, then scans and installs every artifact. A dedicated manifest job uses `anchore/sbom-action` (Syft, pinned to a reviewed full commit SHA and recorded Syft version) to emit both SPDX JSON and CycloneDX JSON for each wheel and sdist, and generates a SHA-256 manifest covering artifacts and SBOMs. A separate attestation job uses `actions/attest-build-provenance` pinned to a reviewed full commit SHA; only that job receives `id-token: write` and `attestations: write`, with `contents: read`, while every build/test job remains read-only. It attests each immutable artifact digest after all smoke and scanner jobs pass. Workflow tests assert the elevated permissions are job-scoped and that every checksum has both SBOM formats and an attestation step. The publish job is absent until the final release plan explicitly adds the protected `pypi` environment and Trusted Publishing; a false guard is not treated as a security boundary.

- [ ] **Step 4: Document reproducible development and release commands**

`development.md` uses `uv python install 3.10`, `uv sync --all-groups`, `maturin develop`, Cargo locked commands, and the full local gate. `release.md` states that no package may publish while any compatibility entry is `planned`, while MCP runtime tools are incomplete, or before two consecutive release-candidate workflows pass.

- [ ] **Step 5: Run workflow and policy linters**

Run pinned, checksum-verified local installations of: `actionlint`, `zizmor`, `cargo-deny`, and `cargo-audit`, then execute `actionlint && zizmor .github/workflows && cargo deny check && cargo audit --locked`.

Expected: every command exits 0.

- [ ] **Step 6: Commit**

```bash
git add .github/workflows/security.yml .github/workflows/release.yml deny.toml docs/SSOT/operations
git commit -m "ci: add security and release candidate policy"
```

### Task 10: Verify, document, and merge the foundation gates

**Files:**
- Create: `docs/WIKI/2026/09/2026-09-29-foundation-execution.md`
- Create: `docs/SSOT/architecture/workspace.md`
- Create: `docs/SSOT/migration/status.md`
- Modify: `docs/SSOT/README.md`
- Modify: `docs/WIKI/README.md`

- [ ] **Step 1: Run the complete verification suite from a clean checkout**

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features --locked -- -D warnings
cargo test --workspace --locked
RUSTDOCFLAGS=-Dwarnings cargo doc --workspace --no-deps --locked
cargo llvm-cov -p kordoc-ir -p kordoc-core --fail-under-lines 80
cargo +nightly-2026-09-20 fuzz run detect_format -- -max_total_time=30
cargo +nightly-2026-09-20 fuzz run zip_preflight -- -max_total_time=30
uv run ruff check python tests scripts
uv run ruff format --check python tests scripts
uv run mypy python/kordoc scripts
uv run pytest -q
uv run maturin build --release
uv run python scripts/check_artifacts.py target/wheels/*.whl
actionlint
zizmor .github/workflows
cargo audit --locked
cargo deny check
```

Expected: every command exits 0. Record exact versions, test counts, wheel filename, and checksums in the WIKI entry.

- [ ] **Step 2: Update current-state SSOT**

`workspace.md` documents actual crate ownership and data flow. `status.md` marks only implemented and verified foundation capabilities complete; every parser, transformation, renderer, OCR operation, and MCP handler remains pending. Link both from the SSOT index.

- [ ] **Step 3: Append the execution log**

Create the WIKI entry from `docs/WIKI/TEMPLATE.md`, linking the two merged PRs, the current draft PR, branch commits, completed CI runs, verification commands, and evidence. Do not claim the current PR's future merge commit and do not edit earlier WIKI entries.

- [ ] **Step 4: Push documentation, collect CI evidence, and ready the CI PR**

```bash
git add docs
git commit -m "docs: record verified foundation state"
git push
```

Wait for the code-bearing commit's workflow runs, append their run URLs and conclusions to the still-unmerged WIKI entry, push that evidence-only commit, update the PR body with the WIKI link, and mark the draft ready. The final documentation-triggered checks must also pass, but their URLs may live in the PR check record because an immutable log cannot contain evidence created after its final content.

- [ ] **Step 5: Enable required checks only after GitHub reports them**

Use the repository ruleset API to require the exact successful aggregate check names `ci-gate`, `wheels-gate`, `fuzz-gate`, and `security-gate` only after GitHub reports them on both PR and `merge_group` events. Enable the repository merge queue with squash-only integration and require queued branches to run those aggregate checks against the current base. Keep PR requirement, linear history, deletion protection, and non-fast-forward protection. Do not guess check names before their first run, and do not require hosted security checks unavailable to the repository. Verify queue operation with a disposable documentation PR before relying on it for subsequent parser branches.

- [ ] **Step 6: Review and squash merge**

The SOL manager verifies file ownership, contract adherence, and parity evidence. The coordinator independently reads the diff and reruns the local gate, then squash-merges only with all required GitHub checks green.

## Plan completion boundary

This plan completes only the shared foundation. It does not claim any document format parser, transformation, renderer, OCR pipeline, generator, or MCP operation is implemented. The next plans are authored from the verified contracts in this order: Hancom readers, Office readers, PDF/image/OCR, shared transformations, format-preserving mutation and generation, rendering, MCP handlers, full packaging/release, and Node removal.
