# Development environment

This guide uses the repository's locked tool versions and the minimum supported CPython for building the abi3 extension. The supported Python range is CPython 3.10–3.14; the full minor-version behavior matrix runs on Linux x86_64 in CI.

## Bootstrap

Install CPython 3.10 and resolve only the exact development dependencies in `uv.lock`:

```bash
uv python install 3.10
uv sync --all-groups --locked --python 3.10
```

Build the extension against that interpreter explicitly. PyO3 is configured for the `abi3-py310` floor, and `PYO3_PYTHON` prevents a different active interpreter from being selected accidentally:

```bash
export PYO3_PYTHON="$(uv run --python 3.10 python -c 'import sys; print(sys.executable)')"
uv run --python 3.10 maturin develop
```

The project pins maturin 1.15.0, PyO3 0.29.2, and Rust 1.97.0. Use the locked Rust toolchain and components for local checks:

```bash
rustup toolchain install 1.97.0 --profile minimal --component clippy,rustfmt,llvm-tools-preview
rustup toolchain install nightly-2026-09-20 --profile minimal
cargo +1.97.0 install cargo-llvm-cov --version 0.9.1 --locked
cargo +nightly-2026-09-20 install cargo-fuzz --version 0.13.2 --locked
```

## Full local quality gate

Run this gate before requesting review. It mirrors the repository's CI, bounded fuzz, coverage, artifact, and security checks. The security CLIs must be installed from checksum-verified releases at the versions recorded in `security.yml`.

```bash
# Rust formatting, lint, tests, and warning-free API documentation
cargo +1.97.0 fmt --all -- --check
cargo +1.97.0 clippy --workspace --all-targets --all-features --locked -- -D warnings
cargo +1.97.0 test --workspace --locked
RUSTDOCFLAGS=-Dwarnings cargo +1.97.0 doc --workspace --no-deps --locked

# Foundation line coverage (cargo-llvm-cov 0.9.1)
cargo +1.97.0 llvm-cov -p kordoc-ir -p kordoc-core --locked --fail-under-lines 80

# Bounded safety campaigns (nightly-2026-09-20 and cargo-fuzz 0.13.2)
cargo +nightly-2026-09-20 fuzz run detect_format -- -max_total_time=30
cargo +nightly-2026-09-20 fuzz run zip_preflight -- -max_total_time=30
cargo +nightly-2026-09-20 fuzz run markdown_units -- -max_total_time=30
cargo +nightly-2026-09-20 fuzz run projections -- -max_total_time=30

# Python behavior, lint, formatting, and types
uv run pytest tests/contracts tests/parity tests/python -q
uv run ruff check python tests scripts
uv run ruff format --check python tests scripts
uv run mypy python/kordoc scripts

# Build both distribution forms and inspect their contents without extraction
uv run maturin build --release --locked --out target/wheels
uv run maturin sdist --out target/wheels
uv run python scripts/check_artifacts.py target/wheels/*.whl target/wheels/*.tar.gz
uv run python scripts/check_docs.py

# Workflow and dependency policy; pinned versions: actionlint 1.7.12,
# zizmor 1.30.1, cargo-deny 0.20.2, cargo-audit 0.22.2
actionlint .github/workflows/*.yml
zizmor .github/workflows
cargo deny --locked check
cargo audit --file Cargo.lock
```

The PR CI additionally exercises the installed wheel and Python 3.10–3.14 matrix. Fuzz campaigns run for 30 seconds per target in CI and weekly campaigns run longer; a local 30-second pass is a safety smoke check, not a substitute for reviewing any discovered crash or regression.
