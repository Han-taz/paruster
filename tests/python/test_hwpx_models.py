from __future__ import annotations

import json

import kordoc
import pytest


def test_validation_models_keep_wire_keys_omission_order_and_immutability() -> None:
    wire = {
        "ok": False,
        "issues": [
            {"message": "first"},
            {"path": "Contents/header.xml", "message": "second"},
        ],
        "entryCount": 6,
    }
    result = kordoc.ValidateResult.from_dict(wire)
    assert isinstance(result.issues, tuple)
    assert isinstance(result.issues[0], kordoc.ValidateIssue)
    assert result.entry_count == 6
    assert result.issues[0].path is None
    assert result.to_dict() == wire
    assert json.loads(json.dumps(result.to_dict())) == wire
    assert not hasattr(result, "__dict__")
    assert not hasattr(result.issues[0], "__dict__")
    with pytest.raises(AttributeError):
        result.ok = True
    with pytest.raises(AttributeError):
        result.issues[0].message = "changed"


@pytest.mark.parametrize(
    "wire",
    [
        {"ok": 1, "issues": [], "entryCount": 0},
        {"ok": True, "issues": [], "entryCount": True},
        {"ok": True, "issues": [], "entryCount": -1},
        {"ok": True, "issues": [], "entryCount": 0.5},
        {"ok": True, "issues": [], "entryCount": None},
        {"ok": True, "issues": None, "entryCount": 0},
        {"ok": True, "issues": [{"message": 3}], "entryCount": 0},
        {"ok": True, "issues": [{"message": "x", "path": None}], "entryCount": 0},
        {"ok": True, "issues": [{"message": "x", "other": 2}], "entryCount": 0},
        {"ok": True, "issues": [], "entryCount": 0, "other": 2},
        {"ok": True, "entryCount": 0},
    ],
)
def test_validation_models_reject_non_contract_values(wire: object) -> None:
    with pytest.raises((TypeError, ValueError)):
        kordoc.ValidateResult.from_dict(wire)


def test_validation_models_copy_mutable_input() -> None:
    issues = [{"message": "first"}]
    result = kordoc.ValidateResult.from_dict(
        {"ok": False, "issues": issues, "entryCount": 1}
    )
    issues[0]["message"] = "changed"
    issues.append({"message": "later"})
    assert result.to_dict() == {
        "ok": False,
        "issues": [{"message": "first"}],
        "entryCount": 1,
    }
