#[test]
fn scaffold_has_no_false_public_parser_or_metadata_api() {
    let source = include_str!("../src/lib.rs");
    let parser = include_str!("../src/parser.rs");

    assert!(!source.contains("pub use parser"));
    assert!(!source.contains("pub fn parse_pdf"));
    assert!(!parser.contains("pub fn parse_pdf"));
    assert!(!parser.contains("pub fn extract_pdf_metadata"));
}

#[test]
fn crate_manifest_keeps_the_parser_dependency_boundary() {
    let manifest = include_str!("../Cargo.toml");

    assert!(manifest.contains("kordoc-ir"));
    assert!(!manifest.contains("kordoc-core"));
    assert!(!manifest.to_ascii_lowercase().contains("pdfium"));
}
