use super::validate;
use crate::hwpx::package::Package;
use kordoc_ir::ErrorCode;
use std::io::{Cursor, Write};
use zip::{ZipWriter, write::SimpleFileOptions};

fn archive(entries: &[(&str, &[u8])]) -> Vec<u8> {
    let mut writer = ZipWriter::new(Cursor::new(Vec::new()));
    for (path, bytes) in entries {
        writer
            .start_file(*path, SimpleFileOptions::default())
            .unwrap();
        writer.write_all(bytes).unwrap();
    }
    writer.finish().unwrap().into_inner()
}

#[test]
fn validator_reports_missing_required_entries_in_contract_order() {
    let bytes = archive(&[("other", b"x")]);
    let mut package = Package::open(&bytes).unwrap();
    let result = validate(&mut package, None).unwrap();
    assert!(!result.ok);
    assert_eq!(result.entry_count, 1);
    assert!(result.issues[0].message.contains("First ZIP entry"));
    assert!(result.issues[1].message.contains("mimetype"));
}

#[test]
fn validator_orders_xml_then_section_count_then_manifest_diagnostics() {
    let bytes = archive(&[
        ("mimetype", b"application/hwp+zip"),
        ("a.xml", b"<root>"),
        ("Contents/container.xml", b"<root/>"),
        (
            "Contents/content.hpf",
            b"<package><item href=\"missing.xml\"/></package>",
        ),
        ("Contents/header.xml", b"<head secCnt=\"2\"/>"),
        ("Contents/section0.xml", b"<section/>"),
        ("b.xml", b"<root>"),
    ]);
    let mut package = Package::open(&bytes).unwrap();
    let result = validate(&mut package, None).unwrap();
    let diagnostic_paths: Vec<_> = result
        .issues
        .iter()
        .filter_map(|issue| issue.path.as_deref())
        .collect();
    assert_eq!(
        diagnostic_paths,
        [
            "a.xml",
            "b.xml",
            "Contents/header.xml",
            "Contents/content.hpf"
        ]
    );
    assert!(result.issues[3].message.contains("secCnt=2"));
    assert!(result.issues[4].message.contains("missing.xml"));
}

#[test]
fn validator_returns_typed_password_required_for_encrypted_input() {
    let bytes = archive(&[
        (
            "META-INF/manifest.xml",
            br#"<manifest><file-entry full-path="secret.xml"><encryption-data checksum-type="sha256-1k" checksum="AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA="><algorithm algorithm-name="http://www.w3.org/2001/04/xmlenc#aes256-cbc" initialisation-vector="AAAAAAAAAAAAAAAAAAAAAA=="/><start-key-generation start-key-generation-name="http://www.w3.org/2000/09/xmldsig#sha256"/><key-derivation key-derivation-name="PBKDF2" iteration-count="1" key-size="32" salt="AQ=="/></encryption-data></file-entry></manifest>"#,
        ),
        ("secret.xml", &[0u8; 16]),
    ]);
    let mut package = Package::open(&bytes).unwrap();
    assert_eq!(
        validate(&mut package, None).unwrap_err().code,
        ErrorCode::Encrypted
    );
}

#[test]
fn validator_returns_typed_password_invalid_for_wrong_password() {
    let bytes = crate::hwpx::crypto::tests::encrypted_archive(false);
    let mut package = Package::open(&bytes).unwrap();
    assert_eq!(
        validate(&mut package, Some("wrong-password"))
            .unwrap_err()
            .code,
        ErrorCode::Encrypted
    );
}

#[test]
fn validator_entry_count_excludes_directory_records() {
    let mut writer = ZipWriter::new(Cursor::new(Vec::new()));
    writer
        .start_file("mimetype", SimpleFileOptions::default())
        .unwrap();
    writer.write_all(b"application/hwp+zip").unwrap();
    writer
        .add_directory("directory/", SimpleFileOptions::default())
        .unwrap();
    let bytes = writer.finish().unwrap().into_inner();
    let mut package = Package::open(&bytes).unwrap();
    assert_eq!(validate(&mut package, None).unwrap().entry_count, 1);
}
