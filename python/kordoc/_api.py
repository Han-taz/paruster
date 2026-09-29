from __future__ import annotations

import os
from collections.abc import Mapping, Sequence
from math import isfinite
from pathlib import Path
from types import MappingProxyType
from typing import Any, BinaryIO, cast

from . import _native
from ._errors import KordocError, OutputTooLargeError, typed_error_from_native
from ._models import (
    ChunkOptions,
    DocChunk,
    PageMarkdown,
    TryParseResult,
    _freeze_wire,
)

MAX_INPUT_BYTES = _native.max_input_bytes()
EMPTY_INPUT_MESSAGE = "빈 버퍼이거나 유효하지 않은 입력입니다."

_OPTION_NAMES = {
    "pages": "pages",
    "ocr": "ocr",
    "remove_header_footer": "removeHeaderFooter",
    "script_tags": "scriptTags",
    "plain": "plain",
    "html_tables": "htmlTables",
    "keep_trailing_empty_cols": "keepTrailingEmptyCols",
    "classify_tables": "classifyTables",
    "keep_empty_paragraphs": "keepEmptyParagraphs",
    "include_field_placeholders": "includeFieldPlaceholders",
    "password": "password",
    "formula_ocr": "formulaOcr",
    "dedupe_running_headers": "dedupeRunningHeaders",
    "inline_images": "inlineImages",
    "images": "images",
    "tables": "tables",
}
_BOOLEAN_OPTIONS = frozenset(_OPTION_NAMES) - {"pages", "ocr", "password"}
_MAX_PAGE_SELECTION_ITEMS = 100_000
_MAX_OPTION_STRING_LENGTH = 65_536
MAX_MARKDOWN_BYTES = 256 * 1024 * 1024
_MAX_PROJECTION_DEPTH = 256
_MAX_PROJECTION_ITEMS = 16_000_000


def _normalize_projection_value(
    value: Any,
    budget: list[int],
    depth: int = 0,
    *,
    freeze: bool = False,
    copy: bool = True,
    native_containers: list[bool] | None = None,
) -> Any:
    if depth > _MAX_PROJECTION_DEPTH:
        raise OutputTooLargeError("Projection input exceeds the nesting limit")
    budget[0] += 1
    if budget[0] > _MAX_PROJECTION_ITEMS:
        raise OutputTooLargeError("Projection input has too many values")
    budget[1] += 1
    if budget[1] > budget[2]:
        raise OutputTooLargeError("Projection input exceeds the byte limit")
    if value is None:
        raise TypeError("projection values cannot be None; omit optional fields")
    if isinstance(value, str):
        budget[1] += len(value.encode("utf-8"))
        if budget[1] > budget[2]:
            raise OutputTooLargeError("Projection input exceeds the byte limit")
        return value
    if isinstance(value, bool | int):
        budget[1] += 8
        if budget[1] > budget[2]:
            raise OutputTooLargeError("Projection input exceeds the byte limit")
        return value
    if isinstance(value, float):
        if not isfinite(value):
            raise TypeError("projection numbers must be finite")
        budget[1] += 8
        if budget[1] > budget[2]:
            raise OutputTooLargeError("Projection input exceeds the byte limit")
        return value
    if isinstance(value, Mapping):
        if native_containers is not None and type(value) is not dict:
            native_containers[0] = False
        if len(value) > _MAX_PROJECTION_ITEMS or any(
            not isinstance(key, str) for key in value
        ):
            raise TypeError("projection objects must have string keys")
        if native_containers is not None and any(type(key) is not str for key in value):
            native_containers[0] = False
        output: dict[str, Any] | None = {} if copy else None
        for key, item in value.items():
            budget[1] += len(key.encode("utf-8"))
            if budget[1] > budget[2]:
                raise OutputTooLargeError("Projection input exceeds the byte limit")
            normalized_item = _normalize_projection_value(
                item,
                budget,
                depth + 1,
                freeze=freeze,
                copy=copy,
                native_containers=native_containers,
            )
            if output is not None:
                output[key] = normalized_item
        if output is None:
            return value
        return MappingProxyType(output) if freeze else output
    if isinstance(value, Sequence) and not isinstance(value, (str, bytes, bytearray)):
        if native_containers is not None and type(value) not in {list, tuple}:
            native_containers[0] = False
        if len(value) > _MAX_PROJECTION_ITEMS:
            raise OutputTooLargeError("Projection input has too many values")
        sequence_output = (
            [
                _normalize_projection_value(
                    item,
                    budget,
                    depth + 1,
                    freeze=freeze,
                    copy=copy,
                    native_containers=native_containers,
                )
                for item in value
            ]
            if copy
            else None
        )
        if sequence_output is None:
            for item in value:
                _normalize_projection_value(
                    item,
                    budget,
                    depth + 1,
                    freeze=freeze,
                    copy=False,
                    native_containers=native_containers,
                )
            return value
        return tuple(sequence_output) if freeze else sequence_output
    raise TypeError(f"unsupported projection value: {type(value).__name__}")


def _normalize_blocks_with_budget(
    value: object, max_bytes: int
) -> list[dict[str, Any]]:
    if not isinstance(value, Sequence) or isinstance(value, (str, bytes, bytearray)):
        raise TypeError("blocks must be a sequence of mappings")
    if len(value) > _MAX_PROJECTION_ITEMS:
        raise OutputTooLargeError("Projection input has too many blocks")
    budget = [0, 0, max_bytes]
    native_containers = [type(value) in {list, tuple}]
    for block in value:
        if not isinstance(block, Mapping):
            raise TypeError("each block must be a mapping")
        _normalize_projection_value(
            block, budget, copy=False, native_containers=native_containers
        )
    if native_containers[0]:
        return value if type(value) is list else list(value)
    return [_materialize_projection_value(block) for block in value]


def _materialize_projection_value(value: Any) -> Any:
    if isinstance(value, Mapping):
        return {key: _materialize_projection_value(item) for key, item in value.items()}
    if isinstance(value, Sequence) and not isinstance(value, (str, bytes, bytearray)):
        return [_materialize_projection_value(item) for item in value]
    return value


def _normalize_blocks(value: object) -> list[dict[str, Any]]:
    return _normalize_blocks_with_budget(value, MAX_MARKDOWN_BYTES)


def _validate_projection_output(value: Any, *, freeze: bool = False) -> Any:
    return _normalize_projection_value(
        value, [0, 0, MAX_MARKDOWN_BYTES], freeze=freeze, copy=freeze
    )


def _translate_projection_error(error: ValueError) -> None:
    translated = typed_error_from_native(error)
    if translated is not None:
        raise translated from error
    raise error


def blocks_to_markdown(blocks: object) -> str:
    """Render an IR block sequence to Markdown."""
    normalized = _normalize_blocks(blocks)
    try:
        result = _native.blocks_to_markdown_wire(normalized)
    except ValueError as error:
        _translate_projection_error(error)
    if not isinstance(result, str):
        raise TypeError("native Markdown projection must return a string")
    if len(result.encode("utf-8")) > MAX_MARKDOWN_BYTES:
        raise OutputTooLargeError("Markdown output exceeds the byte limit")
    return result


def blocks_to_pages(
    blocks: object, render: Any = None
) -> tuple[PageMarkdown, ...] | None:
    """Render page Markdown, optionally using a caller-provided synchronous renderer."""
    normalized = _normalize_blocks(blocks)
    if render is not None and not callable(render):
        raise TypeError("render must be callable or None")
    original_exception: list[BaseException] = []
    native_renderer = None
    if render is not None:

        def native_renderer(page_blocks: Any) -> str:
            try:
                if not isinstance(page_blocks, Sequence) or isinstance(
                    page_blocks, (str, bytes, bytearray)
                ):
                    raise TypeError(
                        "page renderer input must be a sequence of mappings"
                    )
                frozen = tuple(_freeze_wire(block) for block in page_blocks)
                if any(not isinstance(block, Mapping) for block in frozen):
                    raise TypeError("page renderer input must contain mappings")
                result = render(frozen)
                if not isinstance(result, str):
                    raise TypeError("render callback must return a string")
                return result
            except BaseException as error:
                original_exception.append(error)
                raise

    try:
        if native_renderer is None:
            result = _native.blocks_to_pages_wire(normalized)
        else:
            result = _native.blocks_to_pages_wire(normalized, native_renderer)
    except BaseException as error:
        if original_exception:
            raise original_exception[0]
        if isinstance(error, ValueError):
            _translate_projection_error(error)
        raise
    if result is None:
        return None
    if not isinstance(result, Sequence) or isinstance(result, (str, bytes, bytearray)):
        raise TypeError("native pages projection must return a sequence or None")
    normalized_result = _validate_projection_output(result)
    return tuple(PageMarkdown.from_dict(item) for item in normalized_result)


def blocks_to_chunks(
    blocks: object, options: ChunkOptions | Mapping[str, Any] | None = None
) -> tuple[DocChunk, ...]:
    """Project IR blocks into typed structural chunks."""
    normalized = _normalize_blocks(blocks)
    if options is None:
        native_options = None
    elif isinstance(options, ChunkOptions):
        native_options = options.to_dict()
    elif isinstance(options, Mapping):
        native_options = ChunkOptions.from_dict(options).to_dict()
    else:
        raise TypeError("options must be ChunkOptions, a mapping, or None")
    try:
        result = _native.blocks_to_chunks_wire(normalized, native_options)
    except ValueError as error:
        _translate_projection_error(error)
    if not isinstance(result, Sequence) or isinstance(result, (str, bytes, bytearray)):
        raise TypeError("native chunks projection must return a sequence")
    normalized_result = _validate_projection_output(result)
    return tuple(DocChunk.from_dict(item) for item in normalized_result)


def _classify_tables(blocks: object) -> tuple[Mapping[str, Any], ...]:
    """Return deeply immutable IR blocks with table policy classifications."""
    normalized = _normalize_blocks(blocks)
    try:
        result = _native.classify_tables_wire(normalized)
    except ValueError as error:
        _translate_projection_error(error)
    if not isinstance(result, Sequence) or isinstance(result, (str, bytes, bytearray)):
        raise TypeError("native table classification must return a sequence")
    normalized_result = _validate_projection_output(result, freeze=True)
    return tuple(normalized_result)


def _is_finite_number(value: float) -> bool:
    try:
        return isfinite(value)
    except OverflowError:
        return False


def _normalize_options(value: object | None) -> dict[str, object] | None:
    if value is None:
        return None
    if not isinstance(value, Mapping):
        raise TypeError("options must be a mapping or None")
    if len(value) > len(_OPTION_NAMES) + 1:
        raise ValueError("too many parse options")
    if any(not isinstance(key, str) for key in value):
        raise TypeError("parse option names must be strings")
    if "file_path" in value:
        raise ValueError("parse option 'file_path' is internal and unsupported")
    unknown = set(value) - _OPTION_NAMES.keys() - {"on_progress"}
    if unknown:
        raise ValueError(f"unknown parse option: {min(unknown)!r}")

    output: dict[str, object] = {}
    for name, item in value.items():
        if name == "on_progress":
            raise NotImplementedError(
                "parse option 'on_progress' callback is not implemented"
            )
        if name == "ocr" and callable(item):
            raise NotImplementedError("parse option 'ocr' callback is not implemented")
        if item is None:
            raise TypeError(f"parse option {name!r} cannot be None; omit it instead")
        if name in _BOOLEAN_OPTIONS:
            if type(item) is not bool:
                raise TypeError(f"parse option {name!r} must be a boolean")
        elif name == "password":
            if not isinstance(item, str):
                raise TypeError("parse option 'password' must be a string")
            if len(item) > _MAX_OPTION_STRING_LENGTH:
                raise ValueError("parse option 'password' exceeds the length limit")
        elif name == "ocr":
            if type(item) is not bool and not (type(item) is str and item == "force"):
                raise TypeError("parse option 'ocr' must be a boolean or 'force'")
        elif name == "pages":
            if isinstance(item, str):
                if len(item) > _MAX_OPTION_STRING_LENGTH:
                    raise ValueError("parse option 'pages' exceeds the length limit")
            elif isinstance(item, (list, tuple)):
                if len(item) > _MAX_PAGE_SELECTION_ITEMS:
                    raise ValueError("parse option 'pages' has too many entries")
                if any(
                    isinstance(page, bool)
                    or not isinstance(page, (int, float))
                    or not _is_finite_number(page)
                    for page in item
                ):
                    raise TypeError("parse option 'pages' must contain finite numbers")
                item = list(item)
            else:
                raise TypeError(
                    "parse option 'pages' must be a range string or number sequence"
                )
        output[_OPTION_NAMES[name]] = item
    return output


def _file_not_found() -> KordocError:
    error = typed_error_from_native(
        ValueError("FILE_NOT_FOUND", "Input file was not found or could not be opened")
    )
    assert error is not None
    return error


def _normalize_input(value: Any, *, max_bytes: int | None = None) -> bytes:
    if max_bytes is None:
        max_bytes = MAX_INPUT_BYTES
    if isinstance(value, bytes):
        if len(value) > max_bytes:
            raise OutputTooLargeError("Input exceeds the configured byte limit")
        return value
    if isinstance(value, bytearray):
        if len(value) > max_bytes:
            raise OutputTooLargeError("Input exceeds the configured byte limit")
        return bytes(value)
    if isinstance(value, memoryview):
        if value.nbytes > max_bytes:
            raise OutputTooLargeError("Input exceeds the configured byte limit")
        return value.tobytes()

    if isinstance(value, (str, os.PathLike)):
        try:
            path = Path(os.fsdecode(os.fspath(value)))
            if path.stat().st_size > max_bytes:
                raise OutputTooLargeError("Input exceeds the configured byte limit")
            with path.open("rb") as file:
                return _bounded_read(file, max_bytes)
        except FileNotFoundError as error:
            raise _file_not_found() from error
        except OSError as error:
            raise _file_not_found() from error

    read = getattr(value, "read", None)
    if callable(read):
        return _bounded_read(value, max_bytes)
    raise TypeError(
        "input must be a path, bytes-like object, or binary file-like object"
    )


def _bounded_read(file: BinaryIO, max_bytes: int) -> bytes:
    content = bytearray()
    while len(content) <= max_bytes:
        requested = max_bytes + 1 - len(content)
        chunk = file.read(requested)
        if isinstance(chunk, str):
            raise TypeError("file-like input must return bytes, not text")
        if not isinstance(chunk, (bytes, bytearray, memoryview)):
            raise TypeError("file-like input read() must return bytes")
        chunk_size = chunk.nbytes if isinstance(chunk, memoryview) else len(chunk)
        if chunk_size > requested:
            raise OutputTooLargeError("Input exceeds the configured byte limit")
        if not chunk_size:
            break
        content.extend(bytes(chunk))
    if len(content) > max_bytes:
        raise OutputTooLargeError("Input exceeds the configured byte limit")
    return bytes(content)


def detect_format(value: Any) -> str:
    data = _normalize_input(value)
    try:
        return _native.detect_format_bytes(data)
    except ValueError as error:
        translated = typed_error_from_native(error)
        if translated is None:
            raise
        raise translated from error


def detect_zip_format(value: Any) -> str:
    return _native.detect_zip_format_bytes(_normalize_input(value))


def detect_ole2_format(value: Any) -> str:
    return _native.detect_ole2_format_bytes(_normalize_input(value))


def is_zip_file(value: Any) -> bool:
    return _native.is_zip_file_bytes(_normalize_input(value))


def is_hwpx_file(value: Any) -> bool:
    return _native.is_hwpx_file_bytes(_normalize_input(value))


def is_old_hwp_file(value: Any) -> bool:
    return _native.is_old_hwp_file_bytes(_normalize_input(value))


def is_pdf_file(value: Any) -> bool:
    return _native.is_pdf_file_bytes(_normalize_input(value))


def try_parse(value: Any, options: object | None = None) -> TryParseResult:
    native_options = _normalize_options(options)
    try:
        data = _normalize_input(value)
    except KordocError as error:
        return TryParseResult.from_dict(
            {
                "success": False,
                "fileType": "unknown",
                "error": error.message,
                "code": error.code,
            }
        )
    if native_options is None:
        native_result = _native.try_parse_bytes(data)
    else:
        native_result = _native.try_parse_bytes(data, cast(Any, native_options))
    return TryParseResult.from_dict(native_result)


def parse(value: Any, options: object | None = None) -> TryParseResult:
    """Parse input, raising a typed error on failure.

    A successful result remains the serializable :class:`TryParseResult` and
    exposes its immutable :class:`Document` projection through ``.document``.
    """
    result = try_parse(value, options=options)
    if result.success:
        return result
    error = (
        typed_error_from_native(ValueError(result.code, result.error))
        if result.code
        else None
    )
    if error is None:
        from ._errors import ParseError

        error = ParseError(result.error or EMPTY_INPUT_MESSAGE)
    raise error
