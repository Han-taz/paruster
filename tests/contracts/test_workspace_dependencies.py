from __future__ import annotations

from pathlib import Path

ROOT = Path(__file__).parents[2]


def test_workspace_path_dependencies_have_exact_versions() -> None:
    manifests = (
        ROOT / "crates/kordoc-core/Cargo.toml",
        ROOT / "crates/kordoc-python/Cargo.toml",
    )
    for manifest in manifests:
        text = manifest.read_text(encoding="utf-8")
        for line in text.splitlines():
            if 'path = "../kordoc-' in line:
                assert 'version = "=0.1.0"' in line, f"{manifest}: {line}"
