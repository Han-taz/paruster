use crate::hwpx::xml::{XmlFault, parse, parse_critical};
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
