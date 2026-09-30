from __future__ import annotations

import json
import re
from pathlib import Path

ROOT = Path(__file__).parents[2]


def test_parser_scaffolds_are_protected_workspace_members() -> None:
    workspace = (ROOT / "Cargo.toml").read_text(encoding="utf-8")
    members_match = re.search(r"members\s*=\s*\[(.*?)\]", workspace, re.DOTALL)
    assert members_match is not None
    members = set(re.findall(r'"([^"]+)"', members_match.group(1)))
    assert members == {
        "crates/kordoc-ir",
        "crates/kordoc-core",
        "crates/kordoc-hancom",
        "crates/kordoc-pdf",
        "crates/kordoc-python",
    }


def test_format_crates_keep_the_acyclic_dependency_boundary() -> None:
    hancom = (ROOT / "crates/kordoc-hancom/Cargo.toml").read_text(encoding="utf-8")
    pdf = (ROOT / "crates/kordoc-pdf/Cargo.toml").read_text(encoding="utf-8")
    for manifest in (hancom, pdf):
        assert 'kordoc-ir = { path = "../kordoc-ir", version = "=0.1.0" }' in manifest
        assert "kordoc-core" not in manifest
        assert "kordoc-python" not in manifest
        assert "[workspace]" not in manifest
    for dependency in (
        "aes.workspace = true",
        "cbc.workspace = true",
        "flate2.workspace = true",
        "pbkdf2.workspace = true",
        "quick-xml.workspace = true",
        "sha1.workspace = true",
        "sha2.workspace = true",
        "subtle.workspace = true",
        "zip.workspace = true",
    ):
        assert dependency in hancom
    assert "lopdf.workspace = true" in pdf


def test_parser_dependencies_are_exact_reviewed_candidates() -> None:
    workspace = (ROOT / "Cargo.toml").read_text(encoding="utf-8")
    exact_pins = (
        "=0.42.0",
        "=0.9.3",
        "=0.2.1",
        "=0.13.0",
        "=0.11.0",
        "=2.6.1",
        "=1.1.10",
        "=0.45.0",
    )
    for version in exact_pins:
        assert f'"{version}"' in workspace
    assert 'lopdf = { version = "=0.45.0", default-features = false }' in workspace
    assert 'quick-xml = { version = "=0.42.0", default-features = false }' in workspace


def test_hwpx_candidate_registration_does_not_promote_unverified_parser_capability() -> (
    None
):
    public_api = json.loads(
        (ROOT / "contracts/public-api.json").read_text(encoding="utf-8")
    )
    by_source = {entry["source_name"]: entry for entry in public_api["entries"]}
    for source_name in ("parseHwpx", "validateHwpx", "parsePdf"):
        assert by_source[source_name]["disposition"] == "planned"

    core_manifest = (ROOT / "crates/kordoc-core/Cargo.toml").read_text(encoding="utf-8")
    assert (
        'kordoc-hancom = { path = "../kordoc-hancom", version = "=0.1.0" }'
        in core_manifest
    )
    assert "kordoc-pdf" not in core_manifest

    hancom_source = (ROOT / "crates/kordoc-hancom/src/lib.rs").read_text(
        encoding="utf-8"
    )
    pdf_lib = (ROOT / "crates/kordoc-pdf/src/lib.rs").read_text(encoding="utf-8")
    pdf_parser = (ROOT / "crates/kordoc-pdf/src/parser.rs").read_text(encoding="utf-8")
    assert "pub fn parse_hwpx" in hancom_source
    assert "pub fn validate_hwpx" in hancom_source
    assert "hwpx::parse_hwpx(bytes, options)" in hancom_source
    assert "hwpx::validate_hwpx(bytes, password)?" in hancom_source
    assert "pub fn parse_pdf" not in pdf_lib
    assert "pub use parser" not in pdf_lib
    assert "pub fn parse_pdf" not in pdf_parser
