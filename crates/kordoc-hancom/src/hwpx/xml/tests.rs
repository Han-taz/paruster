use crate::hwpx::xml::{XmlFault, parse, parse_critical, unclosed_tag_name_ranges};
use kordoc_ir::ErrorCode;

#[test]
fn uses_local_names_for_prefixed_elements() {
    let root = parse_critical(
        br#"<?xml version="1.0"?><a:document xmlns:a="urn:test"><b:item xmlns:b="urn:other">ok</b:item></a:document>"#,
    )
    .unwrap();
    assert_eq!(root.name, "document");
    assert_eq!(root.children[0].name, "item");
    assert_eq!(root.children[0].text, "ok");
}

#[test]
fn rejects_dtd_and_entities() {
    let error = parse_critical(br#"<!DOCTYPE x [<!ENTITY secret "expanded">]><x>&secret;</x>"#)
        .unwrap_err();
    assert_eq!(error.code, ErrorCode::Corrupted);
    assert!(error.message.contains("DTD"));
}

#[test]
fn critical_xml_depth_201_is_corrupted() {
    let xml = format!("<r>{}</r>", "<x>".repeat(200));
    let error = parse(xml.as_bytes()).unwrap_err();
    assert!(matches!(error.fault, XmlFault::DepthLimit));
    let error = parse_critical(xml.as_bytes()).unwrap_err();
    assert_eq!(error.code, ErrorCode::Corrupted);
}

#[test]
fn unfinished_tag_ranges_preserve_qualified_opening_order() {
    let xml = b"<hs:sec><hp:p><hp:t>partial";
    let ranges = unclosed_tag_name_ranges(xml).unwrap();
    let names: Vec<_> = ranges
        .iter()
        .map(|range| std::str::from_utf8(&xml[range.clone()]).unwrap())
        .collect();
    assert_eq!(names, ["hs:sec", "hp:p", "hp:t"]);
}

#[test]
fn unfinished_tag_ranges_are_only_reported_for_clean_eof() {
    assert!(unclosed_tag_name_ranges(b"<root><child></root>").is_none());
    assert!(unclosed_tag_name_ranges(b"<root><child/></root>").is_none());
    let attributed = b"<root attr='value&lt;fake'>";
    let ranges = unclosed_tag_name_ranges(attributed).unwrap();
    assert_eq!(&attributed[ranges[0].clone()], b"root");
    assert!(unclosed_tag_name_ranges(b"<root attr='value<fake'>").is_none());
    assert!(unclosed_tag_name_ranges(b"<root><bad attr='raw<value'/ >").is_none());
    assert!(unclosed_tag_name_ranges(b"<root><child>&#xZZ;").is_none());
}

#[test]
fn unfinished_tag_diagnostic_respects_xml_depth_bound() {
    let xml = "<x>".repeat(201);
    assert!(unclosed_tag_name_ranges(xml.as_bytes()).is_none());
}

#[test]
fn rejects_non_utf8_xml() {
    let error = parse_critical(b"<root>\xff</root>").unwrap_err();
    assert_eq!(error.code, ErrorCode::Corrupted);
}

#[test]
fn xml_text_limit_is_a_resource_error() {
    let text = "x".repeat(16 * 1024 * 1024 + 1);
    let xml = format!("<root>{text}</root>");
    let error = parse_critical(xml.as_bytes()).unwrap_err();
    assert_eq!(error.code, ErrorCode::DecompressionBomb);
}

#[test]
fn enforces_xml_tree_node_budget() {
    let xml = format!("<root>{}</root>", "<x/>".repeat(100_001));
    assert_eq!(
        parse_critical(xml.as_bytes()).err().map(|error| error.code),
        Some(ErrorCode::DecompressionBomb)
    );
}

#[test]
fn enforces_xml_tree_attribute_budget() {
    let attributes = (0..100_001)
        .map(|index| format!(" a{index}=\"x\""))
        .collect::<String>();
    let xml = format!("<root{attributes}/>");
    assert_eq!(
        parse_critical(xml.as_bytes()).err().map(|error| error.code),
        Some(ErrorCode::DecompressionBomb)
    );
}

#[test]
fn enforces_estimated_xml_tree_byte_budget() {
    let value = "x".repeat(32 * 1024 * 1024);
    let xml = format!("<root value=\"{value}\"/>");
    assert_eq!(
        parse_critical(xml.as_bytes()).err().map(|error| error.code),
        Some(ErrorCode::DecompressionBomb)
    );
}

#[test]
fn preserves_mixed_text_and_child_source_order() {
    let root = parse_critical(b"<x>A<y>B</y>C</x>").unwrap();
    assert_eq!(root.text_content(), "ABC");
}

#[test]
fn accepts_utf8_declaration_with_trailing_standalone_attribute() {
    let root = parse_critical(br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><root/>"#)
        .unwrap();
    assert_eq!(root.name, "root");
}
