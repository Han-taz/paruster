from __future__ import annotations

import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).parents[2]
CHECKER = ROOT / "scripts" / "check_docs.py"


def _write(root: Path, relative: str, content: str) -> None:
    target = root / relative
    target.parent.mkdir(parents=True, exist_ok=True)
    target.write_text(content, encoding="utf-8")


def _repo(root: Path) -> None:
    subprocess.run(["git", "init", "-q", str(root)], check=True)
    _write(
        root,
        "README.md",
        "[local](docs/SSOT/contracts/page.md) [web](https://example.com) [anchor](#top)\n"
        "[paren](target%20%28v1%29.md) [raw-paren](target(v1).md) [reference][target]\n"
        "```md\n[ignored](missing.md)\n```\n",
    )
    _write(root, "target (v1).md", "A link destination with parentheses.\n")
    _write(root, "target(v1).md", "An unescaped balanced-parentheses destination.\n")
    _write(root, "target.md", "A reference-style destination.\n")
    with (root / "README.md").open("a", encoding="utf-8") as readme:
        readme.write("\n[target]: target.md\n")
    _write(root, "docs/SSOT/README.md", "[Page](contracts/page.md)\n")
    _write(root, "docs/SSOT/contracts/page.md", "A current SSOT page.\n")
    _write(
        root,
        "docs/WIKI/README.md",
        "[Entry](2026/09/entry.md)\n",
    )
    _write(root, "docs/WIKI/2026/09/entry.md", "A WIKI entry.\n")
    subprocess.run(["git", "-C", str(root), "add", "."], check=True)


def _run(root: Path) -> subprocess.CompletedProcess[str]:
    return subprocess.run(
        [sys.executable, str(CHECKER), "--root", str(root)],
        check=False,
        capture_output=True,
        text=True,
    )


def test_accepts_valid_local_external_anchor_and_fenced_code_links(
    tmp_path: Path,
) -> None:
    _repo(tmp_path)

    result = _run(tmp_path)

    assert result.returncode == 0, result.stderr


def test_rejects_missing_local_link(tmp_path: Path) -> None:
    _repo(tmp_path)
    _write(tmp_path, "README.md", "[missing](not-here.md)\n")
    subprocess.run(["git", "-C", str(tmp_path), "add", "."], check=True)

    result = _run(tmp_path)

    assert result.returncode != 0
    assert "not-here.md" in result.stderr


def test_parses_balanced_parentheses_in_local_link_target(tmp_path: Path) -> None:
    _repo(tmp_path)
    _write(tmp_path, "README.md", "[missing](missing(file).md)\n")
    subprocess.run(["git", "-C", str(tmp_path), "add", "."], check=True)

    result = _run(tmp_path)

    assert result.returncode != 0
    assert "missing(file).md" in result.stderr


def test_rejects_relative_link_escaping_repository(tmp_path: Path) -> None:
    _repo(tmp_path)
    _write(tmp_path, "README.md", "[escape](../../../../outside.md)\n")
    subprocess.run(["git", "-C", str(tmp_path), "add", "."], check=True)

    result = _run(tmp_path)

    assert result.returncode != 0
    assert "outside" in result.stderr


def test_requires_each_non_template_wiki_entry_exactly_once(tmp_path: Path) -> None:
    _repo(tmp_path)
    _write(
        tmp_path,
        "docs/WIKI/README.md",
        "[Entry](2026/09/entry.md) [duplicate](2026/09/entry.md)\n",
    )
    subprocess.run(["git", "-C", str(tmp_path), "add", "."], check=True)

    result = _run(tmp_path)

    assert result.returncode != 0
    assert "exactly once" in result.stderr


def test_uses_nearest_ssot_index(tmp_path: Path) -> None:
    _repo(tmp_path)
    _write(tmp_path, "docs/SSOT/contracts/README.md", "A local index.\n")
    subprocess.run(["git", "-C", str(tmp_path), "add", "."], check=True)

    result = _run(tmp_path)

    assert result.returncode != 0
    assert "contracts/README.md" in result.stderr
