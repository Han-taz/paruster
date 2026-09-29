"""Validate links and indexes in tracked Markdown files."""

from __future__ import annotations

import argparse
import re
import subprocess
import sys
from dataclasses import dataclass
from pathlib import Path
from urllib.parse import unquote, urlsplit

_LINK_START = re.compile(r"!?\[[^\]]*\]\(\s*")
_REFERENCE_DEFINITION = re.compile(r"^ {0,3}\[([^\]]+)\]:\s*(?:<([^>]+)>|(\S+))")
_REFERENCE_USE = re.compile(r"!?\[([^\]]+)\](?:\[([^\]]*)\])?")
_FENCE = re.compile(r"^ {0,3}(`{3,}|~{3,})")


@dataclass(frozen=True)
class Link:
    line: int
    destination: str


def markdown_links(text: str) -> list[Link]:
    visible_lines: list[tuple[int, str]] = []
    fence_char: str | None = None
    fence_size = 0
    for line_number, line in enumerate(text.splitlines(), 1):
        marker = _FENCE.match(line)
        if marker:
            token = marker.group(1)
            if fence_char is None:
                fence_char, fence_size = token[0], len(token)
            elif token[0] == fence_char and len(token) >= fence_size:
                fence_char, fence_size = None, 0
            continue
        if fence_char is not None:
            continue
        visible_lines.append((line_number, line))

    links: list[Link] = []
    references: dict[str, str] = {}
    for line_number, line in visible_lines:
        definition = _REFERENCE_DEFINITION.match(line)
        if definition:
            label = " ".join(definition.group(1).casefold().split())
            references[label] = definition.group(2) or definition.group(3)
            links.append(Link(line_number, references[label]))

    for line_number, line in visible_lines:
        if _REFERENCE_DEFINITION.match(line):
            continue
        for match in _LINK_START.finditer(line):
            destination, _end = _inline_destination(line, match.end())
            if destination is not None:
                links.append(Link(line_number, destination))
        for match in _REFERENCE_USE.finditer(line):
            after = match.end()
            if after < len(line) and line[after] == "(":
                continue
            explicit_label = match.group(2)
            label = " ".join((explicit_label or match.group(1)).casefold().split())
            if label in references:
                links.append(Link(line_number, references[label]))
    return links


def _inline_destination(line: str, position: int) -> tuple[str | None, int]:
    if position >= len(line):
        return None, position
    if line[position] == "<":
        end = line.find(">", position + 1)
        if end < 0:
            return None, position
        return line[position + 1 : end], end + 1

    destination: list[str] = []
    depth = 0
    current = position
    while current < len(line):
        character = line[current]
        if character == "\\" and current + 1 < len(line):
            destination.append(line[current + 1])
            current += 2
            continue
        if character == "(":
            depth += 1
        elif character == ")":
            if depth == 0:
                break
            depth -= 1
        elif character.isspace() and depth == 0:
            break
        destination.append(character)
        current += 1
    value = "".join(destination)
    return (value or None), current


def _local_target(destination: str) -> str | None:
    parsed = urlsplit(destination)
    if parsed.scheme or parsed.netloc:
        return None
    if not parsed.path:
        return None
    return unquote(parsed.path)


def _resolve_destination(root: Path, source: Path, destination: str) -> Path | None:
    relative = _local_target(destination)
    if relative is None:
        return None
    target = (source.parent / relative).resolve(strict=False)
    try:
        target.relative_to(root.resolve())
    except ValueError:
        return target
    return target


def tracked_markdown(root: Path) -> list[Path]:
    result = subprocess.run(
        ["git", "-C", str(root), "ls-files", "-z", "--", "*.md"],
        check=True,
        capture_output=True,
    )
    paths = result.stdout.split(b"\0")
    return [root / item.decode("utf-8") for item in paths if item]


def _inside_root(root: Path, path: Path) -> bool:
    try:
        path.resolve(strict=False).relative_to(root.resolve())
    except ValueError:
        return False
    return True


def _check_links(root: Path, files: list[Path]) -> list[str]:
    errors: list[str] = []
    for source in files:
        if not source.is_file():
            continue
        text = source.read_text(encoding="utf-8")
        for link in markdown_links(text):
            target = _resolve_destination(root, source, link.destination)
            if target is None:
                continue
            if not _inside_root(root, target):
                errors.append(
                    f"{source.relative_to(root)}:{link.line}: link escapes repository: {link.destination}"
                )
            elif not target.exists():
                errors.append(
                    f"{source.relative_to(root)}:{link.line}: missing local target: {link.destination}"
                )
    return errors


def _linked_targets(root: Path, index: Path) -> list[Path]:
    if not index.is_file():
        return []
    return [
        target
        for link in markdown_links(index.read_text(encoding="utf-8"))
        if (target := _resolve_destination(root, index, link.destination)) is not None
    ]


def _check_wiki_index(root: Path, files: list[Path]) -> list[str]:
    wiki_root = root / "docs" / "WIKI"
    index = wiki_root / "README.md"
    indexed_targets = _linked_targets(root, index)
    entries = [
        path
        for path in files
        if path.is_relative_to(wiki_root)
        and path != index
        and path.name.casefold() != "template.md"
    ]
    errors: list[str] = []
    for entry in entries:
        count = sum(target == entry.resolve(strict=False) for target in indexed_targets)
        if count != 1:
            errors.append(
                f"{index.relative_to(root)}: {entry.relative_to(root)} must appear exactly once (found {count})"
            )
    return errors


def _nearest_ssot_index(root: Path, page: Path) -> Path | None:
    ssot_root = root / "docs" / "SSOT"
    directory = page.parent
    while True:
        candidate = directory / "README.md"
        if candidate.is_file():
            return candidate
        if directory == ssot_root or not _inside_root(ssot_root, directory):
            return None
        directory = directory.parent


def _check_ssot_indexes(root: Path, files: list[Path]) -> list[str]:
    ssot_root = root / "docs" / "SSOT"
    pages = [
        path
        for path in files
        if path.is_relative_to(ssot_root) and path.name.casefold() != "readme.md"
    ]
    errors: list[str] = []
    for page in pages:
        index = _nearest_ssot_index(root, page)
        if index is None:
            errors.append(f"{page.relative_to(root)}: no nearest SSOT README.md index")
            continue
        if page.resolve(strict=False) not in _linked_targets(root, index):
            errors.append(
                f"{index.relative_to(root)}: {page.relative_to(root)} is not linked from its nearest index"
            )
    return errors


def check_docs(root: Path) -> list[str]:
    root = root.resolve()
    files = tracked_markdown(root)
    return [
        *_check_links(root, files),
        *_check_wiki_index(root, files),
        *_check_ssot_indexes(root, files),
    ]


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--root",
        type=Path,
        default=Path(__file__).resolve().parents[1],
        help="Git repository root (defaults to this checkout)",
    )
    args = parser.parse_args(argv)
    try:
        errors = check_docs(args.root)
    except (OSError, subprocess.CalledProcessError) as error:
        print(f"cannot inspect tracked Markdown: {error}", file=sys.stderr)
        return 2
    for message in errors:
        print(message, file=sys.stderr)
    if errors:
        return 1
    print("documentation links and indexes are valid")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
