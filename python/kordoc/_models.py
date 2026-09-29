from __future__ import annotations

from collections.abc import Mapping, Sequence
from dataclasses import dataclass, fields
from math import isfinite
from types import MappingProxyType
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
            if not isinstance(item, Mapping):
                raise TypeError(f"{key} must be an object")
        elif key in {
            "blocks",
            "outline",
            "warnings",
            "images",
            "pages",
            "pageQuality",
        } and (
            not isinstance(item, (list, tuple))
            or any(not isinstance(element, Mapping) for element in item)
        ):
            raise TypeError(f"{key} must be an array of objects")


def _freeze_wire(
    value: Any, *, image_object: bool = False, image_array: bool = False
) -> Any:
    if isinstance(value, bytes | bytearray | memoryview):
        return bytes(value)
    if isinstance(value, Mapping):
        if any(not isinstance(key, str) for key in value):
            raise TypeError("wire object keys must be strings")
        return MappingProxyType(
            {
                key: _freeze_wire(
                    item,
                    image_object=(key == "imageData" or image_array),
                    image_array=(key == "images"),
                )
                if key != "data" or not image_object
                else _freeze_byte_array(item)
                for key, item in value.items()
            }
        )
    if isinstance(value, (list, tuple)):
        return tuple(_freeze_wire(item, image_object=image_array) for item in value)
    if value is None:
        raise ValueError("wire values must omit null fields")
    if isinstance(value, (str, bool, int)):
        return value
    if isinstance(value, float) and isfinite(value):
        return value
    raise TypeError(f"unsupported wire value type: {type(value).__name__}")


def _freeze_byte_array(value: Any) -> bytes:
    if isinstance(value, bytes | bytearray | memoryview):
        return bytes(value)
    if not isinstance(value, (list, tuple)):
        raise TypeError("image data must be bytes or an array of byte integers")
    if any(type(item) is not int or not 0 <= item <= 255 for item in value):
        raise TypeError("image data must contain unsigned byte integers")
    return bytes(value)


def _thaw_wire(value: Any) -> Any:
    if isinstance(value, bytes | bytearray | memoryview):
        return list(bytes(value))
    if isinstance(value, Mapping):
        return {key: _thaw_wire(item) for key, item in value.items()}
    if isinstance(value, (list, tuple)):
        return [_thaw_wire(item) for item in value]
    return value


def _validate_wire(value: Mapping[str, Any], valid_keys: set[str]) -> None:
    if any(not isinstance(key, str) for key in value):
        raise TypeError("wire object keys must be strings")
    unknown = set(value) - valid_keys
    if unknown:
        raise ValueError(f"unknown parse result fields: {sorted(unknown)!r}")
    for item in value.values():
        _freeze_wire(item)
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
        if "blocks" not in value or not isinstance(value["blocks"], (list, tuple)):
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


@dataclass(frozen=True, slots=True)
class TryParseResult:
    """Serializable success/failure envelope returned by :func:`try_parse`."""

    success: bool
    file_type: str
    markdown: str | None = None
    blocks: Sequence[Mapping[str, Any]] | None = None
    error: str | None = None
    code: str | None = None
    page_count: int | None = None
    is_image_based: bool | None = None
    metadata: Mapping[str, Any] | None = None
    outline: Sequence[Mapping[str, Any]] | None = None
    warnings: Sequence[Mapping[str, Any]] | None = None
    images: Sequence[Mapping[str, Any]] | None = None
    pages: Sequence[Mapping[str, Any]] | None = None
    page_quality: Sequence[Mapping[str, Any]] | None = None
    quality_summary: Mapping[str, Any] | None = None

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

    def __post_init__(self) -> None:
        wire = self._wire_mapping()
        _validate_wire(wire, self._KEYS)
        for field in fields(self):
            item = getattr(self, field.name)
            if (
                field.name
                in {
                    "blocks",
                    "metadata",
                    "outline",
                    "warnings",
                    "images",
                    "pages",
                    "page_quality",
                    "quality_summary",
                }
                and item is not None
            ):
                object.__setattr__(
                    self,
                    field.name,
                    _freeze_wire(item, image_array=field.name == "images"),
                )

    def _wire_mapping(self) -> dict[str, Any]:
        mapping = {
            "file_type": "fileType",
            "page_count": "pageCount",
            "is_image_based": "isImageBased",
            "page_quality": "pageQuality",
            "quality_summary": "qualitySummary",
        }
        output: dict[str, Any] = {}
        for field in fields(self):
            item = getattr(self, field.name)
            if item is not None or field.name in {"success", "file_type"}:
                output[mapping.get(field.name, field.name)] = item
        return output

    @classmethod
    def from_dict(cls, value: Mapping[str, Any]) -> TryParseResult:
        if not isinstance(value, Mapping):
            raise TypeError("parse result must be an object")
        _validate_wire(value, cls._KEYS)

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
        wire = self._wire_mapping()
        _validate_wire(wire, self._KEYS)
        return {key: _thaw_wire(value) for key, value in wire.items()}

    @property
    def document(self) -> Document | None:
        """Typed immutable document projection, available only for success."""
        if not self.success:
            return None
        return Document.from_result(self)


@dataclass(frozen=True, slots=True)
class Document:
    """Immutable projection of the successful parse-result document fields."""

    file_type: str
    markdown: str
    blocks: Sequence[Mapping[str, Any]]
    page_count: int | None = None
    is_image_based: bool | None = None
    metadata: Mapping[str, Any] | None = None
    outline: Sequence[Mapping[str, Any]] | None = None
    warnings: Sequence[Mapping[str, Any]] | None = None
    images: Sequence[Mapping[str, Any]] | None = None
    pages: Sequence[Mapping[str, Any]] | None = None
    page_quality: Sequence[Mapping[str, Any]] | None = None
    quality_summary: Mapping[str, Any] | None = None

    _FIELD_NAMES: ClassVar[dict[str, str]] = {
        "fileType": "file_type",
        "pageCount": "page_count",
        "isImageBased": "is_image_based",
        "pageQuality": "page_quality",
        "qualitySummary": "quality_summary",
    }

    def __post_init__(self) -> None:
        validated = TryParseResult(
            success=True,
            file_type=self.file_type,
            markdown=self.markdown,
            blocks=self.blocks,
            page_count=self.page_count,
            is_image_based=self.is_image_based,
            metadata=self.metadata,
            outline=self.outline,
            warnings=self.warnings,
            images=self.images,
            pages=self.pages,
            page_quality=self.page_quality,
            quality_summary=self.quality_summary,
        )
        for name in (
            "blocks",
            "metadata",
            "outline",
            "warnings",
            "images",
            "pages",
            "page_quality",
            "quality_summary",
        ):
            object.__setattr__(self, name, getattr(validated, name))

    @classmethod
    def from_result(cls, result: TryParseResult) -> Document:
        if not result.success:
            raise ValueError("a failed parse result has no document")
        assert result.markdown is not None and result.blocks is not None
        return cls(
            file_type=result.file_type,
            markdown=result.markdown,
            blocks=result.blocks,
            page_count=result.page_count,
            is_image_based=result.is_image_based,
            metadata=result.metadata,
            outline=result.outline,
            warnings=result.warnings,
            images=result.images,
            pages=result.pages,
            page_quality=result.page_quality,
            quality_summary=result.quality_summary,
        )

    def to_dict(self) -> dict[str, Any]:
        output = TryParseResult(
            success=True,
            file_type=self.file_type,
            markdown=self.markdown,
            blocks=self.blocks,
            page_count=self.page_count,
            is_image_based=self.is_image_based,
            metadata=self.metadata,
            outline=self.outline,
            warnings=self.warnings,
            images=self.images,
            pages=self.pages,
            page_quality=self.page_quality,
            quality_summary=self.quality_summary,
        ).to_dict()
        output.pop("success")
        return output

    def __getattr__(self, name: str) -> Any:
        """Support the frozen wire's camelCase names alongside Python names."""
        if name in self._FIELD_NAMES:
            return object.__getattribute__(self, self._FIELD_NAMES[name])
        raise AttributeError(name)
