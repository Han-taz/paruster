#!/usr/bin/env python3
"""Recreate the authored synthetic raw-item inputs for PDF text rewrite captures."""

from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path

FIXTURE_DIR = Path(__file__).resolve().parent
INPUTS = FIXTURE_DIR / "rewrite-inputs.json"
RADICALS = FIXTURE_DIR / "radical-inputs.json"


def item(
    text: str,
    *,
    size: float = 12,
    x: float = 10,
    y: float = 10,
    width: float = 30,
    height: float = 12,
    font_name: str = "authored-font",
) -> dict[str, object]:
    return {
        "str": text,
        "transform": [size, 0, 0, size, x, y],
        "width": width,
        "height": height,
        "fontName": font_name,
    }


def build_inputs() -> dict[str, object]:
    return {
        "schema_version": 1,
        "description": "Authored synthetic PdfTextItem records passed to pinned normalizeItems.",
        "cases": [
            {"name": "numeric_ascii_space_rewrite", "items": [item("45 0 -7 3 40 )")]},
            {
                "name": "numeric_full_permitted_punctuation_class",
                "items": [item("☎ (45.0, -7·3)")],
            },
            {
                "name": "numeric_non_ascii_whitespace_survives_ascii_space_removal",
                "items": [item("1\t2\u00a03\u30004 5")],
            },
            {
                "name": "numeric_newline_survives_ascii_space_removal",
                "items": [item("1\n2 3")],
            },
            {
                "name": "nel_is_not_ecmascript_regex_whitespace",
                "items": [item("1\u00852 3")],
            },
            {"name": "phone_symbol_is_permitted", "items": [item("☎ 1 2")]},
            {"name": "plus_sign_prevents_numeric_rewrite", "items": [item("+ 1 2")]},
            {
                "name": "unicode_digit_prevents_numeric_rewrite",
                "items": [item("١ 2 3")],
            },
            {
                "name": "astral_neighbor_prevents_numeric_rewrite",
                "items": [item("😀1 2")],
            },
            {
                "name": "ecmascript_edge_trim_includes_bom_nbsp_em_space_and_line_separator",
                "items": [item("\ufeff\u00a0A\u2003\u2028")],
            },
            {"name": "ecmascript_trim_retains_nel", "items": [item("\u0085N\u0085")]},
            {
                "name": "radical_nfkc_preserves_astral_neighbors",
                "items": [item("x😀⼀😀y")],
            },
            {"name": "nfkc_is_limited_to_radical_block", "items": [item("Ａ⼀①⿕⿖")]},
            {
                "name": "uppercase_label_at_rounded_font_gate",
                "items": [item("H O W", size=13.5)],
            },
            {
                "name": "uppercase_label_below_font_gate",
                "items": [item("H O W", size=13.499)],
            },
            {
                "name": "uppercase_label_accepts_curly_apostrophe",
                "items": [item("L ’ E T", size=16)],
            },
            {
                "name": "uppercase_label_allows_numeric_and_ascii_punctuation",
                "items": [item("A 1 ? ! & ' ’", size=16)],
            },
            {
                "name": "uppercase_label_rejects_non_ascii_letter",
                "items": [item("Å B C", size=16)],
            },
            {
                "name": "uppercase_label_rejects_two_groups",
                "items": [item("A B", size=16)],
            },
            {
                "name": "uppercase_label_rejects_double_space",
                "items": [item("A  B C", size=16)],
            },
            {
                "name": "rewrite_precedes_even_spacing_split_and_sort_preserves_sequence",
                "items": [
                    item("1 2 3", x=100, y=20, width=30),
                    item("가 나 다", x=10, y=30, width=30),
                ],
            },
        ],
    }


def build_radicals() -> dict[str, object]:
    return {
        "schema_version": 1,
        "description": "Every code point in the exact Kangxi radical replacement range.",
        "items": [
            {"code_point": f"U+{code_point:04X}", "text": chr(code_point)}
            for code_point in range(0x2F00, 0x2FD6)
        ],
    }


def encoded(value: dict[str, object]) -> bytes:
    return (json.dumps(value, ensure_ascii=False, indent=2) + "\n").encode("utf-8")


def expected_files() -> dict[Path, bytes]:
    return {INPUTS: encoded(build_inputs()), RADICALS: encoded(build_radicals())}


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument(
        "--write", action="store_true", help="regenerate authored input files"
    )
    args = parser.parse_args()
    expected_files_by_path = expected_files()
    if args.write:
        for path, expected in expected_files_by_path.items():
            path.write_bytes(expected)
        return 0
    for path, expected in expected_files_by_path.items():
        try:
            actual = path.read_bytes()
        except OSError as error:
            print(f"missing authored input file {path.name}: {error}", file=sys.stderr)
            return 1
        if actual != expected:
            print(
                f"{path.name} differs from the deterministic authored recipe",
                file=sys.stderr,
            )
            return 1
    print("authored rewrite input verified")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
