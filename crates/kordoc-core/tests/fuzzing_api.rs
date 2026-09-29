#![cfg(feature = "fuzzing")]

use kordoc_core::fuzzing::preflight_zip;
use kordoc_core::{FileType, detect_format};

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
