#[test]
fn scaffold_has_no_false_public_parser_or_validator() {
    let source = include_str!("../src/lib.rs");

    assert!(!source.contains("pub fn parse_hwpx"));
    assert!(!source.contains("pub fn parse_hwpx_metadata"));
    assert!(!source.contains("pub fn validate_hwpx"));
    assert!(!source.contains("ValidateResult"));
}

#[test]
fn crate_manifest_keeps_the_parser_dependency_boundary() {
    let manifest = include_str!("../Cargo.toml");

    assert!(manifest.contains("kordoc-ir"));
    assert!(!manifest.contains("kordoc-core"));
}
