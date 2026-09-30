use super::decrypt_package;
use crate::hwpx::package::Package;
use aes::{
    Aes256,
    cipher::{BlockModeEncrypt, KeyIvInit, block_padding::NoPadding},
};
use flate2::{Compression, write::DeflateEncoder};
use kordoc_ir::ErrorCode;
use sha1::Sha1;
use sha2::{Digest, Sha256};
use std::io::{Cursor, Read, Write};
use zip::{ZipArchive, ZipWriter, write::SimpleFileOptions};

type AesCbcEncryptor = cbc::Encryptor<Aes256>;

fn b64(bytes: &[u8]) -> String {
    const TABLE: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::new();
    for chunk in bytes.chunks(3) {
        let n = (u32::from(chunk[0]) << 16)
            | (u32::from(*chunk.get(1).unwrap_or(&0)) << 8)
            | u32::from(*chunk.get(2).unwrap_or(&0));
        out.push(TABLE[((n >> 18) & 63) as usize] as char);
        out.push(TABLE[((n >> 12) & 63) as usize] as char);
        out.push(if chunk.len() > 1 {
            TABLE[((n >> 6) & 63) as usize] as char
        } else {
            '='
        });
        out.push(if chunk.len() > 2 {
            TABLE[(n & 63) as usize] as char
        } else {
            '='
        });
    }
    out
}

pub(crate) fn encrypted_archive(prf_sha256: bool) -> Vec<u8> {
    let plain = b"<root>decrypted test</root>";
    let mut deflater = DeflateEncoder::new(Vec::new(), Compression::default());
    deflater.write_all(plain).unwrap();
    encrypted_archive_with_raw(prf_sha256, deflater.finish().unwrap())
}

fn encrypted_archive_with_raw(prf_sha256: bool, mut deflated: Vec<u8>) -> Vec<u8> {
    let plain = b"<root>decrypted test</root>";
    deflated.resize(deflated.len().div_ceil(16) * 16, 0);
    let salt = [0x11; 8];
    let iv = [0x22; 16];
    let password_hash = Sha256::digest(b"secret");
    let mut key = [0u8; 32];
    if prf_sha256 {
        pbkdf2::pbkdf2_hmac::<Sha256>(&password_hash, &salt, 2, &mut key);
    } else {
        pbkdf2::pbkdf2_hmac::<Sha1>(&password_hash, &salt, 2, &mut key);
    }
    let deflated_len = deflated.len();
    let encrypted = AesCbcEncryptor::new_from_slices(&key, &iv)
        .unwrap()
        .encrypt_padded::<NoPadding>(&mut deflated, deflated_len)
        .unwrap()
        .to_vec();
    let checksum = Sha256::digest(plain);
    let manifest = format!(
        "<manifest><file-entry full-path=\"secret.xml\"><encryption-data checksum-type=\"urn:oasis:names:tc:opendocument:xmlns:manifest:1.0#sha256-1k\" checksum=\"{}\"><algorithm algorithm-name=\"http://www.w3.org/2001/04/xmlenc#aes256-cbc\" initialisation-vector=\"{}\"/><start-key-generation start-key-generation-name=\"http://www.w3.org/2000/09/xmldsig#sha256\" key-size=\"32\"/><key-derivation key-derivation-name=\"http://www.w3.org/2000/09/xmldsig#pbkdf2\" iteration-count=\"2\" key-size=\"32\" salt=\"{}\"/></encryption-data></file-entry></manifest>",
        b64(&checksum),
        b64(&iv),
        b64(&salt)
    );
    let mut writer = ZipWriter::new(Cursor::new(Vec::new()));
    writer
        .start_file("META-INF/manifest.xml", SimpleFileOptions::default())
        .unwrap();
    writer.write_all(manifest.as_bytes()).unwrap();
    writer
        .start_file("secret.xml", SimpleFileOptions::default())
        .unwrap();
    writer.write_all(&encrypted).unwrap();
    writer.finish().unwrap().into_inner()
}

fn metadata_only_archive(iterations: &[u32], algorithm: &str) -> Vec<u8> {
    metadata_only_archive_with_kdf(iterations, algorithm, "PBKDF2")
}

fn metadata_only_archive_with_kdf(iterations: &[u32], algorithm: &str, kdf: &str) -> Vec<u8> {
    let entries = iterations.iter().enumerate().map(|(index, count)| format!(
        "<file-entry full-path=\"secret{index}.xml\"><encryption-data checksum-type=\"urn:oasis:names:tc:opendocument:xmlns:manifest:1.0#sha256-1k\" checksum=\"AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA=\"><algorithm algorithm-name=\"{algorithm}\" initialisation-vector=\"AAAAAAAAAAAAAAAAAAAAAA==\"/><start-key-generation start-key-generation-name=\"http://www.w3.org/2000/09/xmldsig#sha256\" key-size=\"32\"/><key-derivation key-derivation-name=\"{kdf}\" iteration-count=\"{count}\" key-size=\"32\" salt=\"AQ==\"/></encryption-data></file-entry>"
    )).collect::<String>();
    let mut writer = ZipWriter::new(Cursor::new(Vec::new()));
    writer
        .start_file("META-INF/manifest.xml", SimpleFileOptions::default())
        .unwrap();
    write!(writer, "<manifest>{entries}</manifest>").unwrap();
    for index in 0..iterations.len() {
        writer
            .start_file(format!("secret{index}.xml"), SimpleFileOptions::default())
            .unwrap();
        writer.write_all(&[0u8; 16]).unwrap();
    }
    writer.finish().unwrap().into_inner()
}

#[test]
fn encrypted_manifest_requires_password_before_member_access() {
    let mut writer = ZipWriter::new(Cursor::new(Vec::new()));
    writer
        .start_file("META-INF/manifest.xml", SimpleFileOptions::default())
        .unwrap();
    writer.write_all(br#"<manifest><file-entry full-path="Contents/secret.xml"><encryption-data checksum-type="sha256-1k" checksum="AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA="><algorithm algorithm-name="http://www.w3.org/2001/04/xmlenc#aes256-cbc" initialisation-vector="AAAAAAAAAAAAAAAAAAAAAA=="/><start-key-generation start-key-generation-name="http://www.w3.org/2000/09/xmldsig#sha256"/><key-derivation key-derivation-name="PBKDF2" iteration-count="1" key-size="32" salt="AQ=="/></encryption-data></file-entry></manifest>"#).unwrap();
    writer
        .start_file("Contents/secret.xml", SimpleFileOptions::default())
        .unwrap();
    writer.write_all(&[0u8; 16]).unwrap();
    let bytes = writer.finish().unwrap().into_inner();
    let mut package = Package::open(&bytes).unwrap();
    let error = decrypt_package(&mut package, None).unwrap_err();
    assert_eq!(error.code, ErrorCode::Encrypted);
}

#[test]
fn decrypts_odf_sha1_and_sha256_prfs() {
    for prf_sha256 in [false, true] {
        let bytes = encrypted_archive(prf_sha256);
        let mut package = Package::open(&bytes).unwrap();
        assert_eq!(decrypt_package(&mut package, Some("secret")).unwrap(), 1);
        assert_eq!(
            package.read("secret.xml").unwrap().unwrap(),
            b"<root>decrypted test</root>"
        );
    }
}

#[test]
fn rejects_truncated_deflate_but_accepts_arbitrary_cbc_alignment_bytes() {
    let plain = b"<root>decrypted test</root>";
    let mut deflater = DeflateEncoder::new(Vec::new(), Compression::default());
    deflater.write_all(plain).unwrap();
    let compressed = deflater.finish().unwrap();

    let mut truncated = compressed[..compressed.len() - 1].to_vec();
    while !truncated.len().is_multiple_of(16) {
        truncated.push(0xff);
    }
    let bytes = encrypted_archive_with_raw(false, truncated);
    let mut package = Package::open(&bytes).unwrap();
    assert_eq!(
        decrypt_package(&mut package, Some("secret"))
            .unwrap_err()
            .code,
        ErrorCode::Encrypted
    );

    let mut trailing = compressed.clone();
    trailing.push(0x7f);
    let bytes = encrypted_archive_with_raw(false, trailing);
    let mut package = Package::open(&bytes).unwrap();
    assert_eq!(decrypt_package(&mut package, Some("secret")).unwrap(), 1);

    let mut excessive_tail = compressed;
    excessive_tail.extend_from_slice(&[0xff; 32]);
    let bytes = encrypted_archive_with_raw(false, excessive_tail);
    let mut package = Package::open(&bytes).unwrap();
    assert_eq!(
        decrypt_package(&mut package, Some("secret"))
            .unwrap_err()
            .code,
        ErrorCode::Encrypted
    );
}

#[test]
fn wrong_password_is_encrypted_without_secret_detail() {
    let bytes = encrypted_archive(false);
    let mut package = Package::open(&bytes).unwrap();
    let error = decrypt_package(&mut package, Some("wrong")).unwrap_err();
    assert_eq!(error.code, ErrorCode::Encrypted);
    assert!(!error.message.contains("wrong"));
}

#[test]
fn rejects_per_entry_and_aggregate_iteration_limits_before_derivation() {
    let per_entry =
        metadata_only_archive(&[1_000_001], "http://www.w3.org/2001/04/xmlenc#aes256-cbc");
    let mut package = Package::open(&per_entry).unwrap();
    assert_eq!(
        decrypt_package(&mut package, None).unwrap_err().code,
        ErrorCode::Corrupted
    );

    let inclusive = metadata_only_archive(
        &[1_000_000, 1_000_000, 1_000_000, 1_000_000],
        "http://www.w3.org/2001/04/xmlenc#aes256-cbc",
    );
    let mut package = Package::open(&inclusive).unwrap();
    assert_eq!(
        decrypt_package(&mut package, None).unwrap_err().code,
        ErrorCode::Encrypted
    );

    let aggregate = metadata_only_archive(
        &[1_000_000, 1_000_000, 1_000_000, 1_000_000, 1],
        "http://www.w3.org/2001/04/xmlenc#aes256-cbc",
    );
    let mut package = Package::open(&aggregate).unwrap();
    assert_eq!(
        decrypt_package(&mut package, None).unwrap_err().code,
        ErrorCode::Corrupted
    );
}

#[test]
fn decrypts_all_or_none_when_a_later_member_fails_checksum() {
    let single = encrypted_archive(false);
    let mut source = ZipArchive::new(Cursor::new(single)).unwrap();
    let mut manifest = String::new();
    source
        .by_name("META-INF/manifest.xml")
        .unwrap()
        .read_to_string(&mut manifest)
        .unwrap();
    let mut ciphertext = Vec::new();
    source
        .by_name("secret.xml")
        .unwrap()
        .read_to_end(&mut ciphertext)
        .unwrap();
    let first = manifest
        .trim_start_matches("<manifest>")
        .trim_end_matches("</manifest>");
    let good_checksum = b64(&Sha256::digest(b"<root>decrypted test</root>"));
    let second = first.replace("secret.xml", "secret2.xml").replace(
        &good_checksum,
        "AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA=",
    );
    let combined = format!("<manifest>{first}{second}</manifest>");
    let mut writer = ZipWriter::new(Cursor::new(Vec::new()));
    writer
        .start_file("META-INF/manifest.xml", SimpleFileOptions::default())
        .unwrap();
    writer.write_all(combined.as_bytes()).unwrap();
    writer
        .start_file("secret.xml", SimpleFileOptions::default())
        .unwrap();
    writer.write_all(&ciphertext).unwrap();
    writer
        .start_file("secret2.xml", SimpleFileOptions::default())
        .unwrap();
    writer.write_all(&ciphertext).unwrap();
    let bytes = writer.finish().unwrap().into_inner();
    let mut package = Package::open(&bytes).unwrap();
    assert_eq!(
        decrypt_package(&mut package, Some("secret"))
            .unwrap_err()
            .code,
        ErrorCode::Encrypted
    );
    assert_eq!(
        package.read("secret.xml").unwrap_err().code,
        ErrorCode::Encrypted
    );
}

#[test]
fn rejects_unsupported_crypto_algorithm_before_requesting_password() {
    let bytes = metadata_only_archive(&[2], "urn:unsupported");
    let mut package = Package::open(&bytes).unwrap();
    assert_eq!(
        decrypt_package(&mut package, None).unwrap_err().code,
        ErrorCode::Corrupted
    );
}

#[test]
fn accepts_standard_and_oracle_pbkdf2_name_forms() {
    for name in ["http://www.w3.org/2000/09/xmldsig#pbkdf2", "PBKDF2"] {
        let bytes = metadata_only_archive_with_kdf(
            &[2],
            "http://www.w3.org/2001/04/xmlenc#aes256-cbc",
            name,
        );
        let mut package = Package::open(&bytes).unwrap();
        assert_eq!(
            decrypt_package(&mut package, None).unwrap_err().code,
            ErrorCode::Encrypted
        );
    }
}
