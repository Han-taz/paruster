from __future__ import annotations

from collections.abc import Mapping
from types import MappingProxyType
from typing import Any

import kordoc
import pytest
from kordoc import _api, _native
from kordoc._api import (
    _normalize_blocks_with_budget,
    blocks_to_chunks,
    blocks_to_markdown,
    blocks_to_pages,
)
from kordoc._models import ChunkOptions, DocChunk, PageMarkdown


def test_blocks_to_markdown_exact(monkeypatch: pytest.MonkeyPatch) -> None:
    received: list[Any] = []

    def render(blocks: list[dict[str, object]]) -> str:
        received.append(blocks)
        return "# Title\n\nBody"

    monkeypatch.setattr(_native, "blocks_to_markdown_wire", render, raising=False)
    actual = blocks_to_markdown(
        [
            {"type": "heading", "text": "Title", "level": 1},
            {"type": "paragraph", "text": "Body"},
        ]
    )

    assert actual == "# Title\n\nBody"
    assert received == [
        [
            {"type": "heading", "text": "Title", "level": 1},
            {"type": "paragraph", "text": "Body"},
        ]
    ]


def test_blocks_to_pages_omission_and_gap(monkeypatch: pytest.MonkeyPatch) -> None:
    callback_blocks: list[tuple[Mapping[str, object], ...]] = []

    def native_pages(
        blocks: list[dict[str, object]], render: Any = None
    ) -> list[dict[str, object]] | None:
        if not any("pageNumber" in block for block in blocks):
            return None
        assert render is not None
        custom_markdown = render((blocks[0], blocks[1]))
        return [
            {"pageNumber": 1, "markdown": custom_markdown},
            {"pageNumber": 2, "markdown": ""},
            {"pageNumber": 3, "markdown": "third"},
        ]

    monkeypatch.setattr(_native, "blocks_to_pages_wire", native_pages, raising=False)
    assert blocks_to_pages([{"type": "paragraph", "text": "no page"}]) is None

    def renderer(blocks: tuple[Mapping[str, object], ...]) -> str:
        assert isinstance(blocks, tuple)
        assert all(isinstance(block, MappingProxyType) for block in blocks)
        callback_blocks.append(blocks)
        with pytest.raises(TypeError):
            blocks[0]["text"] = "mutated"  # type: ignore[index]
        return "leading and first page"

    pages = blocks_to_pages(
        [
            {"type": "paragraph", "text": "leading"},
            {"type": "paragraph", "text": "first", "pageNumber": 1},
            {"type": "paragraph", "text": "third", "pageNumber": 3},
        ],
        render=renderer,
    )
    assert pages == (
        PageMarkdown(1, "leading and first page"),
        PageMarkdown(2, ""),
        PageMarkdown(3, "third"),
    )
    assert callback_blocks[0][0]["text"] == "leading"


def test_blocks_to_chunks_structural_types(monkeypatch: pytest.MonkeyPatch) -> None:
    seen_options: list[dict[str, object] | None] = []

    def chunks(
        blocks: list[dict[str, object]], options: dict[str, object] | None = None
    ) -> list[dict[str, object]]:
        seen_options.append(options)
        return [
            {
                "id": "c0001",
                "type": "table",
                "breadcrumb": ["Section"],
                "text": "A | B",
                "page": 2,
                "blockRange": [1, 1],
                "table": {"rows": 1, "cols": 2, "cells": [["A", "B"]]},
            }
        ]

    monkeypatch.setattr(_native, "blocks_to_chunks_wire", chunks, raising=False)
    options = ChunkOptions(include_table_cells=True, granularity="block")
    result = blocks_to_chunks([{"type": "table", "pageNumber": 2}], options=options)

    assert result == (
        DocChunk(
            id="c0001",
            type="table",
            breadcrumb=("Section",),
            text="A | B",
            block_range=(1, 1),
            page=2,
            table={"rows": 1, "cols": 2, "cells": (("A", "B"),)},
        ),
    )
    assert seen_options == [{"includeTableCells": True, "granularity": "block"}]
    assert result[0].to_dict() == {
        "id": "c0001",
        "type": "table",
        "breadcrumb": ["Section"],
        "text": "A | B",
        "page": 2,
        "blockRange": [1, 1],
        "table": {"rows": 1, "cols": 2, "cells": [["A", "B"]]},
    }
    with pytest.raises(TypeError):
        result[0].table["rows"] = 2  # type: ignore[index]


def test_table_policy_nested(monkeypatch: pytest.MonkeyPatch) -> None:
    output = [
        {
            "type": "table",
            "table": {
                "rows": 1,
                "cols": 1,
                "classification": {
                    "kind": "uncertain",
                    "confidence": 0.4,
                    "semanticScore": 0.3,
                    "nonTabularScore": 0.3,
                    "reasons": ["low-evidence"],
                },
                "cells": [[{"text": "nested", "blocks": [{"type": "paragraph"}]}]],
            },
        }
    ]
    monkeypatch.setattr(
        _native, "classify_tables_wire", lambda blocks: output, raising=False
    )
    result = _api._classify_tables([{"type": "table", "table": {"rows": 1, "cols": 1}}])

    assert result[0]["table"]["classification"]["kind"] == "uncertain"
    assert isinstance(result[0], MappingProxyType)
    assert isinstance(result[0]["table"], MappingProxyType)
    assert isinstance(result[0]["table"]["cells"], tuple)
    with pytest.raises(TypeError):
        result[0]["table"]["rows"] = 9  # type: ignore[index]


def test_table_tree_has_contract_module_path(monkeypatch: pytest.MonkeyPatch) -> None:
    from kordoc.tables import classify_table_tree

    classified = [{"type": "table", "table": {"rows": 1, "cols": 1}}]
    monkeypatch.setattr(
        _native, "classify_tables_wire", lambda blocks: classified, raising=False
    )
    assert not hasattr(kordoc, "classify_tables")
    assert classify_table_tree(classified)[0]["table"]["rows"] == 1


def test_projection_errors_are_typed(monkeypatch: pytest.MonkeyPatch) -> None:
    def too_large(blocks: list[dict[str, object]]) -> str:
        raise ValueError("OUTPUT_TOO_LARGE", "projection output exceeded")

    monkeypatch.setattr(_native, "blocks_to_markdown_wire", too_large, raising=False)
    with pytest.raises(kordoc.OutputTooLargeError) as error:
        blocks_to_markdown([{"type": "paragraph", "text": "x"}])
    assert error.value.code == "OUTPUT_TOO_LARGE"

    for blocks in (
        None,
        "not a block sequence",
        [{"type": "paragraph", "text": None}],
        [None],
    ):
        with pytest.raises((TypeError, ValueError)):
            blocks_to_markdown(blocks)  # type: ignore[arg-type]


def test_projection_input_size_limit_is_checked_before_native(
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    with pytest.raises(kordoc.OutputTooLargeError):
        _normalize_blocks_with_budget([{"text": "too big"}], max_bytes=3)

    def unexpected(blocks: list[dict[str, object]]) -> str:
        pytest.fail("oversized input reached the native worker")

    monkeypatch.setattr(_native, "blocks_to_markdown_wire", unexpected, raising=False)
    monkeypatch.setattr(_api, "MAX_MARKDOWN_BYTES", 3)
    with pytest.raises(kordoc.OutputTooLargeError):
        blocks_to_markdown([{"text": "oversized"}])


def test_chunk_options_reject_explicit_null() -> None:
    with pytest.raises(TypeError):
        ChunkOptions.from_dict({"granularity": None})
    with pytest.raises(TypeError):
        blocks_to_chunks([], options={"includeTableCells": None})


def test_doc_chunk_validates_u32_and_string_table_cells() -> None:
    wire = {
        "id": "c0001",
        "type": "table",
        "breadcrumb": [],
        "text": "A",
        "blockRange": [0, 0],
        "page": 0,
        "table": {"rows": 1, "cols": 1, "cells": [["A"]]},
    }
    assert DocChunk.from_dict(wire).to_dict() == wire
    for invalid in (
        {**wire, "page": 2**32},
        {**wire, "blockRange": [0, 2**32]},
        {**wire, "table": {"rows": 2**32, "cols": 1}},
        {**wire, "table": {"rows": 1, "cols": 1, "cells": [[{"text": "A"}]]}},
    ):
        with pytest.raises((TypeError, ValueError)):
            DocChunk.from_dict(invalid)


def test_doc_chunk_rejects_explicit_null_optionals() -> None:
    wire = {
        "id": "c0001",
        "type": "text",
        "breadcrumb": [],
        "text": "A",
        "blockRange": [0, 0],
    }
    for invalid in ({**wire, "page": None}, {**wire, "table": None}):
        with pytest.raises((TypeError, ValueError)):
            DocChunk.from_dict(invalid)


def test_projection_item_budget_covers_two_million_cells() -> None:
    assert _api._MAX_PROJECTION_ITEMS >= 16_000_000
    cells = [["x"] * 1_000] * 2_000
    value = {"table": {"rows": 2_000, "cols": 1_000, "cells": cells}}
    assert _api._validate_projection_output(value) is value


def test_native_json_projection_inputs_are_not_deep_copied() -> None:
    nested = {"values": ["keep", ("tuple",)]}
    block = {"type": "paragraph", "text": "x", "nested": nested}
    blocks = [block]

    normalized = _api._normalize_blocks(blocks)

    assert normalized is blocks
    assert normalized[0] is block
    assert normalized[0]["nested"] is nested
    assert normalized[0]["nested"]["values"] is nested["values"]


def test_generic_mapping_projection_input_is_materialized() -> None:
    nested = MappingProxyType({"values": ("convert",)})
    block = {"type": "paragraph", "text": "x", "nested": nested}

    normalized = _api._normalize_blocks((block,))

    assert normalized[0] is not block
    assert normalized[0]["nested"] is not nested
    assert normalized[0]["nested"] == {"values": ["convert"]}


def test_page_callback_exception_is_propagated_unchanged(
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    sentinel = RuntimeError("callback failure")

    def native_pages(blocks: list[dict[str, object]], render: Any) -> object:
        render((blocks[0],))
        return []

    monkeypatch.setattr(_native, "blocks_to_pages_wire", native_pages, raising=False)

    def renderer(blocks: tuple[Mapping[str, object], ...]) -> str:
        raise sentinel

    with pytest.raises(RuntimeError) as raised:
        blocks_to_pages([{"type": "paragraph", "text": "x"}], render=renderer)
    assert raised.value is sentinel


def test_success_document_projection_from_test_adapter(
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    success = {
        "success": True,
        "fileType": "pdf",
        "markdown": "page one",
        "blocks": [{"type": "paragraph", "text": "page one", "pageNumber": 1}],
        "pages": [{"pageNumber": 1, "markdown": "page one"}],
    }
    monkeypatch.setattr(_native, "try_parse_bytes", lambda data: success)

    result = kordoc.try_parse(b"test adapter input")
    document = result.document
    assert document is not None
    assert document.markdown == "page one"
    assert document.pages == ({"pageNumber": 1, "markdown": "page one"},)
    assert result.to_dict() == success
    assert PageMarkdown.from_dict(document.pages[0]).to_dict() == {
        "pageNumber": 1,
        "markdown": "page one",
    }


def test_installed_native_projection_end_to_end() -> None:
    from kordoc.tables import classify_table_tree

    blocks = [
        {"type": "heading", "level": 1, "text": "Section", "pageNumber": 1},
        {"type": "paragraph", "text": "Body", "pageNumber": 3},
        {
            "type": "table",
            "pageNumber": 3,
            "table": {
                "rows": 1,
                "cols": 1,
                "hasHeader": False,
                "cells": [[{"text": "A", "colSpan": 1, "rowSpan": 1}]],
            },
        },
    ]
    assert _native.blocks_to_markdown_wire(blocks).startswith("# Section")

    rendered_inputs: list[tuple[Mapping[str, object], ...]] = []

    def renderer(page_blocks: tuple[Mapping[str, object], ...]) -> str:
        rendered_inputs.append(page_blocks)
        return "; ".join(str(block.get("text", "")) for block in page_blocks)

    pages = blocks_to_pages(blocks, render=renderer)
    assert pages is not None
    assert tuple(page.page_number for page in pages) == (1, 2, 3)
    assert pages[1].markdown == ""
    assert len(rendered_inputs) == 2
    sentinel = RuntimeError("native callback sentinel")

    def failing_renderer(page_blocks: tuple[Mapping[str, object], ...]) -> str:
        raise sentinel

    with pytest.raises(RuntimeError) as raised:
        blocks_to_pages(blocks, render=failing_renderer)
    assert raised.value is sentinel

    chunks = blocks_to_chunks(
        blocks, ChunkOptions(include_table_cells=True, granularity="block")
    )
    assert chunks[0].id == "c0001"
    assert chunks[0].block_range == (0, 0)
    assert chunks[-1].table is not None
    tree = classify_table_tree(blocks)
    assert tree[-1]["table"]["classification"]


def test_native_semantic_depth_limit_exceeds_raw_preflight_depth() -> None:
    block: dict[str, object] = {"type": "paragraph", "text": "deep"}
    for _ in range(32):
        block = {"type": "list", "text": "item", "children": [block]}
    assert blocks_to_markdown([block])

    for _ in range(33):
        block = {"type": "list", "text": "item", "children": [block]}
    with pytest.raises(kordoc.OutputTooLargeError):
        blocks_to_markdown([block])
