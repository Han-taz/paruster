from __future__ import annotations

import os
from collections.abc import Mapping
from math import isfinite
from pathlib import Path
from typing import Any, BinaryIO, cast

from . import _native
from ._errors import KordocError, OutputTooLargeError, typed_error_from_native
from ._models import TryParseResult

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
