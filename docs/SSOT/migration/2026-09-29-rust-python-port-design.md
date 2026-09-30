# Rust and Python full-port design

Status: Approved

Date: 2026-09-29

Owner: project coordinator

## 1. Objective

Replace kordoc's TypeScript and Node.js product implementation with a Rust core exposed through a Python library while preserving the existing 17-tool MCP server. Support CPython 3.10 through every current stable release on Windows x64/ARM64, macOS Intel/Apple Silicon, and Linux x64/ARM64.

The local `kordoc/` checkout remains a read-only behavioral oracle during migration. It is excluded from Git and all deliverables. Node.js and npm are removed after the final parity gate.

## 2. Product scope

The port includes:

- HWP 3.x, HWP 5.x, HWPX, HWPML, PDF, XLS, XLSX, DOCX, and image parsing
- normalized document IR, Markdown, metadata, outlines, pages, images, warnings, and quality signals
- table recovery and classification
- OCR and formula recognition
- document comparison, form recognition and filling, patching, roundtrip editing, and redaction
- HWPX and Korean government-document generation
- layout-preserving render, PDF/SVG/raster output, and print support
- Python object API and convenience functions
- the existing 17 MCP tools and their security/error behavior

Removed product surfaces are the Node library API and general Node CLI. `kordoc-mcp` remains as a Python package entry point.

## 3. External-engine boundary

Rust owns product policy, IR, transformation, generation, security, and orchestration. Proven engines may be embedded through Rust where reimplementing them would reduce correctness or delay the port materially. The September 30 user decision authorizes embedded V8 with PDF.js for PDF semantic extraction; it supersedes the earlier pure-Rust PDF backend requirement:

- V8 with pinned, bundled PDF.js main/worker modules for PDF parsing and text/metadata extraction; no Node.js runtime, filesystem, network, or document-script host APIs
- PDFium for PDF rendering
- ONNX Runtime for OCR and model inference
- platform codecs or similarly bounded native libraries when required

Required runtime libraries and licensed PDF.js assets are bundled in platform wheels. PDF.js runs behind Rust-owned input, execution, allocation, and output limits. A supervised native worker process and six-target installed-wheel evidence are required before production PDF registration; a private in-process feasibility probe does not satisfy those gates. Large versioned model weights are distributed as a verified model bundle rather than duplicated across every base wheel. Offline installation accepts a pre-fetched verified bundle and never requires Node.js.

## 4. Architecture

The initial Cargo workspace has these responsibility boundaries:

- `kordoc-ir`: immutable public IR, metadata, warnings, error codes, serialization
- `kordoc-core`: format detection, dispatch, shared limits, Markdown and table normalization
- format crates: independently owned adapters for Hancom, PDF/image, and Office formats
- transformation crates: diff, forms, roundtrip, redaction, generation, rendering
- `kordoc-python`: thin PyO3 extension named `kordoc._native`
- Python package: object model, ergonomic wrappers, async integration, MCP protocol adapter

Shared contracts are frozen before parser implementation starts. Format workers do not edit shared contracts without coordinator approval.

```text
path | bytes | binary file-like
              |
       Python normalization
              |
       PyO3 boundary (GIL released)
              |
    Rust detect -> parse -> immutable IR
              |
 transform | generate | render | OCR
              |
 Python Document / ParseResult / MCP
```

## 5. Python contract

`pyproject.toml` declares `requires-python = ">=3.10"` with no upper bound. The release matrix tests every stable CPython minor from 3.10 through the latest stable minor. At design time this is 3.10-3.14; 3.15 joins when stable. Python 3.10 remains a project target after upstream EOL.

Use `abi3-py310` where dependencies and native engines permit. Standard CPython builds are the guaranteed target; PyPy and free-threaded CPython are separate compatibility tracks.

Primary API:

```python
import kordoc

result = kordoc.parse("document.hwp")
document = result.document
print(document.markdown)

generated = kordoc.generate_hwpx(markdown, preset="official")
generated.write("output.hwpx")
```

- `parse()` accepts paths, bytes, and binary file-like objects.
- Success returns typed Python objects; failure raises a typed `KordocError` subclass.
- `try_parse()` returns a serializable success/failure result for protocol and batch consumers.
- `Document` owns blocks, Markdown, pages, images, metadata, outline, warnings, and quality data.
- IR objects are immutable at the Python boundary. Editing uses explicit editors or transformations.
- CPU-bound PyO3 calls release the GIL.
- Stable error codes accompany Python exception classes.

## 6. Error model

The minimum stable hierarchy includes unsupported format, invalid document, password required, missing dependency, security limit, and OCR failures. Error messages may improve, but codes and structured fields are compatibility contracts. Internal paths, secrets, and native stack details are sanitized before crossing Python or MCP boundaries.

## 7. MCP contract

The MCP server is a thin Python adapter over the same Rust operations used by the Python API. It retains all 17 tool names, input schemas, output envelopes, error codes, file-access restrictions, response-size limits, and stdio behavior. No parsing or transformation logic is independently reimplemented in the MCP layer.

## 8. Parallel engineering model

The coordinator owns cross-cutting contracts, priorities, integration gates, final review, and merge. A `gpt-6-sol` manager owns each active functional area, decomposes it, reviews results, and reports integration readiness. `gpt-6-luna` workers implement and independently verify bounded tasks.

With four concurrent slots, the default managed wave is one coordinator, one SOL manager, and two Luna workers. For simple independent work, the coordinator may dispatch three Luna workers directly to avoid management overhead. Orca orchestration tracks the dependency DAG, questions, evidence, and completion. Nested orchestration depth is two.

Ownership boundaries prevent simultaneous edits to shared files. Managers must define allowed directories, forbidden shared contracts, required tests, and expected evidence for every dispatch.

## 9. GitHub Flow

`main` rejects direct substantive pushes, deletions, and non-fast-forward updates. Work follows:

```text
issue/task -> focused branch -> draft PR -> required CI
-> manager review -> coordinator review -> merge queue -> squash merge
```

Every PR is small enough to review as one behavioral unit and contains tests, parity evidence, and SSOT impact. Generated output, vendored binary provenance, and license changes are reviewed explicitly.

The empty repository required one content-free base commit before a PR could exist. All actual repository content begins in the bootstrap PR.

## 10. Documentation contract

- `docs/SSOT/` contains current normative truth and changes with the owning code.
- `docs/WIKI/` contains append-only dated evidence and history.
- Accepted ADRs preserve rationale; a material change creates a new ADR that supersedes the old one.
- Public API and schema reference is generated from code when possible.
- Root and directory-local `AGENTS.md` files are short navigation and local constraint maps, not duplicated architecture documents.
- CI validates links, metadata, generated-reference drift, orphan documents, and executable documentation examples.

## 11. CI and security gates

Pull requests and merge-queue candidates run secretless, least-privilege workflows:

- Rust: rustfmt, Clippy with warnings denied, locked build/test, rustdoc, doctests
- Python: Ruff, static typing, pytest, wheel-installed integration tests
- workflows: actionlint and zizmor
- security: CodeQL for Rust/Python/Actions, cargo-audit, cargo-deny, dependency review
- quality: cargo-llvm-cov, golden parity, property tests, documentation validation
- fuzzing: short PR smoke campaigns and longer scheduled parser/protocol campaigns
- packaging: wheel contents, license, SBOM, provenance, and forbidden-path scans

Actions use minimum permissions and third-party actions pinned to full commit SHAs. Release publishing is isolated in a protected environment and uses PyPI Trusted Publishing with short-lived OIDC credentials. Wheels and sdists receive checksums, SBOMs, and artifact attestations.

No finite gate proves the absence of all defects. Reliability comes from overlapping type, lint, unit, integration, parity, corpus, fuzz, platform, security, and release-installation gates.

## 12. Platform matrix

Native build and installation tests cover:

- Linux x86_64 and aarch64
- Windows x86_64 and ARM64
- macOS x86_64 and aarch64
- every stable CPython minor from 3.10 through latest

Linux wheels follow an explicit manylinux policy. Every wheel is installed and exercised on its matching native runner before publication.

## 13. Migration DAG

```text
contract inventory + golden harness
                 |
      Rust/Python foundation and CI
                 |
        +--------+--------+
        |        |        |
     Hancom   PDF/OCR   Office
     parsers   parser    parsers
        +--------+--------+
                 |
       shared parse integration
                 |
       pure IR transformations
                 |
     fill/patch/redact/roundtrip
                 |
       generation and rendering
                 |
       Python + MCP full parity
                 |
     native packaging and release
                 |
         remove Node and npm
```

Stages merge only after their local parity gate. MCP adapters may develop against frozen schemas in parallel but cannot claim completion before the underlying Rust operation passes.

## 14. Parity gates

Compare more than Markdown:

- normalized recursive IR and table topology
- document and page Markdown
- metadata, outline, images, page data, warnings, and errors
- encrypted, damaged, oversized, and unsupported input behavior
- normalized HWPX ZIP/XML structure and generation semantics
- roundtrip reparse results and residual sensitive data
- render geometry and approved visual hashes
- OCR accuracy, execution time, and peak resource use
- existing corpus benchmark dimensions without altering answers or scoring

Normalization removes only non-semantic differences such as ZIP timestamps and XML attribute ordering. Evaluator or ground-truth changes are isolated and labeled rather than mixed with implementation improvements.

Performance tracks parser throughput, peak RSS, import latency, MCP first response, and wheel size. An unexplained regression greater than 10% blocks merge.

## 15. Node removal gate

Node/npm removal requires all of the following:

1. Every public capability is classified in the compatibility manifest.
2. Only the Node library and general CLI are intentionally removed.
3. All 17 MCP tools pass schema, behavior, error, and security parity.
4. All permitted fixture and corpus gates pass without weakened evaluation.
5. Restricted corpora pass on protected runners without exposing originals.
6. Every target wheel installs and passes native smoke tests.
7. Every supported CPython version passes API and MCP tests.
8. No unclassified fuzz crash remains.
9. Security, licenses, SBOM, and provenance gates pass.
10. Two consecutive full release-candidate workflows pass.
11. End-to-end Python and MCP workflows pass on representative documents.
12. A dedicated removal PR proves zero Node/npm runtime and build references.

Only then may the local oracle be archived or removed independently of the Git repository.

## 16. Research basis

- AGENTS.md project guidance: <https://agents.md/>
- Diataxis documentation architecture: <https://diataxis.fr/>
- Architecture Decision Records: <https://adr.github.io/>
- GitHub repository rulesets: <https://docs.github.com/en/repositories/configuring-branches-and-merges-in-your-repository/managing-rulesets/about-rulesets>
- GitHub merge queue: <https://docs.github.com/en/repositories/configuring-branches-and-merges-in-your-repository/configuring-pull-request-merges/managing-a-merge-queue>
- GitHub hosted runners: <https://docs.github.com/en/actions/reference/runners/github-hosted-runners>
- Maturin distribution: <https://www.maturin.rs/distribution.html>
- Python version status: <https://devguide.python.org/versions/>
- Rust API guidelines: <https://rust-lang.github.io/api-guidelines/>
