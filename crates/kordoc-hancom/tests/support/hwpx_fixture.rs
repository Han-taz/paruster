//! CC0-1.0 synthetic HWPX recipe source. No input bytes come from the migration oracle.
//!
//! ZIP records are emitted in an explicit order, stored without compression, and stamped at
//! 1980-01-01 00:00:00. XML is assembled as UTF-8 text. The encrypted recipes use the fixed
//! password `fixture-password`, salt, IV, and iteration count below. Their ciphertext is generated
//! from this module's own plaintext, never copied from a document.

use std::io::{Cursor, Write};

use aes::Aes256;
use aes::cipher::{BlockModeEncrypt, KeyIvInit, block_padding::NoPadding};
use flate2::Compression;
use flate2::write::DeflateEncoder;
use pbkdf2::pbkdf2_hmac;
use sha1::Sha1;
use sha2::{Digest, Sha256};
use zip::write::SimpleFileOptions;
use zip::{CompressionMethod, DateTime, ZipWriter};

const XML_DECL: &str = "<?xml version=\"1.0\" encoding=\"UTF-8\"?>";
const SECTION_NS: &str = "xmlns:hs=\"http://www.hancom.co.kr/hwpml/2011/section\" xmlns:hp=\"http://www.hancom.co.kr/hwpml/2011/paragraph\"";
const PASSWORD: &str = "fixture-password";
const SALT: [u8; 16] = [0x11; 16];
const IV: [u8; 16] = [0x22; 16];
const ITERATIONS: u32 = 1024;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Recipe {
    Minimal,
    TwoSectionSpineReversed,
    NestedTable,
    PageCache,
    MissingPageCache,
    EncryptedSha1,
    EncryptedSha256,
    MalformedSection,
    OverDepthSection,
    OverDepthManifest,
    DirectoryEntry500,
    DirectoryEntry501,
}

impl Recipe {
    pub const ALL: [Self; 12] = [
        Self::Minimal,
        Self::TwoSectionSpineReversed,
        Self::NestedTable,
        Self::PageCache,
        Self::MissingPageCache,
        Self::EncryptedSha1,
        Self::EncryptedSha256,
        Self::MalformedSection,
        Self::OverDepthSection,
        Self::OverDepthManifest,
        Self::DirectoryEntry500,
        Self::DirectoryEntry501,
    ];

    pub const fn id(self) -> &'static str {
        match self {
            Self::Minimal => "minimal",
            Self::TwoSectionSpineReversed => "two_section_spine_reversed",
            Self::NestedTable => "nested_table",
            Self::PageCache => "page_cache",
            Self::MissingPageCache => "missing_page_cache",
            Self::EncryptedSha1 => "encrypted_sha1",
            Self::EncryptedSha256 => "encrypted_sha256",
            Self::MalformedSection => "malformed_section",
            Self::OverDepthSection => "over_depth_section",
            Self::OverDepthManifest => "over_depth_manifest",
            Self::DirectoryEntry500 => "directory_entry_500",
            Self::DirectoryEntry501 => "directory_entry_501",
        }
    }

    pub fn build(self) -> Vec<u8> {
        if matches!(self, Self::DirectoryEntry500 | Self::DirectoryEntry501) {
            return directory_boundary(self == Self::DirectoryEntry501);
        }

        let mut section_names = vec!["section0.xml"];
        let mut sections = vec![section(&paragraph("First section", None))];
        let mut spine = vec![0];
        let mut encrypted = None;
        let mut manifest_override = None;

        match self {
            Self::Minimal => {}
            Self::TwoSectionSpineReversed => {
                section_names.push("section1.xml");
                sections.push(section(&paragraph("Second section", None)));
                spine = vec![1, 0];
            }
            Self::NestedTable => sections[0] = section(&nested_table()),
            Self::PageCache => {
                sections[0] = section(&format!(
                    "{}{}{}",
                    paragraph("First page", Some(0)),
                    paragraph("First page continuation", Some(3000)),
                    paragraph("Second page", Some(0))
                ));
            }
            Self::MissingPageCache => {
                sections[0] = section(&format!(
                    "{}{}",
                    paragraph("First page", None),
                    paragraph("Second page", None)
                ));
            }
            Self::EncryptedSha1 => encrypted = Some(Prf::Sha1),
            Self::EncryptedSha256 => encrypted = Some(Prf::Sha256),
            Self::MalformedSection => {
                section_names.extend(["section1.xml", "section2.xml"]);
                sections.push(format!("{XML_DECL}<hs:sec {SECTION_NS}><hp:p>"));
                sections.push(section(&paragraph("Third section", None)));
                spine = vec![0, 1, 2];
            }
            Self::OverDepthSection => {
                let wrappers = "<hp:wrapper>".repeat(201);
                let closing = "</hp:wrapper>".repeat(201);
                sections[0] = section(&format!("{wrappers}{}{closing}", paragraph("Deep", None)));
            }
            Self::OverDepthManifest => {
                let wrappers = "<odf:wrapper>".repeat(201);
                let closing = "</odf:wrapper>".repeat(201);
                manifest_override = Some(format!(
                    "{XML_DECL}<odf:manifest xmlns:odf=\"urn:oasis:names:tc:opendocument:xmlns:manifest:1.0\">{wrappers}<odf:file-entry odf:full-path=\"/\" odf:media-type=\"application/hwp+zip\"/>{closing}</odf:manifest>"
                ));
            }
            Self::DirectoryEntry500 | Self::DirectoryEntry501 => unreachable!(),
        }

        let section_payload = sections[0].as_bytes();
        let encryption = encrypted.map(|prf| encrypted_member(section_payload, prf));
        let manifest = manifest_override.unwrap_or_else(|| manifest_xml(encryption.as_ref()));
        let opf = content_hpf(&section_names, &spine);
        let container = format!(
            "{XML_DECL}<ocf:container xmlns:ocf=\"urn:oasis:names:tc:opendocument:xmlns:container\"><ocf:rootfiles><ocf:rootfile full-path=\"Contents/content.hpf\" media-type=\"application/hwp+zip\"/></ocf:rootfiles></ocf:container>"
        );
        let header = format!(
            "{XML_DECL}<hh:head xmlns:hh=\"http://www.hancom.co.kr/hwpml/2011/head\" secCnt=\"{}\"/>",
            section_names.len()
        );

        let mut writer = ZipWriter::new(Cursor::new(Vec::new()));
        write_file(&mut writer, "mimetype", b"application/hwp+zip");
        write_file(&mut writer, "META-INF/manifest.xml", manifest.as_bytes());
        write_file(&mut writer, "META-INF/container.xml", container.as_bytes());
        write_file(&mut writer, "Contents/content.hpf", opf.as_bytes());
        write_file(&mut writer, "Contents/header.xml", header.as_bytes());
        for (index, name) in section_names.iter().enumerate() {
            let path = format!("Contents/{name}");
            let data = if index == 0 {
                encryption.as_ref().map(|entry| entry.ciphertext.as_slice())
            } else {
                None
            }
            .unwrap_or_else(|| sections[index].as_bytes());
            write_file(&mut writer, &path, data);
        }
        writer.finish().unwrap().into_inner()
    }
}

fn section(body: &str) -> String {
    format!("{XML_DECL}<hs:sec {SECTION_NS}>{body}</hs:sec>")
}

fn paragraph(text: &str, y: Option<u32>) -> String {
    let cache = y.map_or_else(String::new, |position| format!(
        "<hp:linesegarray><hp:lineseg textpos=\"0\" vertpos=\"{position}\" horzpos=\"0\" vertsize=\"1000\" horzsize=\"42520\"/></hp:linesegarray>"
    ));
    format!(
        "<hp:p paraPrIDRef=\"0\">{cache}<hp:run charPrIDRef=\"0\"><hp:t>{text}</hp:t></hp:run></hp:p>"
    )
}

fn nested_table() -> String {
    fn table(inner: &str) -> String {
        format!(
            "<hp:tbl rowCnt=\"1\" colCnt=\"1\"><hp:caption side=\"TOP\"><hp:subList>{}</hp:subList></hp:caption><hp:tr><hp:tc><hp:cellAddr colAddr=\"0\" rowAddr=\"0\"/><hp:cellSpan colSpan=\"1\" rowSpan=\"1\"/><hp:subList>{inner}</hp:subList></hp:tc></hp:tr></hp:tbl>",
            paragraph("Synthetic caption", None)
        )
    }
    let inner = table(&paragraph("Inner cell", None));
    let outer = table(&format!("{}{}", paragraph("Outer cell", None), inner));
    format!("<hp:p><hp:run>{outer}</hp:run></hp:p>")
}

fn content_hpf(section_names: &[&str], spine: &[usize]) -> String {
    let mut items = String::new();
    for (index, name) in section_names.iter().enumerate() {
        items.push_str(&format!(
            "<opf:item id=\"section{index}\" href=\"{name}\" media-type=\"application/xml\"/>"
        ));
    }
    let mut refs = String::new();
    for index in spine {
        refs.push_str(&format!("<opf:itemref idref=\"section{index}\"/>"));
    }
    format!(
        "{XML_DECL}<opf:package xmlns:opf=\"http://www.idpf.org/2007/opf\" xmlns:dc=\"http://purl.org/dc/elements/1.1/\"><opf:metadata><dc:title>Synthetic HWPX</dc:title><dc:creator>paruster</dc:creator></opf:metadata><opf:manifest>{items}</opf:manifest><opf:spine>{refs}</opf:spine></opf:package>"
    )
}

fn manifest_xml(encryption: Option<&EncryptedMember>) -> String {
    let encryption_data = encryption.map_or_else(String::new, |entry| format!(
        "<odf:encryption-data odf:checksum-type=\"sha256-1k\" odf:checksum=\"{}\"><odf:algorithm odf:algorithm-name=\"http://www.w3.org/2001/04/xmlenc#aes256-cbc\" odf:initialisation-vector=\"{}\"/><odf:key-derivation odf:key-derivation-name=\"PBKDF2\" odf:salt=\"{}\" odf:iteration-count=\"{ITERATIONS}\" odf:key-size=\"32\"/><odf:start-key-generation odf:start-key-generation-name=\"http://www.w3.org/2000/09/xmldsig#sha256\"/></odf:encryption-data>",
        base64(&entry.checksum), base64(&IV), base64(&SALT)
    ));
    format!(
        "{XML_DECL}<odf:manifest xmlns:odf=\"urn:oasis:names:tc:opendocument:xmlns:manifest:1.0\"><odf:file-entry odf:full-path=\"/\" odf:media-type=\"application/hwp+zip\"></odf:file-entry><odf:file-entry odf:full-path=\"Contents/section0.xml\" odf:media-type=\"text/xml\">{encryption_data}</odf:file-entry></odf:manifest>"
    )
}

#[derive(Clone, Copy)]
enum Prf {
    Sha1,
    Sha256,
}

struct EncryptedMember {
    checksum: [u8; 32],
    ciphertext: Vec<u8>,
}

fn encrypted_member(plaintext: &[u8], prf: Prf) -> EncryptedMember {
    let start_key = Sha256::digest(PASSWORD.as_bytes());
    let mut key = [0_u8; 32];
    match prf {
        Prf::Sha1 => pbkdf2_hmac::<Sha1>(&start_key, &SALT, ITERATIONS, &mut key),
        Prf::Sha256 => pbkdf2_hmac::<Sha256>(&start_key, &SALT, ITERATIONS, &mut key),
    }
    let mut deflater = DeflateEncoder::new(Vec::new(), Compression::default());
    deflater.write_all(plaintext).unwrap();
    let mut raw = deflater.finish().unwrap();
    let fill = (16 - raw.len() % 16) % 16;
    raw.resize(raw.len() + fill, 0);
    let length = raw.len();
    cbc::Encryptor::<Aes256>::new(&key.into(), &IV.into())
        .encrypt_padded::<NoPadding>(&mut raw, length)
        .unwrap();
    EncryptedMember {
        checksum: Sha256::digest(&plaintext[..plaintext.len().min(1024)]).into(),
        ciphertext: raw,
    }
}

fn base64(bytes: &[u8]) -> String {
    const ALPHABET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut result = String::with_capacity(bytes.len().div_ceil(3) * 4);
    for chunk in bytes.chunks(3) {
        let a = chunk[0];
        let b = *chunk.get(1).unwrap_or(&0);
        let c = *chunk.get(2).unwrap_or(&0);
        result.push(ALPHABET[(a >> 2) as usize] as char);
        result.push(ALPHABET[(((a & 3) << 4) | (b >> 4)) as usize] as char);
        result.push(if chunk.len() > 1 {
            ALPHABET[(((b & 15) << 2) | (c >> 6)) as usize] as char
        } else {
            '='
        });
        result.push(if chunk.len() > 2 {
            ALPHABET[(c & 63) as usize] as char
        } else {
            '='
        });
    }
    result
}

fn directory_boundary(excess: bool) -> Vec<u8> {
    let mut writer = ZipWriter::new(Cursor::new(Vec::new()));
    write_file(&mut writer, "mimetype", b"application/hwp+zip");
    for index in 0..if excess { 500 } else { 499 } {
        writer
            .add_directory(format!("padding/{index:04}/"), file_options())
            .unwrap();
    }
    writer.finish().unwrap().into_inner()
}

fn file_options() -> SimpleFileOptions {
    SimpleFileOptions::default()
        .compression_method(CompressionMethod::Stored)
        .last_modified_time(DateTime::from_date_and_time(1980, 1, 1, 0, 0, 0).unwrap())
        .unix_permissions(0o644)
}

fn write_file(writer: &mut ZipWriter<Cursor<Vec<u8>>>, path: &str, data: &[u8]) {
    writer.start_file(path, file_options()).unwrap();
    writer.write_all(data).unwrap();
}
