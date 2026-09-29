"""Narrow, explicit normalizations for parity comparisons."""

from __future__ import annotations

from collections.abc import Collection
from typing import Any
from xml.etree import ElementTree


def _pointer_child(pointer: str, token: str) -> str:
    escaped = token.replace("~", "~0").replace("/", "~1")
    return f"{pointer}/{escaped}"


def _canonical_xml_attributes(xml: str, pointer: str) -> str:
    try:
        ElementTree.fromstring(xml)
    except ElementTree.ParseError as error:
        raise ValueError(f"invalid XML at {pointer}: {error}") from error

    result: list[str] = []
    cursor = 0
    while True:
        start = xml.find("<", cursor)
        if start < 0:
            result.append(xml[cursor:])
            break
        result.append(xml[cursor:start])
        if xml.startswith("<!--", start):
            end = xml.find("-->", start + 4)
            result.append(xml[start : end + 3])
            cursor = end + 3
            continue
        if xml.startswith("<![CDATA[", start):
            end = xml.find("]]>", start + 9)
            result.append(xml[start : end + 3])
            cursor = end + 3
            continue
        if xml.startswith("<?", start):
            end = xml.find("?>", start + 2)
            result.append(xml[start : end + 2])
            cursor = end + 2
            continue
        if xml.startswith("<!", start):
            end = _declaration_end(xml, start)
            result.append(xml[start : end + 1])
            cursor = end + 1
            continue
        end = _markup_end(xml, start)
        markup = xml[start : end + 1]
        result.append(_sort_start_tag_attributes(markup))
        cursor = end + 1
    return "".join(result)


def _markup_end(xml: str, start: int) -> int:
    quote: str | None = None
    for position in range(start + 1, len(xml)):
        character = xml[position]
        if quote is not None:
            if character == quote:
                quote = None
        elif character in "\"'":
            quote = character
        elif character == ">":
            return position
    raise ValueError("invalid XML: unterminated markup")


def _declaration_end(xml: str, start: int) -> int:
    quote: str | None = None
    bracket_depth = 0
    for position in range(start + 2, len(xml)):
        character = xml[position]
        if quote is not None:
            if character == quote:
                quote = None
        elif character in "\"'":
            quote = character
        elif character == "[":
            bracket_depth += 1
        elif character == "]":
            bracket_depth -= 1
        elif character == ">" and bracket_depth == 0:
            return position
    raise ValueError("invalid XML: unterminated declaration")


def _sort_start_tag_attributes(markup: str) -> str:
    if markup.startswith(("</", "<!", "<?")):
        return markup

    cursor = 1
    while (
        cursor < len(markup)
        and not markup[cursor].isspace()
        and markup[cursor] not in "/>"
    ):
        cursor += 1
    element_name = markup[1:cursor]
    if not element_name:
        return markup

    attributes: list[tuple[str, str]] = []
    separators: list[str] = []
    tail = ""
    while cursor < len(markup):
        whitespace_start = cursor
        while cursor < len(markup) and markup[cursor].isspace():
            cursor += 1
        whitespace = markup[whitespace_start:cursor]
        if cursor >= len(markup) - 1 or markup[cursor] in "/>":
            tail = whitespace + markup[cursor:]
            break

        attribute_start = cursor
        while (
            cursor < len(markup)
            and not markup[cursor].isspace()
            and markup[cursor] not in "=/>"
        ):
            cursor += 1
        attribute_name = markup[attribute_start:cursor]
        if not attribute_name:
            return markup
        while cursor < len(markup) and markup[cursor].isspace():
            cursor += 1
        if cursor >= len(markup) or markup[cursor] != "=":
            return markup
        cursor += 1
        while cursor < len(markup) and markup[cursor].isspace():
            cursor += 1
        if cursor >= len(markup) or markup[cursor] not in "\"'":
            return markup
        quote = markup[cursor]
        cursor += 1
        value_end = markup.find(quote, cursor)
        if value_end < 0:
            return markup
        cursor = value_end + 1
        separators.append(whitespace)
        attributes.append((attribute_name, markup[attribute_start:cursor]))

    if len(attributes) < 2:
        return markup
    sorted_attributes = sorted(attributes, key=lambda attribute: attribute[0])
    return (
        "<"
        + element_name
        + "".join(
            whitespace + attribute[1]
            for whitespace, attribute in zip(separators, sorted_attributes)
        )
        + tail
    )


def _validate_timestamp_pointer(pointer: str) -> None:
    if not pointer.startswith("/"):
        raise ValueError(
            "ZIP timestamp pointer must be a JSON Pointer ending in /timestamp"
        )
    terminal = pointer.rsplit("/", 1)[-1]
    terminal = terminal.replace("~1", "/").replace("~0", "~")
    if terminal != "timestamp":
        raise ValueError("ZIP timestamp pointer must end in /timestamp")


def normalize(
    value: Any,
    *,
    zip_timestamp_pointers: Collection[str] = frozenset(),
    xml_attribute_pointers: Collection[str] = frozenset(),
) -> Any:
    """Return a recursively ordered copy with only registered deltas removed.

    JSON Pointer registrations are exact paths. Arrays retain their order,
    text and whitespace are untouched, and XML is interpreted only where its
    pointer is explicitly registered.
    """
    timestamps = frozenset(zip_timestamp_pointers)
    xml_values = frozenset(xml_attribute_pointers)
    for pointer in timestamps:
        _validate_timestamp_pointer(pointer)

    def visit(current: Any, pointer: str) -> Any:
        if pointer in xml_values:
            if not isinstance(current, str):
                raise TypeError(f"registered XML value at {pointer} must be a string")
            return _canonical_xml_attributes(current, pointer)
        if isinstance(current, dict):
            normalized: dict[str, Any] = {}
            for key in sorted(current):
                if not isinstance(key, str):
                    raise TypeError("JSON object keys must be strings")
                child_pointer = _pointer_child(pointer, key)
                if child_pointer in timestamps:
                    continue
                normalized[key] = visit(current[key], child_pointer)
            return normalized
        if isinstance(current, list):
            return [
                visit(item, _pointer_child(pointer, str(index)))
                for index, item in enumerate(current)
            ]
        return current

    return visit(value, "")
