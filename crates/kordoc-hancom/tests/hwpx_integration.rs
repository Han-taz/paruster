#[path = "support/hwpx_fixture.rs"]
mod hwpx_fixture;

use std::io::Cursor;

use aes::Aes256;
use aes::cipher::{BlockModeDecrypt, KeyIvInit, block_padding::NoPadding};
use flate2::read::DeflateDecoder;
use hwpx_fixture::Recipe;
use pbkdf2::pbkdf2_hmac;
use sha1::Sha1;
use sha2::{Digest, Sha256};
use zip::ZipArchive;

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

#[test]
fn every_hwpx_recipe_is_byte_for_byte_deterministic() {
    for recipe in Recipe::ALL {
        let first = recipe.build();
        let second = recipe.build();
        assert_eq!(first, second, "{}", recipe.id());
        assert!(!first.is_empty(), "{}", recipe.id());
        assert_eq!(Sha256::digest(&first), Sha256::digest(&second));
    }
}

#[test]
fn fixture_zip_records_have_fixed_order_timestamp_and_utf8_xml() {
    let bytes = Recipe::Minimal.build();
    let mut archive = ZipArchive::new(Cursor::new(bytes)).unwrap();
    let names: Vec<_> = archive.file_names().map(str::to_owned).collect();
    assert_eq!(
        names,
        [
            "mimetype",
            "META-INF/manifest.xml",
            "META-INF/container.xml",
            "Contents/content.hpf",
            "Contents/header.xml",
            "Contents/section0.xml",
        ]
    );
    for index in 0..archive.len() {
        let mut member = archive.by_index(index).unwrap();
        let stamp = member.last_modified().unwrap();
        assert_eq!((stamp.year(), stamp.month(), stamp.day()), (1980, 1, 1));
        assert_eq!((stamp.hour(), stamp.minute(), stamp.second()), (0, 0, 0));
        if member.name().ends_with(".xml") || member.name().ends_with(".hpf") {
            let mut xml = String::new();
            std::io::Read::read_to_string(&mut member, &mut xml).unwrap();
            assert!(xml.starts_with("<?xml version=\"1.0\" encoding=\"UTF-8\"?>"));
        }
    }
}

#[test]
fn section_spine_and_record_boundary_recipes_keep_their_intended_shapes() {
    let bytes = Recipe::TwoSectionSpineReversed.build();
    let mut archive = ZipArchive::new(Cursor::new(bytes)).unwrap();
    let names: Vec<_> = archive.file_names().map(str::to_owned).collect();
    assert!(
        names
            .iter()
            .position(|name| name == "Contents/section0.xml")
            < names
                .iter()
                .position(|name| name == "Contents/section1.xml")
    );
    let mut opf = String::new();
    std::io::Read::read_to_string(
        &mut archive.by_name("Contents/content.hpf").unwrap(),
        &mut opf,
    )
    .unwrap();
    assert!(opf.contains("<opf:itemref idref=\"section1\"/><opf:itemref idref=\"section0\"/>"));

    for (recipe, expected) in [
        (Recipe::DirectoryEntry500, 500),
        (Recipe::DirectoryEntry501, 501),
    ] {
        let archive = ZipArchive::new(Cursor::new(recipe.build())).unwrap();
        assert_eq!(archive.len(), expected);
        assert_eq!(
            archive
                .file_names()
                .filter(|name| !name.ends_with('/'))
                .count(),
            1
        );
    }
}

#[test]
fn encrypted_recipes_exercise_distinct_prf_fallbacks_and_valid_ciphertext() {
    let mut ciphertexts = Vec::new();
    for (recipe, sha256_prf) in [
        (Recipe::EncryptedSha1, false),
        (Recipe::EncryptedSha256, true),
    ] {
        let mut archive = ZipArchive::new(Cursor::new(recipe.build())).unwrap();
        let mut manifest = String::new();
        std::io::Read::read_to_string(
            &mut archive.by_name("META-INF/manifest.xml").unwrap(),
            &mut manifest,
        )
        .unwrap();
        assert!(manifest.contains("encryption-data"));
        assert!(manifest.contains("aes256-cbc"));
        assert!(manifest.contains("sha256-1k"));
        let mut member = archive.by_name("Contents/section0.xml").unwrap();
        assert_eq!(member.size() % 16, 0);
        let mut ciphertext = Vec::new();
        std::io::Read::read_to_end(&mut member, &mut ciphertext).unwrap();
        let plaintext = decrypt_fixture_section(&ciphertext, sha256_prf).unwrap();
        assert!(plaintext.starts_with(b"<?xml version=\"1.0\" encoding=\"UTF-8\"?>"));
        assert!(
            String::from_utf8(plaintext)
                .unwrap()
                .contains("First section")
        );
        assert!(decrypt_fixture_section(&ciphertext, !sha256_prf).is_none());
        ciphertexts.push(ciphertext);
    }
    assert_ne!(ciphertexts[0], ciphertexts[1]);
}

fn decrypt_fixture_section(ciphertext: &[u8], sha256_prf: bool) -> Option<Vec<u8>> {
    let start_key = Sha256::digest(b"fixture-password");
    let mut key = [0_u8; 32];
    if sha256_prf {
        pbkdf2_hmac::<Sha256>(&start_key, &[0x11; 16], 1024, &mut key);
    } else {
        pbkdf2_hmac::<Sha1>(&start_key, &[0x11; 16], 1024, &mut key);
    }
    let mut buffer = ciphertext.to_vec();
    let raw = cbc::Decryptor::<Aes256>::new(&key.into(), &[0x22; 16].into())
        .decrypt_padded::<NoPadding>(&mut buffer)
        .ok()?;
    let mut plain = Vec::new();
    std::io::Read::read_to_end(&mut DeflateDecoder::new(raw), &mut plain).ok()?;
    plain.starts_with(b"<?xml").then_some(plain)
}

#[test]
fn fixture_sha256_values_are_pinned() {
    const EXPECTED: [&str; 12] = [
        "edf2d5dea68e88ff2c101df035bc6b8487a71100c8709b85276ba89054c3ae98",
        "4f3a9fcbaad1156c0b8f53622369bbef19a924bcb6e384fa730efa49e38d9b9d",
        "1ad3b674850b0b87d742bce66775d1e5637d9cb14bc08677a8d1f95fbd470243",
        "62c89c7f0dac1c00fd15199692825bbaaa8b17a75de41e4224831c94c8b7b58a",
        "5e6d6dd4ef537d993a5d7f0296434a4d0939806dbbbf87d9bd3c7262c4efd4e6",
        "4cfd465818a4ac733d6333543d0930f912287642a77a006aecbc95df5e1d306f",
        "a579b3fa7c0d1b4fea4eedf327ad4e2a1bd21b0755c97ae03b9403d5663a306f",
        "2dac442e805b8cfc1baf523899dae2906fc274be05c662f7131e58436f19ab00",
        "7bb57a6512ea2170cd16ce4f0ecd4299344e4254072476f28e9d5f47aaf4a2d9",
        "1e1944755e0cd9c255c76e7e620665de71e57df3f2dba115a459a9f683f33c7a",
        "cbdb78eb1eebf5f402495ec23639687446e414b9f90e85dbd2ba6b360c631277",
        "2813d4bad568a2147394ef70a939d85e46428686597d8ffe914891f951c629ba",
    ];
    for (recipe, expected) in Recipe::ALL.into_iter().zip(EXPECTED) {
        let digest = Sha256::digest(recipe.build())
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect::<String>();
        assert_eq!(digest, expected, "{}", recipe.id());
    }
}

#[test]
fn export_generated_fixture_bytes_for_oracle_capture() {
    let Ok(destination) = std::env::var("PARUSTER_HWPX_FIXTURE_EXPORT") else {
        return;
    };
    std::fs::create_dir_all(&destination).unwrap();
    for recipe in Recipe::ALL {
        let path = std::path::Path::new(&destination).join(format!("{}.hwpx", recipe.id()));
        std::fs::write(path, recipe.build()).unwrap();
    }
}
