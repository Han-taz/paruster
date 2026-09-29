from __future__ import annotations

import json
from io import BytesIO
from pathlib import Path

import kordoc
import pytest
from kordoc import _native


def _successful_wire() -> dict[str, object]:
    return {
        "success": True,
        "fileType": "docx",
        "pageCount": 2,
        "markdown": "# Heading\n\nText",
        "blocks": [
            {
                "type": "heading",
                "level": 1,
                "text": "Heading",
                "children": [
                    {
                        "type": "paragraph",
                        "text": "nested",
                        "spans": [{"text": "nested", "bold": True}],
                    }
                ],
            },
            {
                "type": "table",
                "table": {
                    "rows": 1,
                    "cols": 1,
                    "cells": [
                        [
                            {
                                "text": "cell",
                                "rowSpan": 1,
                                "colSpan": 1,
                                "blocks": [{"type": "paragraph", "text": "cell"}],
                            }
                        ]
                    ],
                    "captionBlocks": [{"type": "paragraph", "text": "caption"}],
                },
            },
            {
                "type": "image",
                "imageData": {"data": [2, 254], "mimeType": "image/png"},
            },
        ],
        "metadata": {"title": "Example", "pageCount": 2},
        "outline": [{"level": 1, "text": "Heading", "pageNumber": 1}],
        "warnings": [{"code": "SKIPPED_IMAGE", "message": "image skipped", "page": 2}],
        "images": [
            {"filename": "figure.png", "data": [0, 255], "mimeType": "image/png"}
        ],
        "pages": [{"pageNumber": 1, "markdown": "# Heading"}],
        "pageQuality": [
            {
                "page": 1,
                "textChars": 7,
                "hangulRatio": 0.0,
                "controlCharRatio": 0.0,
                "replacementCharRatio": 0.0,
                "puaRatio": 0.0,
                "needsOcr": False,
            }
        ],
        "qualitySummary": {
            "totalPages": 1,
            "totalTextChars": 7,
            "avgHangulRatio": 0.0,
            "avgControlCharRatio": 0.0,
            "avgReplacementCharRatio": 0.0,
            "avgPuaRatio": 0.0,
            "lowTextPageCount": 0,
            "highPuaPageCount": 0,
            "needsOcr": False,
            "ocrCandidatePages": [1, 2],
        },
    }


def test_parse_success_exposes_immutable_document_from_native_wire(
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    wire = _successful_wire()
    monkeypatch.setattr(_native, "try_parse_bytes", lambda _: wire)

    result = kordoc.parse(b"synthetic native input")

    assert isinstance(result, kordoc.TryParseResult)
    assert result.success is True
    document = result.document
    assert isinstance(document, kordoc.Document)
    assert document.file_type == "docx"
    assert document.markdown == "# Heading\n\nText"
    assert document.images is not None
    assert document.blocks is not None
    assert document.quality_summary is not None
    assert document.to_dict() == {
        key: value for key, value in wire.items() if key != "success"
    }
    assert document.images[0]["data"] == b"\x00\xff"
    assert document.blocks[2]["imageData"]["data"] == b"\x02\xfe"
    assert document.quality_summary["ocrCandidatePages"] == (1, 2)
    serialized = document.to_dict()
    assert serialized["images"][0]["data"] == [0, 255]
    assert serialized["blocks"][2]["imageData"]["data"] == [2, 254]
    assert not hasattr(result, "__dict__")
    assert not hasattr(document, "__dict__")
    with pytest.raises(AttributeError):
        document.markdown = "changed"  # type: ignore[misc]
    with pytest.raises((AttributeError, TypeError)):
        document.blocks[0]["children"].append({"type": "bad"})  # type: ignore[index,union-attr]


def test_try_parse_keeps_flat_serializable_shape_for_success(
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    wire = _successful_wire()
    monkeypatch.setattr(_native, "try_parse_bytes", lambda _: wire)

    result = kordoc.try_parse(b"synthetic native input")
    data = result.to_dict()

    assert isinstance(result, kordoc.TryParseResult)
    assert data["success"] is True
    assert data["fileType"] == "docx"
    assert data["images"] == [
        {"filename": "figure.png", "data": [0, 255], "mimeType": "image/png"}
    ]
    assert json.loads(json.dumps(data)) == data


def test_parse_failure_still_raises_typed_error(
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    monkeypatch.setattr(
        _native,
        "try_parse_bytes",
        lambda _: {
            "success": False,
            "fileType": "pdf",
            "error": "not supported",
            "code": "UNSUPPORTED_FORMAT",
        },
    )

    with pytest.raises(kordoc.UnsupportedFormatError, match="not supported"):
        kordoc.parse(b"synthetic native input")


def test_failed_try_parse_has_no_document_projection() -> None:
    result = kordoc.try_parse(b"")
    assert result.success is False
    assert result.document is None


def test_parse_options_translate_snake_case_to_native_wire_names(
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    seen: list[tuple[bytes, dict[str, object] | None]] = []

    def native_parse(data: bytes, options: dict[str, object] | None = None):
        seen.append((data, options))
        return _successful_wire()

    monkeypatch.setattr(_native, "try_parse_bytes", native_parse)
    options: dict[str, object] = {
        "pages": [1, 2.5],
        "ocr": "force",
        "remove_header_footer": False,
        "script_tags": True,
        "plain": False,
        "html_tables": True,
        "keep_trailing_empty_cols": False,
        "classify_tables": True,
        "keep_empty_paragraphs": False,
        "include_field_placeholders": True,
        "password": "private-value",
        "formula_ocr": False,
        "dedupe_running_headers": True,
        "inline_images": False,
        "images": False,
        "tables": True,
    }

    result = kordoc.try_parse(b"input", options=options)

    assert result.success is True
    assert seen == [
        (
            b"input",
            {
                "pages": [1, 2.5],
                "ocr": "force",
                "removeHeaderFooter": False,
                "scriptTags": True,
                "plain": False,
                "htmlTables": True,
                "keepTrailingEmptyCols": False,
                "classifyTables": True,
                "keepEmptyParagraphs": False,
                "includeFieldPlaceholders": True,
                "password": "private-value",
                "formulaOcr": False,
                "dedupeRunningHeaders": True,
                "inlineImages": False,
                "images": False,
                "tables": True,
            },
        )
    ]


def test_omitted_options_preserve_one_argument_native_call_and_false_is_sent(
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    calls: list[tuple[object, ...]] = []

    def native_parse(*args: object):
        calls.append(args)
        return _successful_wire()

    monkeypatch.setattr(_native, "try_parse_bytes", native_parse)
    kordoc.try_parse(b"default")
    kordoc.try_parse(b"opt-out", options={"images": False})

    assert calls == [
        (b"default",),
        (b"opt-out", {"images": False}),
    ]


@pytest.mark.parametrize(
    "options",
    [
        {"file_path": "/private/document.hwp"},
        {"password": "private-value", "file_path": "/private/document.hwp"},
        {"unknown_option": True},
        {"pages": True},
        {"pages": [1, True]},
        {"pages": [float("inf")]},
        {"script_tags": None},
        {"password": 42},
        {"ocr": lambda *_: "text"},
        {"on_progress": lambda *_: None},
    ],
)
def test_unsupported_or_invalid_options_fail_before_native_call(
    monkeypatch: pytest.MonkeyPatch, options: object
) -> None:
    called = False

    def native_parse(*_args: object):
        nonlocal called
        called = True
        return _successful_wire()

    monkeypatch.setattr(_native, "try_parse_bytes", native_parse)

    with pytest.raises((TypeError, ValueError, NotImplementedError)) as caught:
        kordoc.try_parse(b"input", options=options)

    assert not called
    assert "private-value" not in str(caught.value)


def test_options_container_must_be_a_mapping() -> None:
    with pytest.raises(TypeError, match="mapping"):
        kordoc.try_parse(b"input", options=[("plain", True)])


def test_parse_options_enforce_conversion_budgets() -> None:
    with pytest.raises(ValueError, match="too many entries"):
        kordoc.try_parse(b"%PDF", options={"pages": [1.0] * 100_001})
    with pytest.raises(ValueError, match="length limit"):
        kordoc.try_parse(b"%PDF", options={"pages": "1" * 65_537})
    with pytest.raises(ValueError, match="length limit"):
        kordoc.try_parse(b"%PDF", options={"password": "x" * 65_537})


def test_native_parse_options_independently_enforce_conversion_budgets() -> None:
    with pytest.raises(ValueError, match="Invalid parser options"):
        _native.try_parse_bytes(b"%PDF", {"pages": [1.0] * 100_001})
    with pytest.raises(ValueError, match="Invalid parser options"):
        _native.try_parse_bytes(b"%PDF", {"password": "x" * 65_537})
    with pytest.raises(ValueError, match="Invalid parser options"):
        _native.try_parse_bytes(b"%PDF", {"unknown": "x" * 65_537})


def test_bytes_path_and_file_like_inputs_reach_native_as_identical_bytes(
    monkeypatch: pytest.MonkeyPatch, tmp_path: Path
) -> None:
    path = tmp_path / "input.docx"
    path.write_bytes(b"same input")
    observed: list[bytes] = []

    def native_parse(data: bytes):
        observed.append(data)
        return _successful_wire()

    monkeypatch.setattr(_native, "try_parse_bytes", native_parse)
    for value in (
        b"same input",
        bytearray(b"same input"),
        memoryview(b"same input"),
        BytesIO(b"same input"),
        path,
    ):
        assert kordoc.try_parse(value).success is True

    assert observed == [b"same input"] * 5


def test_parse_success_does_not_claim_the_production_parser_is_ready() -> None:
    result = kordoc.try_parse(b"%PDF-1.7\n")
    assert result.success is False
    assert result.code == "UNSUPPORTED_FORMAT"
