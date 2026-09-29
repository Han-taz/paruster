import hashlib
import json
from importlib.util import find_spec
from pathlib import Path

import kordoc
import pytest

ROOT = Path(__file__).parents[1]


def _normalize():
    assert find_spec("tests.parity.normalize") is not None
    from tests.parity.normalize import normalize

    return normalize


def _compare_json():
    assert find_spec("tests.parity.compare") is not None
    from tests.parity.compare import compare_json

    return compare_json


def test_committed_golden_cases() -> None:
    manifest_path = ROOT / "golden/manifest.json"
    assert manifest_path.exists(), "the committed golden manifest is missing"
    manifest = json.loads(manifest_path.read_text(encoding="utf-8"))
    assert manifest["cases"]

    for case in manifest["cases"]:
        assert set(case) == {
            "id",
            "input",
            "expected",
            "license",
            "generator",
            "sha256",
            "covered_contract",
        }
        raw = (ROOT / "golden" / case["input"]).read_bytes()
        assert hashlib.sha256(raw).hexdigest() == case["sha256"]
        actual = {"fileType": kordoc.detect_format(raw)}
        expected = json.loads(
            (ROOT / "golden" / case["expected"]).read_text(encoding="utf-8")
        )
        compare_json = _compare_json()
        normalize = _normalize()
        difference = compare_json(normalize(expected), normalize(actual))
        assert difference is None


def test_first_difference_is_a_stable_json_pointer() -> None:
    compare_json = _compare_json()

    difference = compare_json(
        {"z": 0, "a": {"b": 1}}, {"a": {"b": 2}, "z": 0}
    )
    assert difference is not None
    assert difference.pointer == "/a/b"
    assert difference.expected == 1
    assert difference.actual == 2


def test_equal_objects_ignore_key_insertion_order() -> None:
    compare_json = _compare_json()

    assert compare_json({"b": 2, "a": 1}, {"a": 1, "b": 2}) is None


def test_missing_and_extra_object_keys_report_the_key_pointer() -> None:
    compare_json = _compare_json()

    missing = compare_json({"a": 1}, {})
    extra = compare_json({}, {"a": 1})
    assert missing is not None and missing.pointer == "/a"
    assert extra is not None and extra.pointer == "/a"


def test_list_length_mismatch_reports_the_first_missing_index() -> None:
    compare_json = _compare_json()

    difference = compare_json(["first"], ["first", "second"])
    assert difference is not None
    assert difference.pointer == "/1"


def test_root_difference_uses_the_empty_json_pointer() -> None:
    compare_json = _compare_json()

    difference = compare_json("expected", "actual")
    assert difference is not None
    assert difference.pointer == ""


def test_json_pointer_escapes_object_keys_per_rfc6901() -> None:
    compare_json = _compare_json()

    difference = compare_json({"a/b~c": 1}, {"a/b~c": 2})
    assert difference is not None
    assert difference.pointer == "/a~1b~0c"


def test_timestamp_is_removed_only_at_registered_pointer() -> None:
    normalize = _normalize()

    value = {"entries": [{"name": "item", "timestamp": "1980-01-01T00:00:00Z"}]}
    assert normalize(value) == value
    assert normalize(value, zip_timestamp_pointers={"/entries/0/timestamp"}) == {
        "entries": [{"name": "item"}]
    }


def test_normalize_sorts_object_keys_and_preserves_array_order() -> None:
    normalize = _normalize()

    value = {"z": 0, "items": [{"b": 2, "a": 1}, {"d": 4, "c": 3}]}
    normalized = normalize(value)
    assert list(normalized) == ["items", "z"]
    assert [list(item) for item in normalized["items"]] == [["a", "b"], ["c", "d"]]
    assert [item["a"] if "a" in item else item["c"] for item in normalized["items"]] == [1, 3]


def test_xml_normalization_reorders_attributes_only_at_registered_pointer() -> None:
    normalize = _normalize()

    left = {"xml": '<root z="2" a="1">text</root>'}
    right = {"xml": '<root a="1" z="2">text</root>'}
    assert normalize(left) != normalize(right)
    assert normalize(left, xml_attribute_pointers={"/xml"}) == normalize(
        right, xml_attribute_pointers={"/xml"}
    )


def test_xml_normalization_preserves_text_and_child_order() -> None:
    compare_json = _compare_json()
    normalize = _normalize()

    pointers = {"/xml"}
    left = {"xml": '<root z="2" a="1"><first/>text</root>'}
    changed_text = {"xml": '<root a="1" z="2"><first/>text </root>'}
    changed_children = {"xml": '<root a="1" z="2">text<first/></root>'}
    assert compare_json(
        normalize(left, xml_attribute_pointers=pointers),
        normalize(changed_text, xml_attribute_pointers=pointers),
    ) is not None
    assert compare_json(
        normalize(left, xml_attribute_pointers=pointers),
        normalize(changed_children, xml_attribute_pointers=pointers),
    ) is not None


def test_malformed_registered_xml_fails_clearly() -> None:
    normalize = _normalize()

    with pytest.raises(ValueError, match="invalid XML"):
        normalize({"xml": "<root>"}, xml_attribute_pointers={"/xml"})


def test_semantic_document_changes_are_never_normalized() -> None:
    compare_json = _compare_json()
    normalize = _normalize()

    cases = [
        ({"blocks": [{"text": "A"}]}, {"blocks": [{"text": "B"}]}, "/blocks/0/text"),
        (
            {"blocks": [{"type": "heading"}, {"type": "paragraph"}]},
            {"blocks": [{"type": "paragraph"}, {"type": "heading"}]},
            "/blocks/0/type",
        ),
        ({"rows": 2, "cols": 3}, {"rows": 3, "cols": 3}, "/rows"),
        ({"pages": [{"page": 1}]}, {"pages": [{"page": 2}]}, "/pages/0/page"),
        ({"warnings": ["W1"]}, {"warnings": ["W2"]}, "/warnings/0"),
        ({"error": {"code": "E1"}}, {"error": {"code": "E2"}}, "/error/code"),
        ({"quality": {"score": 0.9}}, {"quality": {"score": 0.8}}, "/quality/score"),
        ({"markdown": "line\n"}, {"markdown": "line"}, "/markdown"),
    ]
    for expected, actual, pointer in cases:
        difference = compare_json(normalize(expected), normalize(actual))
        assert difference is not None
        assert difference.pointer == pointer
