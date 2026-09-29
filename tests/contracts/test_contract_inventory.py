import json
import re
from pathlib import Path

ROOT = Path(__file__).parents[2]

ERROR_CODES = {
    "EMPTY_INPUT",
    "UNSUPPORTED_FORMAT",
    "ENCRYPTED",
    "DRM_PROTECTED",
    "CORRUPTED",
    "DECOMPRESSION_BOMB",
    "ZIP_BOMB",
    "IMAGE_BASED_PDF",
    "NO_SECTIONS",
    "PARSE_ERROR",
    "MISSING_DEPENDENCY",
    "OUTPUT_TOO_LARGE",
    "FILE_NOT_FOUND",
}

MCP_TOOLS = [
    "parse_document",
    "detect_format",
    "parse_metadata",
    "parse_pages",
    "parse_table",
    "compare_documents",
    "parse_chunks",
    "parse_form",
    "fill_form",
    "place_seal",
    "patch_document",
    "redact_document",
    "render_document",
    "crop_regions",
    "extract_tables",
    "extract_profile",
    "generate_document",
]


def load(name: str) -> dict:
    return json.loads((ROOT / "contracts" / name).read_text(encoding="utf-8"))


def test_error_inventory_is_exact() -> None:
    errors = load("errors.json")
    assert errors["schema_version"] == 1
    assert {item["code"] for item in errors["codes"]} == ERROR_CODES
    assert len(errors["codes"]) == len(ERROR_CODES)
    assert all(item["description"] for item in errors["codes"])


def test_mcp_inventory_is_exact_and_ordered() -> None:
    inventory = load("mcp-tools.json")
    tools = inventory["tools"]
    assert inventory["schema_version"] == 1
    assert [tool["name"] for tool in tools] == MCP_TOOLS
    assert all(tool["status"] == "planned" for tool in tools)
    assert all(tool["source"].startswith("src/mcp/") for tool in tools)


def test_mcp_protocol_covers_every_tool_and_shared_limit() -> None:
    inventory = load("mcp-tools.json")["tools"]
    protocol = load("mcp-protocol.json")
    names = {tool["name"] for tool in inventory}
    assert names == set(protocol["input_schemas"])
    assert names == set(protocol["output_envelopes"])
    assert protocol["transport"] == "stdio"
    assert protocol["limits"] == {
        "document_bytes": 524_288_000,
        "metadata_bytes": 52_428_800,
        "response_characters": 200_000,
    }


def test_every_input_schema_is_a_unique_described_json_schema() -> None:
    schemas = load("mcp-protocol.json")["input_schemas"]
    for name, schema in schemas.items():
        assert schema["type"] == "object", name
        assert schema["properties"], name
        assert schema.get("additionalProperties") is False, name
        assert all(
            isinstance(field.get("description"), str) and field["description"]
            for field in schema["properties"].values()
        ), name
        assert set(schema.get("required", [])) <= set(schema["properties"]), name
        for field in schema["properties"].values():
            if "enum" in field:
                assert field["enum"] and len(field["enum"]) == len(set(field["enum"]))
    serialized = [json.dumps(value, sort_keys=True) for value in schemas.values()]
    assert len(serialized) == len(set(serialized))


def test_generated_levels_schema_rejects_keys_outside_zero_to_seven() -> None:
    levels = load("mcp-protocol.json")["input_schemas"]["generate_document"]["properties"]["levels"]
    assert levels["patternProperties"] == {
        "^[0-7]$": {
            "type": "object",
            "additionalProperties": False,
            "properties": {
                "font": {"type": "string"},
                "pt": {"type": "number", "minimum": 6, "maximum": 60},
                "bold": {"type": "boolean"},
            },
        }
    }
    assert levels["additionalProperties"] is False


def test_nested_generation_objects_reject_unrecognized_keys() -> None:
    fields = load("mcp-protocol.json")["input_schemas"]["generate_document"]["properties"]
    for name in ("doc_info", "fonts", "sizes", "doc_head", "doc_foot", "notice_head"):
        assert fields[name]["additionalProperties"] is False, name
    assert fields["press"]["additionalProperties"] is False
    assert fields["press"]["properties"]["contact"]["additionalProperties"] is False
    checklist_object = fields["checklist"]["oneOf"][1]
    assert checklist_object["additionalProperties"] is False


def test_output_envelopes_capture_success_and_error_shapes_for_every_tool() -> None:
    envelopes = load("mcp-protocol.json")["output_envelopes"]
    for name, envelope in envelopes.items():
        variants = envelope["success"]["variants"]
        assert variants, name
        for variant in variants:
            assert variant["content"], name
            assert all(item["type"] in {"text", "image"} for item in variant["content"]), name
        assert envelope["error"] == {
            "content": [{"type": "text", "text": "operation-specific error message"}],
            "isError": True,
        }, name
        assert envelope["response_behavior"], name


def test_source_hash_evidence_is_complete_and_reproducible() -> None:
    evidence = load("mcp-protocol.json")["source_evidence"]
    expected = {
        "src/mcp.ts",
        "src/mcp/tools-parse.ts",
        "src/mcp/tools-form.ts",
        "src/mcp/tools-render.ts",
        "src/mcp/tools-generate.ts",
        "src/mcp/shared.ts",
        "src/shared/offline.ts",
        "src/utils.ts",
        "src/hwpx/gongmun-surface.ts",
        "src/hwpx/gongmun.ts",
        "src/shared/generate-images.ts",
    }
    assert {item["path"] for item in evidence["files"]} == expected
    assert all(re.fullmatch(r"[0-9a-f]{64}", item["sha256"]) for item in evidence["files"])
    assert evidence["extraction_command"]
    assert evidence["review_command"]
    assert evidence["oracle_path"] == "kordoc/src/mcp"
    expected_hashes = {
        "src/mcp.ts": "16b373581ec8ef71645febc8e47e0c5312b33848406d04d48aff572cc5f8f0f5",
        "src/mcp/shared.ts": "8e71683e60d08ebc05bad2cb90412581dfde2c98c7c472f79a9cc33bd2e773ff",
        "src/mcp/tools-parse.ts": "b633e46ca9f10b95c05fcc6b5b85a358f112df8dc416a748613867cdb9def71c",
        "src/mcp/tools-form.ts": "ea72ed8ac108f87b36e3af77e524e7bbee66e27441ecd36ae37fe7dbaccb01e0",
        "src/mcp/tools-render.ts": "29e1b75e821944f7877af210585d7f461de1bb3720392ba2d01d220aebcca314",
        "src/mcp/tools-generate.ts": "00da61ee0042629045fdf087c4cbb9fe6803bfa5ce3308a6b9e814641c3202a5",
        "src/shared/offline.ts": "66da8b3681443aa677b0efdd17a6cf4105f818b61258d7d2957c3902979a7e72",
        "src/utils.ts": "b5e59e707826cd7aad771de63a09d1d397f0c58a5f7eebb9abd2c0cd534d54ca",
        "src/hwpx/gongmun-surface.ts": "cfd3c05933cd735d7a5dc09b8c0967185272d19db71daf5368676e41edc18970",
        "src/hwpx/gongmun.ts": "3a3fb6a24167f613530d44b069c57fcfb681866d31b8ceaaedda6d8d7ec5af1c",
        "src/shared/generate-images.ts": "b8a25ab961c06013d3b4f5f28b0de38a40a3d683873da767aae064856033b33a",
    }
    assert {item["path"]: item["sha256"] for item in evidence["files"]} == expected_hashes


def test_transport_security_and_limits_are_normative() -> None:
    protocol = load("mcp-protocol.json")
    assert protocol["stdio"] == {
        "framing": "newline-delimited JSON-RPC messages on stdin/stdout",
        "stdout": "protocol messages only",
        "stderr": "diagnostics only",
    }
    security = protocol["security"]
    assert security["root_confinement"]
    assert security["symlink_safe_outputs"]
    assert security["sanitized_errors"]
    assert security["extension_allowlists"]
    assert security["no_input_overwrite"]


def test_response_caps_and_image_limits_match_per_tool_behavior() -> None:
    envelopes = load("mcp-protocol.json")["output_envelopes"]
    capped = {name for name, item in envelopes.items() if "200000" in item["response_behavior"]}
    assert capped == {"parse_document", "parse_chunks", "redact_document"}
    assert "up to 8 image items" in envelopes["render_document"]["response_behavior"]
    assert "up to 8 image content items" in envelopes["extract_tables"]["response_behavior"].lower()
    render = envelopes["render_document"]["success"]["variants"]
    assert render[0]["when"] == {"format": ["png", "jpeg"]}
    assert [item["type"] for item in render[0]["content"]] == ["image", "text"]
    assert render[0]["max_image_items"] == 8
    assert render[1]["when"] == {"format": ["svg", "html", "pdf"]}
    assert [item["type"] for item in render[1]["content"]] == ["text"]
    tables = envelopes["extract_tables"]["success"]["variants"]
    assert tables[0]["when"] == {"visual": ["none"]}
    assert [item["type"] for item in tables[0]["content"]] == ["text"]
    assert tables[1]["when"] == {"visual": ["non-tabular", "non-tabular-and-uncertain", "all"]}
    assert [item["type"] for item in tables[1]["content"]] == ["image", "text"]
    assert tables[1]["max_image_items"] == 8


def test_each_tool_freezes_file_extensions_and_conditional_output_paths() -> None:
    files = load("mcp-protocol.json")["file_behavior"]
    document = [".hwp", ".hwpx", ".hml", ".pdf", ".xls", ".xlsx", ".docx"]
    parse = document + [".png", ".jpg", ".jpeg", ".webp"]
    for name in ("parse_document", "detect_format", "parse_metadata", "parse_pages", "parse_table", "parse_chunks"):
        assert files[name]["input_extensions"] == parse
    for name in ("compare_documents", "parse_form", "fill_form", "place_seal", "redact_document", "extract_profile"):
        assert files[name]["input_extensions"] == document
    assert files["compare_documents"]["input_extensions"] == document
    assert files["place_seal"]["image_extensions"] == [".png", ".jpg", ".jpeg", ".gif", ".bmp"]
    assert files["fill_form"]["output_by_format"] == {
        "markdown": [".md", ".markdown", ".txt"],
        "hwpx": [".hwpx"],
        "hwpx-preserve": [".hwpx"],
    }
    assert files["fill_form"]["output_path_required"] is False
    assert files["fill_form"]["input_source_precedence"] == "template wins when truthy; otherwise file_path is read; absence of both returns an error"
    assert files["place_seal"]["output_extensions"] == [".hwpx"]
    assert files["patch_document"]["input_extensions"] == [".hwpx", ".hwp"]
    assert files["patch_document"]["output_extensions"] == [".hwpx", ".hwp"]
    assert files["patch_document"]["same_extension_as_input_enforced"] is False
    assert files["redact_document"]["output_extensions_by_detected_format"] == {
        "hwpx": [".hwpx"], "hwp": [".hwp"], "other": [".md", ".markdown", ".txt"]
    }
    assert files["redact_document"]["output_path_required_when_dry_run_false"] is True
    assert files["render_document"]["output_extensions_by_format"] == {
        "png": [".png"], "jpeg": [".jpg", ".jpeg"], "svg": [".svg"], "html": [".html", ".htm"], "pdf": [".pdf"]
    }
    assert files["crop_regions"]["manifest"] == "regions.json"
    assert files["extract_tables"]["manifest"] == "tables.json"
    assert files["extract_tables"]["output_dir_required_when_visual_not_none"] is True
    assert files["extract_profile"]["output_extensions"] == [".json"]
    assert files["generate_document"]["output_extensions"] == [".hwpx"]
    assert files["generate_document"]["image_dir_confined_to_root"] is True
    assert files["generate_document"]["image_extension_filter_enforced"] is False


def test_dynamic_generation_schema_preserves_source_enums_ranges_and_key_sets() -> None:
    schema = load("mcp-protocol.json")["input_schemas"]["generate_document"]
    fields = schema["properties"]
    assert fields["preset"]["enum"] == [
        "official", "기안문", "시행문", "공문", "공문서", "report", "보고서", "plan", "계획서", "계획",
        "notice", "통지", "알림", "안내", "minutes", "회의록", "gaejosik", "개조식", "개조식보고서",
        "정부보고서", "정부표준개조식보고서", "press", "보도자료", "ministry", "업무보고", "부처업무보고",
        "중앙부처보고서", "bangchim", "서울방침", "방침서", "방침",
    ]
    assert fields["font"]["enum"] == ["myeongjo", "gothic"]
    assert fields["h2_marker"]["enum"] == ["band", "roman", "box", "number", "none"]
    assert fields["bullet2"]["enum"] == ["ㅇ", "○"]
    assert (fields["body_pt"]["minimum"], fields["body_pt"]["maximum"]) == (6, 40)
    assert (fields["line_spacing"]["minimum"], fields["line_spacing"]["maximum"]) == (50, 300)
    assert set(fields["doc_info"]["properties"]) == {"docNum", "date", "disclosure", "policyNo"}
    assert set(fields["fonts"]["properties"]) == {"body", "heading", "ref", "table"}
    assert set(fields["sizes"]["properties"]) == {
        "dae", "cham", "chapter", "coverTitle", "coverSub", "tocLabel", "tocRoman", "tocItem", "table", "bodyTitle"
    }
    assert fields["approval"]["maxItems"] == 6
    assert "today" in fields["date"]["description"]
    assert "gaejosik" in fields["toc"]["description"] and "press" in fields["toc"]["description"]
    assert "gaejosik" in fields["cover"]["description"]
    assert "org, date" in fields["cover"]["description"]
    assert "report" in fields["page_numbers"]["description"] and "plan" in fields["page_numbers"]["description"]
    assert "heading" in fields["fonts"]["description"] and "body only" in fields["fonts"]["description"]
    assert "all four roles" in fields["fonts"]["description"]
    behavior = load("mcp-protocol.json")["generation_behavior"]
    assert behavior["preset_defaults"]["official"] == {
        "body_pt": 12, "line_spacing": 160, "cover": False, "toc": False,
        "page_numbers": False, "end_mark": True, "body_title_box": False,
        "h2_marker": "none", "bullet2": "ㅇ",
    }
    assert behavior["preset_defaults"]["gaejosik"]["cover"] is True
    assert behavior["preset_defaults"]["gaejosik"]["body_title_box"] is True
    assert behavior["preset_defaults"]["ministry"]["toc"] is True
    assert behavior["preset_defaults"]["bangchim"]["page_numbers"] is True
    assert behavior["preset_defaults"]["press"]["toc"] is False
    assert behavior["font_override_applicability"]["all_roles_presets"] == ["gaejosik", "report", "plan", "bangchim"]
    assert behavior["font_override_applicability"]["body_only_for_other_presets"] is True


def test_detect_format_records_header_probe_and_conditional_full_read() -> None:
    detect = load("mcp-protocol.json")["file_behavior"]["detect_format"]
    assert detect["initial_read_bytes"] == 512
    assert detect["full_file_read_only_for_header_formats"] == ["hwpx", "hwp"]
    assert detect["full_file_read_limit_bytes"] == 524288000
    mcp_doc = (ROOT / "docs/SSOT/contracts/mcp.md").read_text(encoding="utf-8")
    assert "512-byte header" in mcp_doc
    assert "ZIP or OLE" in mcp_doc


def test_contract_pages_and_pull_request_template_cover_review_evidence() -> None:
    errors = (ROOT / "docs/SSOT/contracts/errors.md").read_text(encoding="utf-8")
    mcp = (ROOT / "docs/SSOT/contracts/mcp.md").read_text(encoding="utf-8")
    index = (ROOT / "docs/SSOT/README.md").read_text(encoding="utf-8")
    template = (ROOT / ".github/pull_request_template.md").read_text(encoding="utf-8")
    assert "stable protocol values" in errors
    assert "reserved protocol values" in errors
    assert "compatibility requirements" in mcp
    assert "planned" in mcp
    assert "contracts/errors.md" in index and "contracts/mcp.md" in index
    for section in ("Scope", "Tests", "Parity evidence", "SSOT impact", "WIKI", "Security impact", "Ownership"):
        assert section.lower() in template.lower()
