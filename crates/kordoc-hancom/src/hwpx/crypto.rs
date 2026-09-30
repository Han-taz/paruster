//! Bounded ODF encryption support for encrypted HWPX package members.

#![allow(
    dead_code,
    reason = "H1b crypto substrate is wired by the H3 integration join"
)]

use aes::Aes256;
use aes::cipher::{BlockModeDecrypt, KeyIvInit, block_padding::NoPadding};
use flate2::{Decompress, FlushDecompress, Status};
use kordoc_ir::{ErrorCode, KordocError};
use sha1::Sha1;
use sha2::{Digest, Sha256};
use subtle::ConstantTimeEq;

use super::{package::Package, xml};

const AES256_CBC: &str = "http://www.w3.org/2001/04/xmlenc#aes256-cbc";
const START_KEY_SHA256: &str = "http://www.w3.org/2000/09/xmldsig#sha256";
const MAX_ITERATIONS: u32 = 1_000_000;
const MAX_ITERATION_SUM: u64 = 4_000_000;
const MAX_DECRYPTED_MEMBER: usize = 256 * 1024 * 1024;
const CHECKSUM_PREFIX: usize = 1024;

type AesCbcDecryptor = cbc::Decryptor<Aes256>;

#[derive(Debug, Clone)]
struct EncryptedEntry {
    path: String,
    checksum: Vec<u8>,
    iv: Vec<u8>,
    salt: Vec<u8>,
    iterations: u32,
}

pub(crate) fn decrypt_package(
    package: &mut Package<'_>,
    password: Option<&str>,
) -> Result<usize, KordocError> {
    let Some(manifest) = package.read("META-INF/manifest.xml")? else {
        return Ok(0);
    };
    let root = xml::parse_critical(&manifest)?;
    let mut encrypted = Vec::new();
    let mut iteration_sum = 0u64;
    for entry in root.descendants("file-entry") {
        let Some(data) = entry
            .children
            .iter()
            .find(|child| child.name == "encryption-data")
        else {
            continue;
        };
        let path = entry
            .attr("full-path")
            .ok_or_else(|| corrupted("Encrypted package entry has no path"))?;
        let algorithm = data
            .children
            .iter()
            .find(|n| n.name == "algorithm")
            .ok_or_else(|| unsupported("Encrypted package algorithm is missing"))?;
        if algorithm.attr("algorithm-name") != Some(AES256_CBC) {
            return Err(unsupported("Unsupported HWPX encryption algorithm"));
        }
        let start_key = data
            .children
            .iter()
            .find(|n| n.name == "start-key-generation")
            .ok_or_else(|| unsupported("Encrypted package start key is missing"))?;
        if start_key.attr("start-key-generation-name") != Some(START_KEY_SHA256) {
            return Err(unsupported("Unsupported HWPX start-key algorithm"));
        }
        let checksum_type = data.attr("checksum-type").unwrap_or_default();
        if checksum_type != "sha256-1k"
            && checksum_type != "http://www.w3.org/2000/09/xmldsig#sha256-1k"
            && checksum_type != "urn:oasis:names:tc:opendocument:xmlns:manifest:1.0#sha256-1k"
        {
            return Err(unsupported("Unsupported HWPX checksum algorithm"));
        }
        let key_derivation = data
            .children
            .iter()
            .find(|n| n.name == "key-derivation")
            .ok_or_else(|| unsupported("Encrypted package key derivation is missing"))?;
        if !matches!(
            key_derivation.attr("key-derivation-name"),
            Some("PBKDF2")
                | Some("http://www.w3.org/2000/09/xmldsig#pbkdf2")
                | Some("urn:oasis:names:tc:opendocument:xmlns:manifest:1.0#pbkdf2")
        ) {
            return Err(unsupported("Unsupported HWPX key derivation algorithm"));
        }
        if key_derivation
            .attr("start-key-size")
            .or_else(|| start_key.attr("key-size"))
            .is_some_and(|size| size != "32")
        {
            return Err(unsupported("Unsupported HWPX start-key size"));
        }
        let iterations = parse_u32(key_derivation.attr("iteration-count"))?;
        if iterations == 0 || iterations > MAX_ITERATIONS {
            return Err(unsupported(
                "HWPX encryption iteration count exceeds its limit",
            ));
        }
        if parse_u32(key_derivation.attr("key-size"))? != 32 {
            return Err(unsupported("Unsupported HWPX encryption key size"));
        }
        iteration_sum = iteration_sum
            .checked_add(u64::from(iterations))
            .ok_or_else(|| unsupported("HWPX iteration count overflows"))?;
        let iv = decode_b64(algorithm.attr("initialisation-vector").unwrap_or_default())?;
        let salt = decode_b64(key_derivation.attr("salt").unwrap_or_default())?;
        let checksum = decode_b64(data.attr("checksum").unwrap_or_default())?;
        if iv.len() != 16 || salt.is_empty() || salt.len() > 1024 || checksum.len() != 32 {
            return Err(unsupported("HWPX encryption parameters are invalid"));
        }
        let path = safe_member_path(path)?;
        if !package.contains(&path) {
            return Err(corrupted("Encrypted package member is missing"));
        }
        encrypted.push(EncryptedEntry {
            path,
            checksum,
            iv,
            salt,
            iterations,
        });
    }
    if iteration_sum > MAX_ITERATION_SUM {
        return Err(unsupported(
            "HWPX package iteration budget exceeds its limit",
        ));
    }
    if encrypted.is_empty() {
        return Ok(0);
    }
    let paths: Vec<_> = encrypted.iter().map(|entry| entry.path.clone()).collect();
    package.mark_encrypted_members(&paths)?;
    let password = password.ok_or_else(|| {
        KordocError::new(
            ErrorCode::Encrypted,
            "A password is required to read this HWPX document",
        )
    })?;
    let start_key = Sha256::digest(password.as_bytes());
    let mut staged = Vec::with_capacity(encrypted.len());
    for entry in &encrypted {
        let cipher = package
            .read_ciphertext(&entry.path)?
            .ok_or_else(|| corrupted("Encrypted package member is missing"))?;
        let plain = decrypt_entry(package, &cipher, entry, &start_key)?.ok_or_else(|| {
            KordocError::new(
                ErrorCode::Encrypted,
                "The password is invalid or encrypted data is damaged",
            )
        })?;
        staged.push((entry.path.clone(), plain));
    }
    package.install_decrypted_batch(staged)?;
    Ok(encrypted.len())
}

fn decrypt_entry(
    package: &mut Package<'_>,
    cipher: &[u8],
    entry: &EncryptedEntry,
    start_key: &[u8],
) -> Result<Option<Vec<u8>>, KordocError> {
    if cipher.is_empty() || !cipher.len().is_multiple_of(16) {
        return Ok(None);
    }
    for use_sha256 in [false, true] {
        let mut key = [0u8; 32];
        if use_sha256 {
            pbkdf2::pbkdf2_hmac::<Sha256>(start_key, &entry.salt, entry.iterations, &mut key);
        } else {
            pbkdf2::pbkdf2_hmac::<Sha1>(start_key, &entry.salt, entry.iterations, &mut key);
        }
        let mut decrypted = cipher.to_vec();
        let Some(decryptor) = AesCbcDecryptor::new_from_slices(&key, &entry.iv).ok() else {
            continue;
        };
        let Ok(unpadded) = decryptor.decrypt_padded::<NoPadding>(&mut decrypted) else {
            continue;
        };
        let mut inflater = Decompress::new(false);
        let mut plain = Vec::new();
        let mut chunk = [0u8; 8192];
        let mut failed = false;
        let mut stream_end = false;
        let mut input_offset = 0usize;
        loop {
            let before_in = inflater.total_in();
            let before_out = inflater.total_out();
            let status = match inflater.decompress(
                &unpadded[input_offset..],
                &mut chunk,
                FlushDecompress::None,
            ) {
                Ok(status) => status,
                Err(_) => {
                    failed = true;
                    break;
                }
            };
            let consumed = usize::try_from(inflater.total_in() - before_in)
                .map_err(|_| corrupted("HWPX deflate input counter overflowed"))?;
            let produced = usize::try_from(inflater.total_out() - before_out)
                .map_err(|_| corrupted("HWPX deflate output counter overflowed"))?;
            input_offset = input_offset
                .checked_add(consumed)
                .ok_or_else(|| corrupted("HWPX deflate input counter overflowed"))?;
            if produced > 0 {
                if plain
                    .len()
                    .checked_add(produced)
                    .is_none_or(|len| len > MAX_DECRYPTED_MEMBER)
                {
                    package.rollback_decrypted_attempt(&entry.path)?;
                    return Err(KordocError::new(
                        ErrorCode::DecompressionBomb,
                        "HWPX encrypted member exceeds its plaintext limit",
                    ));
                }
                package.charge_decrypted(&entry.path, produced)?;
                plain.extend_from_slice(&chunk[..produced]);
            }
            if status == Status::StreamEnd {
                let trailing = &unpadded[input_offset..];
                stream_end = trailing.len() <= 16;
                break;
            }
            if consumed == 0 && produced == 0 {
                if input_offset == unpadded.len() {
                    let before_out = inflater.total_out();
                    match inflater.decompress(&[], &mut chunk, FlushDecompress::Finish) {
                        Ok(Status::StreamEnd) => {
                            let produced = usize::try_from(inflater.total_out() - before_out)
                                .map_err(|_| corrupted("HWPX deflate output counter overflowed"))?;
                            if produced > 0 {
                                if plain
                                    .len()
                                    .checked_add(produced)
                                    .is_none_or(|len| len > MAX_DECRYPTED_MEMBER)
                                {
                                    package.rollback_decrypted_attempt(&entry.path)?;
                                    return Err(KordocError::new(
                                        ErrorCode::DecompressionBomb,
                                        "HWPX encrypted member exceeds its plaintext limit",
                                    ));
                                }
                                package.charge_decrypted(&entry.path, produced)?;
                                plain.extend_from_slice(&chunk[..produced]);
                            }
                            stream_end = true;
                        }
                        _ => failed = true,
                    }
                } else {
                    failed = true;
                }
                break;
            }
        }
        if failed || !stream_end {
            package.rollback_decrypted_attempt(&entry.path)?;
            continue;
        }
        let sum = Sha256::digest(&plain[..plain.len().min(CHECKSUM_PREFIX)]);
        if bool::from(sum.as_slice().ct_eq(entry.checksum.as_slice())) {
            package.finish_decrypted(&entry.path)?;
            return Ok(Some(plain));
        }
        package.rollback_decrypted_attempt(&entry.path)?;
    }
    Ok(None)
}

fn parse_u32(value: Option<&str>) -> Result<u32, KordocError> {
    value
        .and_then(|value| value.parse().ok())
        .ok_or_else(|| unsupported("HWPX encryption parameter is invalid"))
}

fn decode_b64(value: &str) -> Result<Vec<u8>, KordocError> {
    let compact: Vec<u8> = value
        .bytes()
        .filter(|byte| !byte.is_ascii_whitespace())
        .collect();
    if compact.is_empty() || !compact.len().is_multiple_of(4) {
        return Err(unsupported("HWPX encryption parameter is invalid"));
    }
    let padding = compact
        .iter()
        .rev()
        .take_while(|byte| **byte == b'=')
        .count();
    if padding > 2 || compact[..compact.len() - padding].contains(&b'=') {
        return Err(unsupported("HWPX encryption parameter is invalid"));
    }
    let mut out = Vec::with_capacity(value.len() * 3 / 4);
    let mut bits = 0u32;
    let mut count = 0u8;
    for byte in compact.iter().copied().take(compact.len() - padding) {
        let digit = match byte {
            b'A'..=b'Z' => byte - b'A',
            b'a'..=b'z' => byte - b'a' + 26,
            b'0'..=b'9' => byte - b'0' + 52,
            b'+' => 62,
            b'/' => 63,
            _ => return Err(unsupported("HWPX encryption parameter is invalid")),
        };
        bits = (bits << 6) | u32::from(digit);
        count += 6;
        if count >= 8 {
            count -= 8;
            out.push((bits >> count) as u8);
        }
    }
    if count > 0 && (bits & ((1u32 << count) - 1)) != 0 {
        return Err(unsupported("HWPX encryption parameter is invalid"));
    }
    let expected = compact.len() / 4 * 3 - padding;
    if out.len() != expected {
        return Err(unsupported("HWPX encryption parameter is invalid"));
    }
    Ok(out)
}

fn safe_member_path(path: &str) -> Result<String, KordocError> {
    if path.is_empty()
        || path.starts_with('/')
        || path.contains(['\\', ':', '\0', '?', '#'])
        || path
            .split('/')
            .any(|part| part.is_empty() || part == "." || part == "..")
    {
        return Err(corrupted("Encrypted package path is invalid"));
    }
    Ok(path.to_owned())
}

fn corrupted(message: &'static str) -> KordocError {
    KordocError::new(ErrorCode::Corrupted, message)
}
fn unsupported(message: &'static str) -> KordocError {
    KordocError::new(ErrorCode::Corrupted, message)
}

#[cfg(test)]
#[path = "crypto/tests.rs"]
pub(crate) mod tests;
