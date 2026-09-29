"""Deterministic first-difference reporting for JSON-compatible values."""

from __future__ import annotations

from dataclasses import dataclass
from typing import Any


class _MissingValue:
    def __repr__(self) -> str:
        return "<missing>"


MISSING = _MissingValue()


@dataclass(frozen=True, slots=True)
class JsonDifference:
    pointer: str
    expected: Any
    actual: Any


def _pointer_child(pointer: str, token: str) -> str:
    escaped = token.replace("~", "~0").replace("/", "~1")
    return f"{pointer}/{escaped}"


def compare_json(expected: Any, actual: Any) -> JsonDifference | None:
    """Return the first difference in key/index order, using RFC 6901 paths."""

    def compare(left: Any, right: Any, pointer: str) -> JsonDifference | None:
        if isinstance(left, dict) and isinstance(right, dict):
            for key in sorted(left.keys() | right.keys()):
                child_pointer = _pointer_child(pointer, key)
                if key not in left:
                    return JsonDifference(child_pointer, MISSING, right[key])
                if key not in right:
                    return JsonDifference(child_pointer, left[key], MISSING)
                difference = compare(left[key], right[key], child_pointer)
                if difference is not None:
                    return difference
            return None

        if isinstance(left, list) and isinstance(right, list):
            for index, (left_item, right_item) in enumerate(zip(left, right)):
                difference = compare(
                    left_item, right_item, _pointer_child(pointer, str(index))
                )
                if difference is not None:
                    return difference
            if len(left) != len(right):
                index = min(len(left), len(right))
                child_pointer = _pointer_child(pointer, str(index))
                if len(left) < len(right):
                    return JsonDifference(child_pointer, MISSING, right[index])
                return JsonDifference(child_pointer, left[index], MISSING)
            return None

        if type(left) is type(right) and left == right:
            return None
        return JsonDifference(pointer, left, right)

    return compare(expected, actual, "")
