#[path = "../src/hwpx/mod.rs"]
mod hwpx;
#[path = "support/hwpx_fixture.rs"]
mod hwpx_fixture;

use std::io::{Cursor, Write};

use aes::Aes256;
use aes::cipher::{BlockModeDecrypt, KeyIvInit, block_padding::NoPadding};
use flate2::read::DeflateDecoder;
use hwpx_fixture::Recipe;
use pbkdf2::pbkdf2_hmac;
use sha1::Sha1;
use sha2::{Digest, Sha256};
use zip::write::SimpleFileOptions;
use zip::{CompressionMethod, ZipArchive, ZipWriter};

fn image_page_fixture() -> Vec<u8> {
    let base = Recipe::Minimal.build();
    let mut archive = ZipArchive::new(Cursor::new(base)).unwrap();
    let mut writer = ZipWriter::new(Cursor::new(Vec::new()));
    let options = SimpleFileOptions::default().compression_method(CompressionMethod::Stored);
    let section = r#"<?xml version="1.0" encoding="UTF-8"?><hs:sec xmlns:hs="http://www.hancom.co.kr/hwpml/2011/section" xmlns:hp="http://www.hancom.co.kr/hwpml/2011/paragraph"><hp:p><hp:linesegarray><hp:lineseg vertpos="0"/></hp:linesegarray><hp:run><hp:t>page one</hp:t><hp:pic><hp:imgRect binaryItemIDRef="one.png"/></hp:pic></hp:run></hp:p><hp:p><hp:linesegarray><hp:lineseg vertpos="0"/></hp:linesegarray><hp:run><hp:t>page two</hp:t><hp:tbl><hp:tr><hp:tc><hp:cellAddr rowAddr="0" colAddr="0"/><hp:subList><hp:p><hp:run><hp:pic><hp:imgRect binaryItemIDRef="two.png"/></hp:pic></hp:run></hp:p></hp:subList></hp:tc></hp:tr></hp:tbl></hp:run></hp:p></hs:sec>"#;
    for index in 0..archive.len() {
        let mut member = archive.by_index(index).unwrap();
        let name = member.name().to_owned();
        let mut data = Vec::new();
        std::io::Read::read_to_end(&mut member, &mut data).unwrap();
        writer.start_file(&name, options).unwrap();
        if name == "Contents/section0.xml" {
            writer.write_all(section.as_bytes()).unwrap();
        } else {
            writer.write_all(&data).unwrap();
        }
    }
    for path in [
        "BinData/one.png",
        "BinData/two.png",
        "BinData/unused.png",
        "Assets/BinData/deep.png",
    ] {
        writer.start_file(path, options).unwrap();
        writer.write_all(b"\x89PNG\r\n\x1a\nimage").unwrap();
    }
    writer.finish().unwrap().into_inner()
}

fn without_member(bytes: Vec<u8>, omitted: &str) -> Vec<u8> {
    let mut archive = ZipArchive::new(Cursor::new(bytes)).unwrap();
    let mut writer = ZipWriter::new(Cursor::new(Vec::new()));
    let options = SimpleFileOptions::default().compression_method(CompressionMethod::Stored);
    for index in 0..archive.len() {
        let mut member = archive.by_index(index).unwrap();
        if member.name() == omitted {
            continue;
        }
        let name = member.name().to_owned();
        let mut data = Vec::new();
        std::io::Read::read_to_end(&mut member, &mut data).unwrap();
        writer.start_file(name, options).unwrap();
        writer.write_all(&data).unwrap();
    }
    writer.finish().unwrap().into_inner()
}

fn manifest_marking_content_encrypted() -> Vec<u8> {
    let bytes = Recipe::EncryptedSha1.build();
    let mut archive = ZipArchive::new(Cursor::new(bytes)).unwrap();
    let mut writer = ZipWriter::new(Cursor::new(Vec::new()));
    let options = SimpleFileOptions::default().compression_method(CompressionMethod::Stored);
    for index in 0..archive.len() {
        let mut member = archive.by_index(index).unwrap();
        let name = member.name().to_owned();
        let mut data = Vec::new();
        std::io::Read::read_to_end(&mut member, &mut data).unwrap();
        if name == "META-INF/manifest.xml" {
            let xml = String::from_utf8(data).unwrap().replace(
                "odf:full-path=\"Contents/section0.xml\"",
                "odf:full-path=\"Contents/content.hpf\"",
            );
            data = xml.into_bytes();
        }
        writer.start_file(name, options).unwrap();
        writer.write_all(&data).unwrap();
    }
    writer.finish().unwrap().into_inner()
}

#[test]
fn selected_hwpx_page_does_not_read_or_return_other_page_assets() {
    let options = kordoc_ir::ParseOptions {
        pages: Some(kordoc_ir::PageSelection::Numbers(vec![
            kordoc_ir::PageNumber::new(2.0).unwrap(),
        ])),
        ..kordoc_ir::ParseOptions::default()
    };
    let parsed = hwpx::parse_hwpx(&image_page_fixture(), &options).unwrap();
    assert_eq!(parsed.page_count, Some(2));
    assert_eq!(parsed.metadata.as_ref().unwrap().page_count, Some(2));
    let images = parsed.images.unwrap();
    assert_eq!(images.len(), 1);
    assert_eq!(images[0].source.as_deref(), Some("BinData/two.png"));
    assert_eq!(images[0].filename, "image_001.png");
    assert_eq!(
        parsed
            .blocks
            .iter()
            .filter_map(|block| block.text.as_deref())
            .filter(|text| text.starts_with("page "))
            .collect::<Vec<_>>(),
        ["page two"]
    );
    assert!(parsed.warnings.is_none());
}

#[test]
fn full_hwpx_parse_sweeps_unreferenced_bindata() {
    let parsed =
        hwpx::parse_hwpx(&image_page_fixture(), &kordoc_ir::ParseOptions::default()).unwrap();
    let images = parsed.images.unwrap();
    assert_eq!(images.len(), 4);
    assert_eq!(images[0].source.as_deref(), Some("BinData/one.png"));
    assert_eq!(images[1].source.as_deref(), Some("BinData/two.png"));
    assert_eq!(images[2].source.as_deref(), Some("BinData/unused.png"));
    assert_eq!(images[3].source.as_deref(), Some("Assets/BinData/deep.png"));
    assert!(parsed.blocks.iter().any(|block| {
        block.text.as_deref() == Some("image_003.png")
            && block
                .image_data
                .as_ref()
                .and_then(|image| image.filename.as_deref())
                == Some("BinData/unused.png")
    }));
}

#[test]
fn private_hwpx_keeps_spine_order_and_isolates_malformed_middle_section() {
    let options = kordoc_ir::ParseOptions::default();
    let reversed = hwpx::parse_hwpx(&Recipe::TwoSectionSpineReversed.build(), &options).unwrap();
    assert_eq!(reversed.blocks[0].text.as_deref(), Some("Second section"));
    assert_eq!(reversed.blocks[1].text.as_deref(), Some("First section"));

    let recovered = hwpx::parse_hwpx(&Recipe::MalformedSection.build(), &options).unwrap();
    let texts: Vec<_> = recovered
        .blocks
        .iter()
        .filter_map(|block| block.text.as_deref())
        .collect();
    assert_eq!(texts, ["First section", "Third section"]);
    assert_eq!(recovered.warnings.as_ref().unwrap().len(), 1);
    assert_eq!(
        recovered.warnings.as_ref().unwrap()[0].code,
        kordoc_ir::WarningCode::PartialParse
    );
    assert_eq!(recovered.warnings.as_ref().unwrap()[0].page, Some(2));
    assert_eq!(
        recovered.warnings.as_ref().unwrap()[0].message,
        "섹션 2 파싱 실패: Reporting fatalError \"unclosed xml tag(s): hs:sec, hp:p\" caused KordocError: XML 파싱 실패: unclosed xml tag(s): hs:sec, hp:p"
    );
}

#[test]
fn damaged_section_member_keeps_neighbor_source_pages() {
    let mut bytes = Recipe::MalformedSection.build();
    let mut archive = ZipArchive::new(Cursor::new(bytes.as_slice())).unwrap();
    let offset = archive
        .by_name("Contents/section1.xml")
        .unwrap()
        .data_start()
        .unwrap() as usize;
    bytes[offset] ^= 0x01;
    let parsed = hwpx::parse_hwpx(&bytes, &kordoc_ir::ParseOptions::default()).unwrap();
    let texts: Vec<_> = parsed
        .blocks
        .iter()
        .filter_map(|block| block.text.as_deref())
        .collect();
    assert_eq!(texts, ["First section", "Third section"]);
    assert_eq!(parsed.blocks[0].page_number, Some(1));
    assert_eq!(parsed.blocks[1].page_number, Some(3));
    assert!(
        parsed
            .warnings
            .unwrap()
            .iter()
            .any(|warning| warning.code == kordoc_ir::WarningCode::BrokenZipRecovery)
    );
}

#[test]
fn private_hwpx_metadata_does_not_require_valid_section_xml() {
    let metadata = hwpx::parse_hwpx_metadata(
        &Recipe::OverDepthSection.build(),
        &kordoc_ir::ParseOptions::default(),
    )
    .unwrap();
    assert_eq!(metadata.title.as_deref(), Some("Synthetic HWPX"));
    assert_eq!(metadata.page_count, Some(1));
    for recipe in [Recipe::EncryptedSha1, Recipe::EncryptedSha256] {
        let metadata =
            hwpx::parse_hwpx_metadata(&recipe.build(), &kordoc_ir::ParseOptions::default())
                .unwrap();
        assert_eq!(metadata.title.as_deref(), Some("Synthetic HWPX"));
    }
}

#[test]
fn metadata_only_refuses_manifest_marked_encrypted_content_without_plaintext_read() {
    let error = hwpx::parse_hwpx_metadata(
        &manifest_marking_content_encrypted(),
        &kordoc_ir::ParseOptions::default(),
    )
    .unwrap_err();
    assert_eq!(error.code, kordoc_ir::ErrorCode::Encrypted);
}

#[test]
fn private_hwpx_requires_at_least_one_usable_section() {
    let error = hwpx::parse_hwpx(
        &Recipe::OverDepthSection.build(),
        &kordoc_ir::ParseOptions::default(),
    )
    .unwrap_err();
    assert_eq!(error.code, kordoc_ir::ErrorCode::NoSections);
}

#[test]
fn plain_table_cells_and_captions_do_not_emit_redundant_nested_blocks() {
    let parsed = hwpx::parse_hwpx(
        &Recipe::NestedTable.build(),
        &kordoc_ir::ParseOptions::default(),
    )
    .unwrap();
    let outer = parsed.blocks[0].table.as_ref().unwrap();
    assert_eq!(outer.caption.as_deref(), Some("Synthetic caption"));
    assert!(outer.caption_blocks.is_none());
    let outer_cell = &outer.cells[0][0];
    assert_eq!(outer_cell.blocks.as_ref().unwrap().len(), 2);
    let inner = outer_cell.blocks.as_ref().unwrap()[1]
        .table
        .as_ref()
        .unwrap();
    assert!(inner.caption_blocks.is_none());
    assert!(inner.cells[0][0].blocks.is_none());
}

#[test]
fn private_hwpx_rejects_package_and_critical_xml_faults() {
    let options = kordoc_ir::ParseOptions::default();
    assert_eq!(
        hwpx::parse_hwpx(&Recipe::DirectoryEntry501.build(), &options)
            .unwrap_err()
            .code,
        kordoc_ir::ErrorCode::ZipBomb,
    );
    assert_eq!(
        hwpx::parse_hwpx(&Recipe::OverDepthManifest.build(), &options)
            .unwrap_err()
            .code,
        kordoc_ir::ErrorCode::Corrupted,
    );
    assert_eq!(
        hwpx::parse_hwpx(
            &without_member(Recipe::Minimal.build(), "Contents/content.hpf"),
            &options,
        )
        .unwrap_err()
        .code,
        kordoc_ir::ErrorCode::Corrupted,
    );
}

#[test]
fn private_hwpx_encrypted_entry_points_require_and_accept_password() {
    for recipe in [Recipe::EncryptedSha1, Recipe::EncryptedSha256] {
        let bytes = recipe.build();
        let default = kordoc_ir::ParseOptions::default();
        assert_eq!(
            hwpx::parse_hwpx(&bytes, &default).unwrap_err().code,
            kordoc_ir::ErrorCode::Encrypted
        );
        assert_eq!(
            hwpx::validate_hwpx(&bytes, None).unwrap_err().code,
            kordoc_ir::ErrorCode::Encrypted
        );
        let wrong = kordoc_ir::ParseOptions {
            password: Some("wrong".into()),
            ..default.clone()
        };
        assert_eq!(
            hwpx::parse_hwpx(&bytes, &wrong).unwrap_err().code,
            kordoc_ir::ErrorCode::Encrypted
        );
        let valid = kordoc_ir::ParseOptions {
            password: Some("fixture-password".into()),
            ..default
        };
        assert_eq!(
            hwpx::parse_hwpx(&bytes, &valid).unwrap().blocks[0]
                .text
                .as_deref(),
            Some("First section")
        );
        assert!(
            hwpx::validate_hwpx(&bytes, Some("fixture-password"))
                .unwrap()
                .ok
        );
    }
}

#[test]
fn private_hwpx_entry_points_compose_minimal_fixture() {
    let bytes = Recipe::Minimal.build();
    let parsed = hwpx::parse_hwpx(&bytes, &kordoc_ir::ParseOptions::default()).unwrap();
    assert_eq!(parsed.blocks[0].text.as_deref(), Some("First section"));
    assert_eq!(parsed.page_count, Some(1));
    assert_eq!(
        parsed.metadata.as_ref().and_then(|m| m.title.as_deref()),
        Some("Synthetic HWPX")
    );
    assert!(parsed.images.is_none());
    assert!(parsed.warnings.is_none());

    let metadata = hwpx::parse_hwpx_metadata(&bytes, &kordoc_ir::ParseOptions::default()).unwrap();
    assert_eq!(metadata.title.as_deref(), Some("Synthetic HWPX"));
    assert_eq!(metadata.page_count, Some(1));

    let validation = hwpx::validate_hwpx(&bytes, None).unwrap();
    assert!(validation.ok);
    assert!(validation.issues.is_empty());
    assert_eq!(validation.entry_count, 6);
}

#[test]
fn private_hwpx_result_has_no_markdown_field() {
    let bytes = Recipe::Minimal.build();
    let parsed = hwpx::parse_hwpx(&bytes, &kordoc_ir::ParseOptions::default()).unwrap();
    assert_eq!(parsed.blocks[0].text.as_deref(), Some("First section"));
    let kordoc_ir::ParsedDocument {
        blocks: _,
        page_count: _,
        metadata: _,
        outline: _,
        warnings: _,
        images: _,
        is_image_based: _,
        page_quality: _,
        quality_summary: _,
        page_evidence: _,
    } = parsed;
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
