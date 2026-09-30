from __future__ import annotations

from pathlib import Path

import kordoc
import pytest

from tests.python.test_hwpx_api import _hwpx_bytes, _section


def _body_section(body: str) -> bytes:
    return (
        '<hs:sec xmlns:hs="http://www.hancom.co.kr/hwpml/2011/section" '
        'xmlns:hp="http://www.hancom.co.kr/hwpml/2011/paragraph">'
        f"{body}</hs:sec>"
    ).encode()


@pytest.mark.parametrize(
    "parse_api", [kordoc.parse, kordoc.try_parse, kordoc.parse_hwpx]
)
def test_script_tag_option_reaches_recursive_ir_and_projections(parse_api) -> None:
    data = _hwpx_bytes((_section("H&lt;sub&gt;2&lt;/sub&gt;O"),), image=False)
    default = parse_api(data).document
    preserved = parse_api(data, options={"script_tags": True}).document
    stripped = parse_api(data, options={"script_tags": False}).document
    assert default is not None and preserved is not None and stripped is not None
    assert default.to_dict() == preserved.to_dict()
    assert "<sub>2</sub>" in default.markdown
    assert stripped.markdown == "H2O"
    assert stripped.blocks[0]["text"] == "H2O"
    assert all("<sub>" not in page["markdown"] for page in stripped.pages or ())


@pytest.mark.parametrize(
    "parse_api", [kordoc.parse, kordoc.try_parse, kordoc.parse_hwpx]
)
def test_plain_option_removes_placeholders_without_altering_ir(parse_api) -> None:
    data = _hwpx_bytes()
    default = parse_api(data).document
    explicit_false = parse_api(data, options={"plain": False}).document
    plain = parse_api(data, options={"plain": True}).document
    assert default is not None and explicit_false is not None and plain is not None
    assert default.to_dict() == explicit_false.to_dict()
    assert "![" in default.markdown
    assert "![" not in plain.markdown
    assert plain.blocks == default.blocks
    assert plain.images == default.images
    assert "First section" in plain.markdown and "Second section" in plain.markdown
    assert all("![" not in page["markdown"] for page in plain.pages or ())


@pytest.mark.parametrize(
    "parse_api", [kordoc.parse, kordoc.try_parse, kordoc.parse_hwpx]
)
def test_html_table_option_preserves_ir_and_prettifies_nested_tables(parse_api) -> None:
    data = (
        Path(__file__).parents[2]
        / "tests/golden/document/hwpx/fixtures/nested_table.hwpx"
    )
    default = parse_api(data).document
    explicit_false = parse_api(data, options={"html_tables": False}).document
    html = parse_api(data, options={"html_tables": True}).document
    assert default is not None and explicit_false is not None and html is not None
    assert default.to_dict() == explicit_false.to_dict()
    assert html.blocks == default.blocks
    assert "<table>\n" in html.markdown
    assert "\n <tr>\n" in html.markdown
    assert "| ---" not in html.markdown


@pytest.mark.parametrize(
    "parse_api", [kordoc.parse, kordoc.try_parse, kordoc.parse_hwpx]
)
def test_field_guide_option_hides_markdown_while_preserving_body_text(
    parse_api,
) -> None:
    section = _body_section(
        "<hp:p><hp:run><hp:t>Label </hp:t></hp:run><hp:ctrl>"
        '<hp:fieldBegin type="CLICK_HERE" dirty="0"><hp:parameters>'
        '<hp:stringParam name="Direction">Enter name</hp:stringParam>'
        "</hp:parameters></hp:fieldBegin></hp:ctrl>"
        "<hp:run><hp:t>Enter name</hp:t></hp:run>"
        "<hp:ctrl><hp:fieldEnd/></hp:ctrl></hp:p>"
    )
    data = _hwpx_bytes((section,), image=False)
    default = parse_api(data).document
    explicit_false = parse_api(
        data, options={"include_field_placeholders": False}
    ).document
    included = parse_api(data, options={"include_field_placeholders": True}).document
    assert default is not None and explicit_false is not None and included is not None
    assert default.to_dict() == explicit_false.to_dict()
    assert default.markdown == "Label"
    assert included.markdown == "Label Enter name"
    assert default.blocks[0]["text"] == included.blocks[0]["text"] == "Label Enter name"
    assert default.blocks[0]["spans"][1]["placeholder"] is True
    assert "spans" not in included.blocks[0]


@pytest.mark.parametrize(
    "parse_api", [kordoc.parse, kordoc.try_parse, kordoc.parse_hwpx]
)
def test_trailing_column_option_retains_the_explicit_blank_anchor(parse_api) -> None:
    section = _body_section(
        '<hp:tbl><hp:tr><hp:tc><hp:cellAddr rowAddr="0" colAddr="0"/>'
        "<hp:subList><hp:p><hp:run><hp:t>Head</hp:t></hp:run></hp:p>"
        '</hp:subList></hp:tc><hp:tc><hp:cellAddr rowAddr="0" colAddr="1"/>'
        "<hp:subList/></hp:tc></hp:tr></hp:tbl>"
    )
    data = _hwpx_bytes((section,), image=False)
    default = parse_api(data).document
    explicit_false = parse_api(
        data, options={"keep_trailing_empty_cols": False}
    ).document
    kept = parse_api(data, options={"keep_trailing_empty_cols": True}).document
    assert default is not None and explicit_false is not None and kept is not None
    assert default.to_dict() == explicit_false.to_dict()
    assert default.blocks[0]["table"]["cols"] == 1
    assert kept.blocks[0]["table"]["cols"] == 2
    assert kept.blocks[0]["table"]["cells"][0][1]["text"] == ""


@pytest.mark.parametrize(
    "parse_api", [kordoc.parse, kordoc.try_parse, kordoc.parse_hwpx]
)
@pytest.mark.parametrize("html_tables", [False, True])
def test_cell_guides_preserve_ir_text_and_hide_in_pipe_and_html_tables(
    parse_api, html_tables: bool
) -> None:
    section = _body_section(
        '<hp:tbl><hp:tr><hp:tc><hp:cellAddr rowAddr="0" colAddr="0"/>'
        "<hp:subList><hp:p><hp:run><hp:t>Label </hp:t></hp:run><hp:ctrl>"
        '<hp:fieldBegin type="CLICK_HERE" dirty="0"><hp:parameters>'
        '<hp:stringParam name="Direction">Guide</hp:stringParam>'
        "</hp:parameters></hp:fieldBegin></hp:ctrl>"
        "<hp:run><hp:t>Guide</hp:t></hp:run>"
        "<hp:ctrl><hp:fieldEnd/></hp:ctrl></hp:p></hp:subList></hp:tc>"
        '<hp:tc><hp:cellAddr rowAddr="0" colAddr="1"/><hp:subList>'
        "<hp:p><hp:run><hp:t>Other</hp:t></hp:run></hp:p>"
        "</hp:subList></hp:tc></hp:tr></hp:tbl>"
    )
    data = _hwpx_bytes((section,), image=False)
    default = parse_api(data, options={"html_tables": html_tables}).document
    included = parse_api(
        data,
        options={"html_tables": html_tables, "include_field_placeholders": True},
    ).document
    assert default is not None and included is not None
    assert default.blocks[0]["table"]["cells"][0][0]["text"] == "Label Guide"
    assert included.blocks[0]["table"]["cells"][0][0]["text"] == "Label Guide"
    assert "Label" in default.markdown and "Other" in default.markdown
    assert "Guide" not in default.markdown
    assert "Guide" in included.markdown
    assert all("Guide" not in page["markdown"] for page in default.pages or ())
