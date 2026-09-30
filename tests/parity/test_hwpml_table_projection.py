from __future__ import annotations

import json
from pathlib import Path

import kordoc
import pytest

CAPTURES = (
    Path(__file__).parents[2]
    / "crates/kordoc-hancom/tests/support/hwpml/hwpml-oracle-results.jsonl"
)
CASES = (
    "nested_table_default",
    "nested_table_keep_trailing_empty_cols",
    "nested_table_drop_trailing_empty_cols",
)


@pytest.mark.parametrize("case_id", CASES)
def test_frozen_hwpml_nested_ir_retains_exact_core_markdown(case_id: str) -> None:
    records = [json.loads(line) for line in CAPTURES.read_text().splitlines()]
    result = next(row["result"] for row in records if row["case_id"] == case_id)
    assert result["success"] is True
    assert kordoc.blocks_to_markdown(result["blocks"]) == result["markdown"]
