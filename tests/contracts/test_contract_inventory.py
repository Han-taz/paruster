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


def validate_schema(schema: dict, path: str) -> None:
    if "enum" in schema:
        assert schema["enum"] and len(schema["enum"]) == len(set(schema["enum"])), path
    if "required" in schema:
        assert isinstance(schema["required"], list), path
        assert set(schema["required"]) <= set(schema.get("properties", {})), path
    if "properties" in schema or schema.get("type") == "object":
        assert schema.get("type") == "object", path
        assert "additionalProperties" in schema, path
    if "properties" in schema:
        assert schema["additionalProperties"] is False, path
    if "patternProperties" in schema:
        assert schema.get("additionalProperties") is False, path
        for pattern, child in schema["patternProperties"].items():
            re.compile(pattern)
            validate_schema(child, f"{path}.patternProperties[{pattern!r}]")
    for key, child in schema.get("properties", {}).items():
        validate_schema(child, f"{path}.properties[{key!r}]")
    additional = schema.get("additionalProperties")
    if isinstance(additional, dict):
        validate_schema(additional, f"{path}.additionalProperties")
    if isinstance(schema.get("items"), dict):
        validate_schema(schema["items"], f"{path}.items")
    for key in ("anyOf", "oneOf", "allOf"):
        for index, child in enumerate(schema.get(key, [])):
            validate_schema(child, f"{path}.{key}[{index}]")
    for low, high in (("minimum", "maximum"), ("minLength", "maxLength"), ("minItems", "maxItems")):
        if low in schema and high in schema:
            assert schema[low] <= schema[high], path


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
        validate_schema(schema, name)
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
        assert set(envelope) == {"success", "error", "response_behavior", "response_character_cap", "truncation"}, name
        assert set(envelope["success"]) == {"variants"}, name
        variants = envelope["success"]["variants"]
        assert variants, name
        for variant in variants:
            assert set(variant) <= {"when", "content", "max_image_items"}, name
            assert isinstance(variant["when"], dict), name
            assert variant["content"], name
            for item in variant["content"]:
                assert item["type"] in {"text", "image"}, name
                if item["type"] == "text":
                    assert set(item) <= {"type", "text", "position"}, name
                    assert isinstance(item["text"], str), name
                else:
                    assert set(item) <= {"type", "data", "mimeType", "min_items", "max_items"}, name
                    assert item["data"] == "base64", name
                    assert item["mimeType"] in {"image/png or image/jpeg", "image/png", "image/jpeg"}, name
                    assert 0 <= item.get("min_items", 1) <= item.get("max_items", 1) <= 8, name
        assert envelope["error"] == {
            "content": [{"type": "text", "text": "operation-specific error message"}],
            "isError": True,
        }, name
        assert envelope["response_behavior"], name


def test_output_variant_conditions_reference_and_partition_input_enums() -> None:
    protocol = load("mcp-protocol.json")
    for name, envelope in protocol["output_envelopes"].items():
        schema = protocol["input_schemas"][name]
        variants = envelope["success"]["variants"]
        for variant in variants:
            for field_name, accepted in variant["when"].items():
                field = schema["properties"][field_name]
                assert set(accepted) <= set(field["enum"]), (name, field_name)
        for index, left in enumerate(variants):
            for right in variants[index + 1:]:
                for field_name in set(left["when"]) & set(right["when"]):
                    assert not (set(left["when"][field_name]) & set(right["when"][field_name])), name


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
    assert evidence["cwd"] == "kordoc"
    review_commands = evidence["review_commands"]
    assert {command.rsplit(" ", 1)[-1] for command in review_commands} == expected
    assert all(command.startswith("sed -n '1,$p' ") for command in review_commands)
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
    assert security["root_confinement"] == {
        "enabled_when": {"environment_variable": "KORDOC_ROOT", "condition": "set_and_non_empty"},
        "offline_independence": {"environment_variable": "KORDOC_OFFLINE", "alone_enables_confinement": False},
        "scope": ["canonicalized_input_paths", "canonicalized_output_paths"],
        "boundary_check": {"method": "path_segment_relative", "base": "real_root"},
    }
    assert security["input_path_resolution"]["steps"] == [
        {"action": "reject_empty_path"},
        {"action": "resolve_path"},
        {"action": "realpath_input", "follows_symlinks": True},
        {"action": "require_absolute_canonical_path"},
        {"action": "assert_within_root", "condition": "KORDOC_ROOT_non_empty"},
        {"action": "check_operation_extension_allowlist"},
        {"action": "stat_and_enforce_operation_size_limit"},
        {"action": "read_file"},
    ]
    assert security["output_path_resolution"]["steps"] == [
        {"action": "reject_empty_path"},
        {"action": "resolve_path"},
        {"action": "check_output_extension_allowlist"},
        {"action": "lstat_final_leaf", "reject_if_symlink": True},
        {"action": "find_nearest_existing_ancestor"},
        {"action": "realpath_ancestor_and_append_remaining_segments"},
        {"action": "assert_within_root", "condition": "KORDOC_ROOT_non_empty"},
        {"action": "recheck_real_parent_and_assert_root", "condition": "after_processing"},
        {"action": "open_final_leaf", "flags": ["O_NOFOLLOW"]},
    ]
    assert security["symlink_rules"] == {
        "input_paths": {"realpath_follows_symlinks_before_root_check": True},
        "output_leaf": {"lstat_before_write": True, "reject_symlink": True},
        "output_ancestors": {"resolve_nearest_existing_ancestor_with_realpath": True, "reject_dangling_symlink": True},
        "write_race": {"recheck_real_parent": True, "open_flags": ["O_NOFOLLOW"]},
        "generation_assets": {
            "resolve_image_dir_and_target": True,
            "remain_within_image_dir": True,
            "remain_within_configured_root_when_set": True,
            "open_leaf_flags": ["O_NOFOLLOW"],
        },
    }
    assert security["error_sanitization"] == {
        "mcp_failure_envelope": {"content_type": "text", "isError": True},
        "categories": {
            "KordocError": {"action": "pass_message_through", "may_include_path": True},
            "filesystem_error": {
                "branch": {"error_code_pattern": "^E[A-Z]+$"},
                "action": "emit_code_specific_hint",
                "include_raw_os_message": False,
                "include_raw_path": False,
            },
            "parse_error": {"classification": "PARSE_ERROR", "action": "sanitize_error"},
            "other_error": {"action": "emit_classified_category_without_native_details"},
        },
    }
    assert security["extension_allowlists"]
    assert "no_input_overwrite" not in security
    assert security["same_file_policy"] == {
        name: (
            {"checked": True, "reject_if_same_resolved_file": True}
            if name == "redact_document"
            else {"checked": False, "reject_if_same_resolved_file": False}
        )
        for name in MCP_TOOLS
    }
    mcp_doc = (ROOT / "docs/SSOT/contracts/mcp.md").read_text(encoding="utf-8")
    assert "KORDOC_OFFLINE alone does not enable root confinement" in mcp_doc
    assert "Only `redact_document` rejects a same-file input/output path" in mcp_doc


def test_response_caps_and_image_limits_match_per_tool_behavior() -> None:
    envelopes = load("mcp-protocol.json")["output_envelopes"]
    capped = {name for name, item in envelopes.items() if item["response_character_cap"] == 200_000}
    assert capped == {"parse_document", "parse_chunks", "redact_document"}
    for name, envelope in envelopes.items():
        if name in capped:
            assert envelope["truncation"]["enabled"] is True
            assert envelope["truncation"]["limit_chars"] == 200_000
            assert envelope["truncation"]["marker_semantics"] == {
                "append_after_truncated_text": True,
                "includes_limit": True,
                "includes_original_length": True,
            }
            assert "{limit_formatted}" in envelope["truncation"]["marker_template"]
            assert "{actual_formatted}" in envelope["truncation"]["marker_template"]
        else:
            assert envelope["response_character_cap"] is None
            assert envelope["truncation"]["enabled"] is False
    assert "up to 8 image items" in envelopes["render_document"]["response_behavior"]
    assert "zero to 8 image content items" in envelopes["extract_tables"]["response_behavior"].lower()
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
    assert tables[1]["content"][0]["min_items"] == 0
    assert tables[1]["content"][0]["max_items"] == 8
    assert tables[1]["content"][1]["position"] == "last"
    assert tables[1]["max_image_items"] == 8


def test_each_tool_freezes_file_extensions_and_conditional_output_paths() -> None:
    files = load("mcp-protocol.json")["file_behavior"]
    assert set(files) == set(MCP_TOOLS)
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
    assert files["patch_document"]["input_extensions"] == document
    assert files["patch_document"]["runtime_input_format"] == ["hwpx", "hwp"]
    assert files["patch_document"]["rejects_confirmed_detected_other_formats"] is True
    assert files["patch_document"]["unknown_detection"] == "defer to the HWPX/HWP patcher"
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


def test_every_api_entry_has_a_disposition_and_exact_export_coverage() -> None:
    public_api = load("public-api.json")
    oracle = load("oracle-public-exports.json")
    entries = public_api["entries"]
    type_entries = public_api["type_entries"]
    values = [entry["source_name"] for entry in entries]
    types = [entry["source_name"] for entry in type_entries]

    assert values and types
    assert len(values) == len(set(values))
    assert len(types) == len(set(types))
    assert set(values) == set(oracle["named_exports"])
    assert set(types) == set(oracle["type_exports"])
    assert {entry["disposition"] for entry in entries} <= {
        "foundation", "planned", "removed-node-surface", "internal",
    }
    assert {entry["disposition"] for entry in type_entries} <= {
        "foundation", "planned", "removed-node-surface", "internal",
    }
    for entry in entries:
        assert entry["python_name"]
        assert entry["mapping"]
        assert entry["rationale"]
    for entry in type_entries:
        assert entry["mapping"]
        assert entry["rationale"]
        assert entry["disposition"] in {"foundation", "planned", "removed-node-surface", "internal"}
    mapped_names = [entry["python_name"] for entry in entries if entry["python_name"]]
    assert len(mapped_names) == len(set(mapped_names))
    assert {entry["source_name"]: entry["python_name"] for entry in entries}["parse"] == "parse"
    assert {entry["source_name"]: entry["disposition"] for entry in entries}["parse"] == "foundation"
    format_parsers = {
        "parseHwpx": "parse_hwpx", "parseHwp": "parse_hwp", "parseHwp3": "parse_hwp3",
        "parsePdf": "parse_pdf", "parseXlsx": "parse_xlsx", "parseXls": "parse_xls",
        "parseDocx": "parse_docx", "parseHwpml": "parse_hwpml", "parseImage": "parse_image",
    }
    by_source = {entry["source_name"]: entry for entry in entries}
    assert {name: by_source[name]["python_name"] for name in format_parsers} == format_parsers
    assert all(by_source[name]["disposition"] == "planned" for name in format_parsers)
    assert {item["source_name"]: item["disposition"] for item in public_api["excluded_surfaces"]}["filePath"] == "internal"
    assert {item["source_name"]: item["disposition"] for item in public_api["excluded_surfaces"]}["Node CLI"] == "removed-node-surface"
    assert {item["source_name"]: item["disposition"] for item in public_api["excluded_surfaces"]}["Node package entry machinery"] == "removed-node-surface"
    assert {"compare", "diffBlocks", "fillForm", "markdownToHwpx", "patchHwpx", "patchHwp",
            "validateHwpx", "redactText", "redactMarkdown", "blocksToChunks", "renderDocument",
            "detectFormat", "blocksToMarkdown", "blocksToPages"} <= set(values)
    assert len(public_api["excluded_surfaces"]) == len({item["source_name"] for item in public_api["excluded_surfaces"]})
    assert all(item["disposition"] and item["mapping"] is not None or item["disposition"] in {"internal", "removed-node-surface"}
               for item in public_api["excluded_surfaces"])


def test_oracle_export_snapshot_is_hashed_and_reproducible_without_runtime_oracle() -> None:
    snapshot = load("oracle-public-exports.json")
    evidence = snapshot["source_evidence"]
    assert evidence["path"] == "src/index.ts"
    assert re.fullmatch(r"[0-9a-f]{64}", evidence["sha256"])
    assert snapshot["source_sha256"] == evidence["sha256"]
    assert evidence["sha256_command"] == "sha256sum src/index.ts"
    assert evidence["extraction_command"]
    assert evidence["cwd"] == "kordoc"
    assert evidence["review_command"] == "sed -n '1,$p' src/index.ts"
    assert snapshot["named_exports"] and snapshot["type_exports"]
    assert len(snapshot["named_exports"]) == len(set(snapshot["named_exports"]))
    assert len(snapshot["type_exports"]) == len(set(snapshot["type_exports"]))
    assert not any("kordoc/" in str(path) for path in (ROOT / "tests/contracts").rglob("*.py"))
    tracked = __import__("subprocess").run(
        ["git", "ls-files", "--", "kordoc"], cwd=ROOT, check=True, capture_output=True, text=True
    ).stdout
    assert not tracked.splitlines()


def test_contract_tests_and_build_manifests_do_not_depend_on_oracle_checkout() -> None:
    paths = list((ROOT / "tests").rglob("*.py"))
    paths.extend(path for path in (ROOT / "Cargo.toml", ROOT / "pyproject.toml") if path.exists())
    for path in paths:
        contents = path.read_text(encoding="utf-8")
        assert "from " + "kordoc" not in contents and "import " + "kordoc" not in contents, path
        assert "ROOT / " + "\"kordoc\"" not in contents, path
        assert "file:" + "//" not in contents, path


def test_ir_schema_is_recursive_complete_and_internally_consistent() -> None:
    contract = load("ir-schema.json")
    oracle = load("oracle-public-exports.json")
    schema = contract["$defs"]
    evidence = contract["source_evidence"]
    assert contract["schema_version"] == 1
    assert evidence["source_sha256"] == oracle["source_sha256"]
    assert evidence["source_path"] == oracle["source_evidence"]["path"]
    assert contract["wire_types"]
    type_classes = contract["type_classifications"]
    assert {item["source_name"] for item in type_classes} == set(oracle["type_exports"])
    assert len(type_classes) == len({item["source_name"] for item in type_classes})
    assert {item["category"] for item in type_classes} <= {
        "serializable-object", "serializable-enum", "serializable-union", "input-options", "callable-adapter",
    }
    classes_by_name = {item["source_name"]: item for item in type_classes}
    assert all(item["mapping"] == "#/$defs/" + item["source_name"] for item in type_classes)
    serializable = {
        name for name, item in classes_by_name.items()
        if item["category"] in {"serializable-object", "serializable-enum", "serializable-union", "input-options"}
    }
    assert set(contract["wire_types"]) == serializable
    assert set(contract["root_types"]) == serializable
    assert serializable <= set(schema)
    assert set(contract["anyOf"][index]["$ref"].removeprefix("#/$defs/") for index in range(len(contract["anyOf"]))) == serializable
    for name in serializable:
        kind = schema[name].get("type")
        assert kind in {"object", "string"} or "oneOf" in schema[name] or "anyOf" in schema[name], name
        fields = classes_by_name[name].get("fields")
        if fields is not None:
            assert set(fields) - set(classes_by_name[name].get("excluded_fields", [])) == set(schema[name].get("properties", {})), name
            assert set(classes_by_name[name]["required_fields"]) == set(schema[name].get("required", [])), name
        if "excluded_fields" in classes_by_name[name]:
            assert classes_by_name[name]["excluded_fields"] == schema[name]["x-internal-fields"]
        if "enum" in schema[name]:
            assert classes_by_name[name]["enum_values"] == schema[name]["enum"], name
        assert classes_by_name[name]["mapping"]
    assert set(classes_by_name["OcrProvider"]["python_adapter_fields"]) == {"ocr"}
    expected_adapters = {
        "ExtractRegionOptions": {"filter"}, "ExtractedImage": {"data"}, "ExtractedTableCrop": {"data"},
        "FillFormOutput": {"output"}, "HwpxFillResult": {"buffer"}, "ImageData": {"data"},
        "MarkdownToHwpxOptions": {"images"}, "ParseOptions": {"ocr", "onProgress"},
        "PatchResult": {"data"}, "PlaceSealResult": {"buffer"}, "RegionAsset": {"data"},
        "RenderAsset": {"data"}, "ScanTable": {"cellByAnchor"}, "SceneRenderResult": {"pageSvgs"},
        "SealOp": {"image"},
    }
    for name, definition in schema.items():
        if definition.get("type") != "object":
            continue
        expected = expected_adapters.get(name, set())
        classified = set(classes_by_name[name].get("adapter_fields", [])) if name in classes_by_name else set()
        assert classified == expected, name
        assert expected <= set(definition.get("properties", {})), name
    assert schema["SceneRenderResult"]["properties"]["pageSvgs"]["type"] == "object"
    assert schema["ScanTable"]["properties"]["cellByAnchor"]["type"] == "object"
    assert schema["ExtractRegionOptions"]["properties"]["filter"]["not"] == {}
    assert set(contract["supporting_types"]) == set(schema) - set(oracle["type_exports"])
    assert set(schema) == set(oracle["type_exports"]) | set(contract["supporting_types"])
    assert contract["$schema"] == "https://json-schema.org/draft/2020-12/schema"
    assert contract["anyOf"] == [{"$ref": "#/$defs/" + name} for name in contract["root_types"]]
    assert evidence["type_source_sha256"] == "5b1e4b5793a04bd059600b6daf3b14b1b299958ee9f22633d9027c9d9956110c"
    assert contract["roots"] == {
        "parse_result": {"$ref": "#/$defs/ParseResult"},
        "block": {"$ref": "#/$defs/IRBlock"},
        "warning": {"$ref": "#/$defs/ParseWarning"},
        "error_code": {"$ref": "#/$defs/ErrorCode"},
    }

    references: list[str] = []

    def collect_refs(value: object, path: str) -> None:
        if isinstance(value, dict):
            if "$ref" in value:
                ref = value["$ref"]
                assert ref.startswith("#/$defs/"), path
                name = ref.removeprefix("#/$defs/")
                assert name in schema, (path, ref)
                references.append(name)
            for key, child in value.items():
                collect_refs(child, f"{path}.{key}")
        elif isinstance(value, list):
            for index, child in enumerate(value):
                collect_refs(child, f"{path}[{index}]")

    collect_refs(contract["roots"], "roots")
    collect_refs(schema, "$defs")
    assert "IRBlock" in references
    reachable = set()
    pending = list(contract["root_types"])
    while pending:
        name = pending.pop()
        if name in reachable:
            continue
        reachable.add(name)
        nested: list[str] = []
        collect_refs(schema[name], name)
        def gather(value: object) -> None:
            if isinstance(value, dict):
                ref = value.get("$ref")
                if isinstance(ref, str) and ref.startswith("#/$defs/"):
                    nested.append(ref.removeprefix("#/$defs/"))
                for child in value.values():
                    gather(child)
            elif isinstance(value, list):
                for child in value:
                    gather(child)
        gather(schema[name])
        pending.extend(nested)
    assert set(contract["supporting_types"]) <= reachable
    for name, definition in schema.items():
        validate_schema(definition, name)
        if definition.get("type") == "object":
            assert set(definition.get("required", [])) <= set(definition.get("properties", {})), name
            assert definition.get("additionalProperties") is False, name
        if "enum" in definition:
            assert definition["enum"] and len(definition["enum"]) == len(set(definition["enum"])), name
    assert schema["IRBlock"]["properties"]["children"]["items"] == {"$ref": "#/$defs/IRBlock"}
    assert schema["IRCell"]["properties"]["blocks"]["items"] == {"$ref": "#/$defs/IRBlock"}
    assert schema["IRTable"]["properties"]["captionBlocks"]["items"] == {"$ref": "#/$defs/IRBlock"}
    assert set(schema["ParseSuccess"]["properties"]) >= {
        "success", "fileType", "markdown", "blocks", "metadata", "outline", "warnings",
        "images", "pages", "pageQuality", "qualitySummary", "pageCount", "isImageBased",
    }
    assert set(schema["ParseFailure"]["properties"]) == {"success", "fileType", "error", "code", "pageCount", "isImageBased"}
    assert set(schema["ParseSuccess"]["required"]) == {"success", "fileType", "markdown", "blocks"}
    assert set(schema["ParseFailure"]["required"]) == {"success", "fileType", "error"}
    assert schema["ErrorCode"]["enum"] == [item["code"] for item in load("errors.json")["codes"]]
    assert schema["WarningCode"]["enum"] == contract["warning_codes"]
    assert schema["IRBlock"]["properties"]["pageNumber"]["type"] == "number"
    assert "pageNumber" not in schema["IRBlock"].get("required", [])
    assert "kind" not in schema["IRBlock"]["properties"]
    assert set(schema["IRBlock"]["properties"]) == {
        "type", "text", "table", "level", "pageNumber", "bbox", "style", "listType", "children", "href",
        "footnoteText", "imageData", "spans", "quote", "indent", "listDepth",
    }
    assert set(schema["IRSpan"]["properties"]) == {
        "text", "bold", "italic", "strike", "underline", "code", "placeholder",
    }
    assert set(schema["IRTable"]["properties"]) == {
        "rows", "cols", "cells", "renderAsTable", "hasHeader", "classification", "sourceId", "regions", "caption", "captionBlocks",
    }
    assert set(schema["IRCell"]["properties"]) == {"text", "colSpan", "rowSpan", "blocks", "isHeader"}
    assert set(schema["DocumentMetadata"]["properties"]) == {
        "title", "author", "creator", "createdAt", "modifiedAt", "pageCount", "pageMode", "version", "description", "keywords",
    }
    assert set(schema["ParseOptions"]["properties"]) == {
        "pages", "ocr", "onProgress", "removeHeaderFooter", "scriptTags", "plain", "htmlTables", "keepTrailingEmptyCols", "classifyTables",
        "keepEmptyParagraphs", "includeFieldPlaceholders", "password", "formulaOcr", "dedupeRunningHeaders", "inlineImages", "images", "tables",
    }
    assert "filePath" not in schema["ParseOptions"]["properties"]
    assert schema["ParseOptions"]["properties"]["onProgress"]["not"] == {}
    assert schema["ParseOptions"]["properties"]["onProgress"]["x-typescript-callable"] is True
    assert schema["ParseOptions"]["properties"]["onProgress"]["x-python-translation"] == "callable (current, total)"
    assert schema["ParseOptions"]["properties"]["ocr"]["oneOf"] == [
        {"type": "boolean"}, {"const": "force"},
    ]
    assert schema["ParseOptions"]["properties"]["ocr"]["x-python-callable-adapter"] is True
    assert schema["ParseOptions"]["x-internal-fields"] == ["filePath"]
    assert schema["ParseOptions"]["x-python-callable-fields"] == ["onProgress", "ocr"]
    assert schema["ParseOptions"]["x-node-only-fields"] == ["filePath"]
    assert "code" not in schema["ParseFailure"]["required"]
    assert schema["ParseFailure"]["properties"]["code"] == {"$ref": "#/$defs/ErrorCode"}
    assert schema["WarningCode"]["enum"] == [
        "SKIPPED_IMAGE", "SKIPPED_OLE", "TRUNCATED_TABLE", "OCR_FALLBACK", "UNSUPPORTED_ELEMENT",
        "BROKEN_ZIP_RECOVERY", "HIDDEN_TEXT_FILTERED", "MALFORMED_XML", "PARTIAL_PARSE", "LENIENT_CFB_RECOVERY",
        "NEEDS_OCR", "OCR_FAILED", "OCR_APPLIED", "OCR_LOW_CONF", "COM_EMPTY", "DRM_COM_FALLBACK", "PAGE_BOUNDARY_APPROXIMATE",
    ]
    assert schema["IRBlockType"]["enum"] == ["paragraph", "table", "heading", "list", "image", "separator"]
    assert schema["TableClassificationKind"]["enum"] == ["semantic-table", "non-tabular-layout", "uncertain"]
    assert schema["TableClassificationReason"]["enum"] == [
        "repeated-row-schema", "grid-regularity", "high-active-density", "column-type-consistency",
        "nested-structure-wrapper", "span-irregularity", "spacer-bands", "extreme-sparsity",
        "diagram-context-keyword", "low-evidence", "ambiguous-scores",
    ]
    assert schema["PageQuality"]["properties"]["ocrReason"]["enum"] == [
        "vector_text", "low_text", "high_pua", "high_control", "high_replacement", "garbled_hangul",
    ]
    assert contract["source_evidence"]["source_sha256"] == "85943942619d9c2b7ed0855bdb58eb0360fec028e84efdd0f719589ba6ce4e01"
    assert schema["ImageData"]["properties"]["data"]["x-python-translation"] == "bytes"
    assert all(item.get("type") != "null" for item in schema.values())
