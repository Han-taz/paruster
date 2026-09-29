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


def test_output_envelopes_capture_success_and_error_shapes_for_every_tool() -> None:
    envelopes = load("mcp-protocol.json")["output_envelopes"]
    for name, envelope in envelopes.items():
        assert envelope["success"]["content"], name
        assert all(item["type"] in {"text", "image"} for item in envelope["success"]["content"]), name
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
    assert "image" in {item["type"] for item in envelopes["render_document"]["success"]["content"]}
    assert "image" in {item["type"] for item in envelopes["extract_tables"]["success"]["content"]}


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
