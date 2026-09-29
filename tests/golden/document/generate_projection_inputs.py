"""Generate small CC0 inputs for the P7 pure-projection oracle matrix."""

from __future__ import annotations

import json
from pathlib import Path

ROOT = Path(__file__).resolve().parent
FIXTURES = ROOT / "fixtures"


def cell(text: str = "", **extra: object) -> dict[str, object]:
    return {"text": text, "colSpan": 1, "rowSpan": 1, **extra}


def table(rows: list[list[dict[str, object]]], **extra: object) -> dict[str, object]:
    return {
        "rows": len(rows),
        "cols": max(map(len, rows), default=0),
        "cells": rows,
        "hasHeader": len(rows) > 1,
        **extra,
    }


def projection_inputs() -> dict[str, dict[str, object]]:
    dense = table(
        [
            [cell("이름"), cell("부서"), cell("인원")],
            [cell("홍길동"), cell("개발"), cell("10")],
            [cell("김철수"), cell("기획"), cell("7")],
            [cell("이영희"), cell("영업"), cell("12")],
            [cell("박민수"), cell("총무"), cell("3")],
        ]
    )
    sparse = table(
        [
            [cell(), cell(), cell("기관장"), cell(), cell()],
            [cell(), cell(), cell(), cell(), cell()],
            [cell(), cell("기획본부"), cell(), cell("사업본부"), cell()],
            [cell(), cell(), cell(), cell(), cell()],
            [cell("기획팀"), cell(), cell("총무팀"), cell(), cell("사업팀")],
            [cell(), cell(), cell(), cell(), cell()],
            [cell(), cell(), cell(), cell(), cell()],
        ]
    )
    nested = table(
        [[cell("내부 항목"), cell("내부 값")], [cell("A"), cell("1")]],
    )
    wrapper = table(
        [[cell("", blocks=[{"type": "table", "table": nested}])]],
        hasHeader=False,
    )
    uncertain = table(
        [[cell("a"), cell("b")], [cell("c"), cell("d")]],
        hasHeader=True,
    )

    return {
        "projection-ordered-blocks": {
            "operation": "markdown",
            "blocks": [
                {"type": "heading", "level": 1, "text": "P7 투영"},
                {"type": "paragraph", "text": "안전 <span> & A | B"},
                {"type": "paragraph", "text": "unsafe", "href": "javascript:alert(1)"},
                {"type": "paragraph", "text": "본문", "footnoteText": "주석"},
                {"type": "list", "listType": "unordered", "text": "항목"},
                {"type": "image", "text": "asset(image).png"},
                {"type": "paragraph", "text": "PUA \uf0a9"},
                {"type": "separator"},
            ],
        },
        "projection-pages-gap": {
            "operation": "pages",
            "blocks": [
                {"type": "paragraph", "text": "선두 무페이지"},
                {"type": "paragraph", "text": "1쪽 본문", "pageNumber": 1},
                {"type": "paragraph", "text": "이어지는 본문"},
                {"type": "paragraph", "text": "3쪽 본문", "pageNumber": 3},
            ],
        },
        "projection-chunks-tree": {
            "operation": "chunks",
            "options": {"granularity": "section", "includeTableCells": True},
            "blocks": [
                {"type": "heading", "level": 1, "text": "계획"},
                {"type": "heading", "level": 2, "text": "1. 개요"},
                {"type": "paragraph", "text": "도입"},
                {"type": "paragraph", "text": "□ 항목", "listDepth": 0},
                {"type": "paragraph", "text": "○ 하위", "listDepth": 1},
                {
                    "type": "table",
                    "table": table(
                        [[cell("항목"), cell("값")], [cell("예산"), cell("100")]]
                    ),
                },
                {"type": "paragraph", "text": ""},
                {"type": "paragraph", "text": "마지막", "pageNumber": 4},
            ],
        },
        "projection-table-merged": {
            "operation": "markdown",
            "blocks": [
                {
                    "type": "table",
                    "table": table(
                        [
                            [cell("통합 제목", colSpan=2), cell("")],
                            [
                                cell("왼쪽", rowSpan=2),
                                cell(
                                    "오른쪽",
                                    blocks=[{"type": "table", "table": nested}],
                                ),
                            ],
                            [cell(""), cell("아래")],
                        ],
                        caption="병합 표",
                    ),
                }
            ],
        },
        "projection-classifier-matrix": {
            "operation": "classify_table_tree",
            "blocks": [
                {"type": "table", "table": dense},
                {"type": "paragraph", "text": "조직도"},
                {"type": "table", "table": sparse},
                {"type": "table", "table": wrapper},
                {"type": "table", "table": uncertain},
            ],
        },
        "projection-html-units": {
            "operation": "markdown",
            "blocks": [
                {"type": "paragraph", "text": "GFM A | B"},
                {
                    "type": "table",
                    "table": table(
                        [
                            [cell("바깥 | 셀"), cell("설명")],
                            [
                                cell(
                                    "부모", blocks=[{"type": "table", "table": nested}]
                                ),
                                cell("끝"),
                            ],
                        ],
                        renderAsTable=True,
                    ),
                },
            ],
        },
    }


def main() -> None:
    FIXTURES.mkdir(parents=True, exist_ok=True)
    for name, value in projection_inputs().items():
        path = FIXTURES / f"{name}.json"
        path.write_text(
            json.dumps(value, ensure_ascii=False, indent=2) + "\n",
            encoding="utf-8",
        )
        print(path.relative_to(ROOT.parent.parent))


if __name__ == "__main__":
    main()
