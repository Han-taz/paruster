use super::Package;
use kordoc_ir::{ErrorCode, WarningCode};
use std::io::{Cursor, Write};
use zip::write::SimpleFileOptions;
use zip::{CompressionMethod, ZipWriter};

fn archive(entries: &[(&str, &[u8])]) -> Vec<u8> {
    let mut writer = ZipWriter::new(Cursor::new(Vec::new()));
    let options = SimpleFileOptions::default().compression_method(CompressionMethod::Stored);
    for (name, contents) in entries {
        writer.start_file(*name, options).unwrap();
        writer.write_all(contents).unwrap();
    }
    writer.finish().unwrap().into_inner()
}

fn archive_with_directories(count: usize) -> Vec<u8> {
    let mut writer = ZipWriter::new(Cursor::new(Vec::new()));
    let options = SimpleFileOptions::default().compression_method(CompressionMethod::Stored);
    writer.start_file("mimetype", options).unwrap();
    for n in 0..count {
        writer.add_directory(format!("d{n}/"), options).unwrap();
    }
    writer.finish().unwrap().into_inner()
}

fn archive_with_duplicate_central_name(first: &str, second: &str) -> Vec<u8> {
    assert_eq!(first.len(), second.len());
    let mut bytes = archive(&[(first, b"one"), (second, b"two")]);
    let records: Vec<usize> = bytes
        .windows(4)
        .enumerate()
        .filter_map(|(position, window)| (window == b"PK\x01\x02").then_some(position))
        .collect();
    assert_eq!(records.len(), 2);
    let second_name_start = records[1] + 46;
    bytes[second_name_start..second_name_start + first.len()].copy_from_slice(first.as_bytes());
    bytes
}

fn error_code<T>(result: Result<T, kordoc_ir::KordocError>) -> ErrorCode {
    result.err().unwrap().code
}

fn locate_first_payload(bytes: &[u8]) -> usize {
    assert_eq!(&bytes[..4], b"PK\x03\x04");
    30 + u16::from_le_bytes([bytes[26], bytes[27]]) as usize
        + u16::from_le_bytes([bytes[28], bytes[29]]) as usize
}

fn read_u32(bytes: &[u8], position: usize) -> u32 {
    u32::from_le_bytes(bytes[position..position + 4].try_into().unwrap())
}

fn read_u16(bytes: &[u8], position: usize) -> u16 {
    u16::from_le_bytes(bytes[position..position + 2].try_into().unwrap())
}

fn read_u64(bytes: &[u8], position: usize) -> u64 {
    u64::from_le_bytes(bytes[position..position + 8].try_into().unwrap())
}

fn write_u16(bytes: &mut [u8], position: usize, value: u16) {
    bytes[position..position + 2].copy_from_slice(&value.to_le_bytes());
}

fn write_u32(bytes: &mut [u8], position: usize, value: u32) {
    bytes[position..position + 4].copy_from_slice(&value.to_le_bytes());
}

fn upgrade_to_zip64(mut bytes: Vec<u8>) -> Vec<u8> {
    let eocd = bytes
        .windows(4)
        .rposition(|window| window == b"PK\x05\x06")
        .unwrap();
    let entries = read_u16(&bytes, eocd + 10) as u64;
    let central_size = read_u32(&bytes, eocd + 12) as u64;
    let central_offset = read_u32(&bytes, eocd + 16) as u64;
    let zip64_position = eocd as u64;
    let mut end64 = Vec::with_capacity(76);
    end64.extend_from_slice(&0x0606_4b50u32.to_le_bytes());
    end64.extend_from_slice(&44u64.to_le_bytes());
    end64.extend_from_slice(&45u16.to_le_bytes());
    end64.extend_from_slice(&45u16.to_le_bytes());
    end64.extend_from_slice(&0u32.to_le_bytes());
    end64.extend_from_slice(&0u32.to_le_bytes());
    end64.extend_from_slice(&entries.to_le_bytes());
    end64.extend_from_slice(&entries.to_le_bytes());
    end64.extend_from_slice(&central_size.to_le_bytes());
    end64.extend_from_slice(&central_offset.to_le_bytes());
    end64.extend_from_slice(&0x0706_4b50u32.to_le_bytes());
    end64.extend_from_slice(&0u32.to_le_bytes());
    end64.extend_from_slice(&zip64_position.to_le_bytes());
    end64.extend_from_slice(&1u32.to_le_bytes());
    let mut classic = bytes.split_off(eocd);
    write_u16(&mut classic, 8, u16::MAX);
    write_u16(&mut classic, 10, u16::MAX);
    write_u32(&mut classic, 12, u32::MAX);
    write_u32(&mut classic, 16, u32::MAX);
    bytes.extend_from_slice(&end64);
    bytes.extend_from_slice(&classic);
    bytes
}

#[test]
fn rejects_corrupt_central_directory_before_recovery() {
    let mut bytes = archive(&[("Contents/section0.xml", b"<section/>")]);
    let central = bytes
        .windows(4)
        .position(|window| window == b"PK\x01\x02")
        .unwrap();
    bytes[central] ^= 0xff;
    assert_eq!(error_code(Package::open(&bytes)), ErrorCode::ZipBomb);
}

#[test]
fn validates_single_disk_zip64_records_before_opening_members() {
    let bytes = upgrade_to_zip64(archive(&[("mimetype", b"application/vnd.hancom.hwpx")]));
    let locator = bytes
        .windows(4)
        .rposition(|window| window == b"PK\x06\x07")
        .unwrap();
    assert_eq!(read_u64(&bytes, locator + 8), (locator - 56) as u64);
    let mut package = Package::open(&bytes).unwrap();
    assert_eq!(
        package.read("mimetype").unwrap().unwrap(),
        b"application/vnd.hancom.hwpx"
    );

    let mut malformed = archive(&[("mimetype", b"x")]);
    let eocd = malformed
        .windows(4)
        .rposition(|window| window == b"PK\x05\x06")
        .unwrap();
    write_u16(&mut malformed, eocd + 8, u16::MAX);
    write_u16(&mut malformed, eocd + 10, u16::MAX);
    write_u32(&mut malformed, eocd + 12, u32::MAX);
    write_u32(&mut malformed, eocd + 16, u32::MAX);
    assert_eq!(error_code(Package::open(&malformed)), ErrorCode::ZipBomb);

    let mut multi_disk = upgrade_to_zip64(archive(&[("mimetype", b"x")]));
    let locator = multi_disk
        .windows(4)
        .rposition(|window| window == b"PK\x06\x07")
        .unwrap();
    write_u32(&mut multi_disk, locator + 4, 1);
    assert_eq!(error_code(Package::open(&multi_disk)), ErrorCode::ZipBomb);
}

#[test]
fn counts_directory_and_duplicate_records_toward_500() {
    let bytes = archive_with_directories(499);
    let mut at_limit = Package::open(&bytes).unwrap();
    assert_eq!(at_limit.read("d0/").unwrap(), Some(Vec::new()));
    assert_eq!(
        error_code(Package::open(&archive_with_directories(500))),
        ErrorCode::ZipBomb
    );
    let duplicate =
        archive_with_duplicate_central_name("Contents/section0.xml", "Contents/section1.xml");
    assert_eq!(error_code(Package::open(&duplicate)), ErrorCode::ZipBomb);
}

#[test]
fn accepts_500_records_rejects_501() {
    assert!(Package::open(&archive_with_directories(499)).is_ok());
    assert_eq!(
        error_code(Package::open(&archive_with_directories(500))),
        ErrorCode::ZipBomb
    );
}

#[test]
fn rejects_traversal_absolute_backslash_and_duplicate_names() {
    for name in [
        "../escape",
        "/absolute",
        "C:/drive",
        "Contents\\section0.xml",
    ] {
        assert_eq!(
            error_code(Package::open(&archive(&[(name, b"x")]))),
            ErrorCode::ZipBomb,
            "{name}"
        );
    }
    let duplicate = archive_with_duplicate_central_name("mimetype", "mimetypf");
    assert_eq!(error_code(Package::open(&duplicate)), ErrorCode::ZipBomb);
}

#[test]
fn meters_all_members_at_256_mib() {
    const LIMIT: usize = 256 * 1024 * 1024;
    let payload = vec![b'x'; LIMIT];
    let bytes = archive(&[
        ("Contents/first.bin", &payload),
        ("Contents/second.bin", b"!"),
    ]);
    let mut package = Package::open(&bytes).unwrap();
    assert_eq!(
        package.read("Contents/first.bin").unwrap().unwrap().len(),
        LIMIT
    );
    assert_eq!(
        package.read("Contents/first.bin").unwrap().unwrap().len(),
        LIMIT
    );
    assert_eq!(
        error_code(package.read("Contents/second.bin")),
        ErrorCode::DecompressionBomb
    );
}

#[test]
fn ciphertext_and_decrypted_output_use_separate_shared_meters() {
    let bytes = archive(&[("Contents/section0.xml", b"cipher")]);
    let mut package = Package::open(&bytes).unwrap();
    assert_eq!(
        package.read_ciphertext("Contents/section0.xml").unwrap(),
        Some(b"cipher".to_vec())
    );

    const LIMIT: usize = 256 * 1024 * 1024;
    package
        .charge_decrypted("Contents/section0.xml", LIMIT)
        .unwrap();
    let output = [0u8; 1];
    let result = package.charge_decrypted("Contents/section0.xml", output.len());
    assert_eq!(result.unwrap_err().code, ErrorCode::DecompressionBomb);
    assert_eq!(output, [0]);
}

#[test]
fn finalized_decrypted_member_cannot_be_charged_again() {
    let bytes = archive(&[("Contents/section0.xml", b"cipher")]);
    let mut package = Package::open(&bytes).unwrap();
    package
        .read_ciphertext("Contents/section0.xml")
        .unwrap()
        .unwrap();
    package
        .charge_decrypted("Contents/section0.xml", 8)
        .unwrap();
    package.finish_decrypted("Contents/section0.xml").unwrap();

    assert_eq!(
        error_code(package.charge_decrypted("Contents/section0.xml", 8)),
        ErrorCode::Corrupted
    );
}

#[test]
fn rejects_member_crc_or_extent_damage_without_local_header_scan() {
    let mut crc_bytes = archive(&[("Contents/section0.xml", b"<section/>")]);
    let payload = locate_first_payload(&crc_bytes);
    crc_bytes[payload] ^= 0x01;
    let mut package = Package::open(&crc_bytes).unwrap();
    assert_eq!(
        error_code(package.read("Contents/section0.xml")),
        ErrorCode::Corrupted
    );

    let mut extent_bytes = archive(&[("Contents/section0.xml", b"<section/>")]);
    let central = extent_bytes
        .windows(4)
        .position(|window| window == b"PK\x01\x02")
        .unwrap();
    extent_bytes[central + 42..central + 46].copy_from_slice(&u32::MAX.to_le_bytes());
    assert_eq!(error_code(Package::open(&extent_bytes)), ErrorCode::ZipBomb);
}

#[test]
fn retains_other_sections_after_bounded_member_failure() {
    let mut bytes = archive(&[
        ("Contents/section0.xml", b"zero"),
        ("Contents/section1.xml", b"one"),
    ]);
    let first_payload = locate_first_payload(&bytes);
    bytes[first_payload] ^= 0x01;
    let mut package = Package::open(&bytes).unwrap();
    assert_eq!(
        package.read_optional("Contents/section0.xml").unwrap(),
        None
    );
    assert_eq!(
        package.read("Contents/section1.xml").unwrap().unwrap(),
        b"one"
    );
    let warnings = package.take_warnings();
    assert_eq!(warnings.len(), 1);
    assert_eq!(warnings[0].code, WarningCode::BrokenZipRecovery);
}

#[test]
fn orders_sections_by_manifest_spine_then_numeric_fallback() {
    let manifest = b"<package><manifest><item id='s0' href='section0.xml'/><item id='s1' href='section1.xml'/></manifest><rogue><itemref idref='s0'/></rogue><spine><itemref idref='s1'/><itemref idref='s0'/></spine></package>";
    let with_spine = archive(&[
        ("Contents/content.hpf", manifest),
        ("Contents/section0.xml", b"zero"),
        ("Contents/section1.xml", b"one"),
    ]);
    let mut package = Package::open(&with_spine).unwrap();
    assert_eq!(
        package.section_paths().unwrap(),
        ["Contents/section1.xml", "Contents/section0.xml"]
    );

    let without_spine = archive(&[
        ("Contents/section10.xml", b"ten"),
        ("Contents/section2.xml", b"two"),
    ]);
    let mut package = Package::open(&without_spine).unwrap();
    assert_eq!(
        package.section_paths().unwrap(),
        ["Contents/section2.xml", "Contents/section10.xml"]
    );

    let manifest_without_spine = archive(&[
        ("Contents/content.hpf", b"<package><manifest/></package>"),
        ("Contents/section10.xml", b"ten"),
        ("Contents/section2.xml", b"two"),
    ]);
    let mut package = Package::open(&manifest_without_spine).unwrap();
    assert_eq!(
        package.section_paths().unwrap(),
        ["Contents/section2.xml", "Contents/section10.xml"]
    );

    let malformed_manifest = archive(&[
        ("Contents/content.hpf", b"<package><spine>"),
        ("Contents/section0.xml", b"zero"),
    ]);
    let mut package = Package::open(&malformed_manifest).unwrap();
    assert_eq!(error_code(package.section_paths()), ErrorCode::Corrupted);
}
