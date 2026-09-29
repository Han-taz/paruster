"""Table policy helpers exposed by the frozen Python API."""

from __future__ import annotations

from collections.abc import Mapping, Sequence
from typing import Any

from ._api import _classify_tables


def classify_table_tree(
    blocks: Sequence[Mapping[str, Any]],
) -> tuple[Mapping[str, Any], ...]:
    """Return deeply immutable blocks with recursive table classifications."""
    return _classify_tables(blocks)


__all__ = ["classify_table_tree"]
