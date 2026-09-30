#![cfg(feature = "fuzzing")]

use kordoc_core::fuzzing::{hwpx_package, hwpx_xml, preflight_zip};
use kordoc_core::{FileType, detect_format};
use kordoc_ir::{ErrorCode, WarningCode};

const DETECT_SEED: &[u8] = include_bytes!("../../../fuzz/corpus/detect_format/minimal-pdf");
const ZIP_SEED: &[u8] = include_bytes!("../../../fuzz/corpus/zip_preflight/minimal.zip");

#[test]
fn fuzzing_surface_accepts_a_minimal_valid_zip() {
    let zip = minimal_zip();
    assert!(preflight_zip(&zip));
}

#[test]
fn fuzzing_surface_rejects_malformed_zip_without_panicking() {
    assert!(!preflight_zip(b"PK\x03\x04 malformed"));
}

#[test]
fn checked_in_seeds_are_synthetic_valid_inputs() {
    assert_eq!(detect_format(DETECT_SEED), Ok(FileType::Pdf));
    assert!(preflight_zip(ZIP_SEED));
}

#[test]
fn hwpx_package_fuzz_surface_checks_untrusted_zip_without_a_core_detection_bypass() {
    assert_eq!(
        hwpx_package(b"not a zip").unwrap_err().code,
        ErrorCode::ZipBomb
    );
}

#[test]
fn hwpx_xml_fuzz_surface_retains_sound_neighbors_after_malformed_middle_section() {
    let result = hwpx_xml(b"<sec><p>").unwrap();
    assert_eq!(result.blocks.len(), 2);
    assert_eq!(result.blocks[0].text.as_deref(), Some("before"));
    assert_eq!(result.blocks[1].text.as_deref(), Some("after"));
    assert_eq!(result.blocks[1].page_number, Some(3));
    let warnings = result.warnings.unwrap();
    assert_eq!(warnings.len(), 1);
    assert_eq!(warnings[0].page, Some(2));
    assert_eq!(warnings[0].code, WarningCode::PartialParse);
}

#[test]
fn hwpx_xml_fuzz_surface_does_not_resolve_declared_entities() {
    let result = hwpx_xml(
        b"<!DOCTYPE sec [<!ENTITY e SYSTEM 'file:///secret'>]><sec><p><t>&e;</t></p></sec>",
    )
    .unwrap();
    assert_eq!(result.blocks.len(), 2);
    assert_eq!(result.warnings.unwrap()[0].code, WarningCode::PartialParse);
}

fn minimal_zip() -> Vec<u8> {
    let mut zip = Vec::new();
    zip.extend_from_slice(b"PK\x03\x04");
    zip.extend_from_slice(&[0; 22]);
    zip.extend_from_slice(&1u16.to_le_bytes());
    zip.extend_from_slice(&0u16.to_le_bytes());
    zip.extend_from_slice(b"x");
    let central_offset = zip.len() as u32;
    zip.extend_from_slice(b"PK\x01\x02");
    let mut central = [0; 42];
    central[24..26].copy_from_slice(&1u16.to_le_bytes());
    zip.extend_from_slice(&central);
    zip.extend_from_slice(b"x");
    let central_size = zip.len() as u32 - central_offset;
    zip.extend_from_slice(b"PK\x05\x06");
    zip.extend_from_slice(&[0; 4]);
    zip.extend_from_slice(&1u16.to_le_bytes());
    zip.extend_from_slice(&1u16.to_le_bytes());
    zip.extend_from_slice(&central_size.to_le_bytes());
    zip.extend_from_slice(&central_offset.to_le_bytes());
    zip.extend_from_slice(&0u16.to_le_bytes());
    zip
}
