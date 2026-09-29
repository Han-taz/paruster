from __future__ import annotations

from collections.abc import Mapping
from dataclasses import dataclass, fields
from math import isfinite
from typing import Any, ClassVar

from ._errors import _ERROR_TYPES

_FILE_TYPES = {
    "hwpx",
    "hwp",
    "hwp3",
    "hwpml",
    "pdf",
    "xlsx",
    "xls",
    "docx",
    "pptx",
    "image",
    "unknown",
}
_ERROR_CODES = frozenset(_ERROR_TYPES)


def _check_optional_fields(value: Mapping[str, Any]) -> None:
    for key, item in value.items():
        if key in {"success", "fileType", "markdown", "error", "code"}:
            continue
        if key == "pageCount":
            if type(item) is not int or not 0 <= item <= 2**32 - 1:
                raise TypeError("pageCount must be an unsigned 32-bit integer")
        elif key == "isImageBased":
            if type(item) is not bool:
                raise TypeError("isImageBased must be a boolean")
        elif key in {"metadata", "qualitySummary"}:
            if not isinstance(item, dict):
                raise TypeError(f"{key} must be an object")
        elif key in {
            "blocks",
            "outline",
            "warnings",
            "images",
            "pages",
            "pageQuality",
        } and (
            not isinstance(item, list)
            or any(not isinstance(element, dict) for element in item)
        ):
            raise TypeError(f"{key} must be an array of objects")


def _json_value(value: Any) -> Any:
    if isinstance(value, bytes | bytearray | memoryview):
        return list(bytes(value))
    if isinstance(value, dict):
        if any(not isinstance(key, str) for key in value):
            raise TypeError("wire object keys must be strings")
        return {key: _json_value(item) for key, item in value.items()}
    if isinstance(value, list):
        return [_json_value(item) for item in value]
    if value is None:
        raise ValueError("wire values must omit null fields")
    if isinstance(value, (str, bool, int)):
        return value
    if isinstance(value, float) and isfinite(value):
        return value
    raise TypeError(f"unsupported wire value type: {type(value).__name__}")


@dataclass(frozen=True, slots=True)
class TryParseResult:
    """Serializable success/failure envelope returned by :func:`try_parse`."""

    success: bool
    file_type: str
    markdown: str | None = None
    blocks: list[dict[str, Any]] | None = None
    error: str | None = None
    code: str | None = None
    page_count: int | None = None
    is_image_based: bool | None = None
    metadata: dict[str, Any] | None = None
    outline: list[dict[str, Any]] | None = None
    warnings: list[dict[str, Any]] | None = None
    images: list[dict[str, Any]] | None = None
    pages: list[dict[str, Any]] | None = None
    page_quality: list[dict[str, Any]] | None = None
    quality_summary: dict[str, Any] | None = None

    _KEYS: ClassVar[set[str]] = {
        "success",
        "fileType",
        "markdown",
        "blocks",
        "error",
        "code",
        "pageCount",
        "isImageBased",
        "metadata",
        "outline",
        "warnings",
        "images",
        "pages",
        "pageQuality",
        "qualitySummary",
    }

    @classmethod
    def from_dict(cls, value: Mapping[str, Any]) -> TryParseResult:
        if not isinstance(value, dict):
            raise TypeError("parse result must be an object")
        unknown = set(value) - cls._KEYS
        if unknown:
            raise ValueError(f"unknown parse result fields: {sorted(unknown)!r}")
        if any(item is None for item in value.values()):
            raise ValueError("optional parse result fields must be omitted, not null")
        if type(value.get("success")) is not bool or not isinstance(
            value.get("fileType"), str
        ):
            raise TypeError("parse result requires boolean success and string fileType")
        if value["fileType"] not in _FILE_TYPES:
            raise ValueError("fileType is not a contract value")
        _check_optional_fields(value)

        if value["success"]:
            if "markdown" not in value or not isinstance(value["markdown"], str):
                raise ValueError("successful result requires string markdown")
            if "blocks" not in value or not isinstance(value["blocks"], list):
                raise ValueError("successful result requires blocks array")
            if "error" in value or "code" in value:
                raise ValueError("successful result cannot contain error fields")
        else:
            if "error" not in value or not isinstance(value["error"], str):
                raise ValueError("failed result requires string error")
            if any(
                key in value
                for key in (
                    "markdown",
                    "blocks",
                    "metadata",
                    "outline",
                    "warnings",
                    "images",
                    "pages",
                    "pageQuality",
                    "qualitySummary",
                )
            ):
                raise ValueError("failed result cannot contain success fields")
            if "code" in value and value["code"] not in _ERROR_CODES:
                raise ValueError("code is not a contract value")

        mapping: dict[str, str] = {
            "fileType": "file_type",
            "pageCount": "page_count",
            "isImageBased": "is_image_based",
            "pageQuality": "page_quality",
            "qualitySummary": "quality_summary",
        }
        kwargs: dict[str, Any] = {}
        for key, item in value.items():
            target = mapping[key] if key in mapping else key  # noqa: SIM401
            kwargs[target] = item
        return cls(**kwargs)

    def to_dict(self) -> dict[str, Any]:
        mapping = {
            "file_type": "fileType",
            "page_count": "pageCount",
            "is_image_based": "isImageBased",
            "page_quality": "pageQuality",
            "quality_summary": "qualitySummary",
        }
        output: dict[str, Any] = {}
        for field in fields(self):
            if field.name.startswith("_"):
                continue
            value = getattr(self, field.name)
            if value is not None:
                output[mapping.get(field.name, field.name)] = _json_value(value)
        return output
