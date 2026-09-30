use super::{extract_metadata, metadata_value, read_dublin_core, read_opf};
use crate::hwpx::budget::LoweringBudget;
use crate::hwpx::package::Package;
use crate::hwpx::xml::parse;
use kordoc_ir::{DocumentMetadata, ErrorCode};
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
fn metadata_reads_hpf_without_parsing_section_payloads() {
    let mut writer = ZipWriter::new(Cursor::new(Vec::new()));
    let options = SimpleFileOptions::default();
    writer.start_file("Contents/content.hpf", options).unwrap();
    writer.write_all(br#"<opf:package><opf:metadata><dc:title>Report</dc:title><opf:meta name="creator">Ada</opf:meta></opf:metadata><opf:manifest><opf:item id="s" href="section0.xml"/></opf:manifest><opf:spine><opf:itemref idref="s"/></opf:spine></opf:package>"#).unwrap();
    writer.start_file("Contents/section0.xml", options).unwrap();
    writer.write_all(b"not xml").unwrap();
    let mut bytes = writer.finish().unwrap().into_inner();
    let name_at = bytes
        .windows(b"Contents/section0.xml".len())
        .position(|window| window == b"Contents/section0.xml")
        .unwrap();
    let local_header = name_at - 30;
    let name_len =
        u16::from_le_bytes([bytes[local_header + 26], bytes[local_header + 27]]) as usize;
    let extra_len =
        u16::from_le_bytes([bytes[local_header + 28], bytes[local_header + 29]]) as usize;
    let payload_at = local_header + 30 + name_len + extra_len;
    bytes[payload_at] ^= 0xff;
    let mut package = Package::open(&bytes).unwrap();
    let metadata = extract_metadata(&mut package).unwrap();
    assert_eq!(metadata.title.as_deref(), Some("Report"));
    assert_eq!(metadata.author.as_deref(), Some("Ada"));
    assert_eq!(metadata.page_count, Some(1));
}

#[test]
fn malformed_optional_metadata_is_skipped_for_later_fallback() {
    let bytes = archive(&[
        ("meta.xml", b"<broken>"),
        (
            "META-INF/meta.xml",
            b"<meta><title>Fallback</title><creator>Writer</creator></meta>",
        ),
    ]);
    let mut package = Package::open(&bytes).unwrap();
    let metadata = extract_metadata(&mut package).unwrap();
    assert_eq!(metadata.title.as_deref(), Some("Fallback"));
    assert_eq!(metadata.author.as_deref(), Some("Writer"));
}

#[test]
fn first_optional_title_stops_later_metadata_candidates() {
    let bytes = archive(&[
        (
            "META-INF/meta.xml",
            b"<meta><title>First title</title></meta>",
        ),
        (
            "docProps/core.xml",
            b"<core><creator>Later author</creator></core>",
        ),
    ]);
    let mut package = Package::open(&bytes).unwrap();
    let metadata = extract_metadata(&mut package).unwrap();
    assert_eq!(metadata.title.as_deref(), Some("First title"));
    assert_eq!(metadata.author, None);
}

#[test]
fn opf_title_stops_dublin_core_fallback_and_description_prefers_name_order() {
    let bytes = archive(&[
        (
            "Contents/content.hpf",
            br#"<package><metadata><title>OPF title</title><meta name="subject">Subject</meta><meta name="creator">First</meta><meta name="creator">Second</meta><meta name="description">Description</meta><meta name="CreatedDate">2025-07-07 09:40:07</meta><meta name="ModifiedDate">not-a-date 09:40:07</meta></metadata></package>"#,
        ),
        ("meta.xml", b"<meta><creator>Fallback author</creator></meta>"),
        ("Contents/section0.xml", b"unread section bytes"),
    ]);
    let mut package = Package::open(&bytes).unwrap();
    let metadata = extract_metadata(&mut package).unwrap();
    assert_eq!(metadata.title.as_deref(), Some("OPF title"));
    assert_eq!(metadata.author.as_deref(), Some("First"));
    assert_eq!(metadata.description.as_deref(), Some("Description"));
    assert_eq!(metadata.created_at.as_deref(), Some("2025-07-07T09:40:07"));
    assert_eq!(metadata.modified_at.as_deref(), Some("not-a-date 09:40:07"));
}

#[test]
fn existing_opf_title_prevents_optional_author_fallback() {
    let bytes = archive(&[
        (
            "Contents/content.hpf",
            b"<package><metadata><title>OPF title</title></metadata></package>",
        ),
        (
            "meta.xml",
            b"<meta><creator>Fallback author</creator></meta>",
        ),
    ]);
    let mut package = Package::open(&bytes).unwrap();
    let metadata = extract_metadata(&mut package).unwrap();
    assert_eq!(metadata.title.as_deref(), Some("OPF title"));
    assert_eq!(metadata.author, None);
}

#[test]
fn nested_opf_text_obeys_reduced_aggregate_metadata_budget() {
    let nested = format!(
        "<package><metadata><title>{}{}{}</title></metadata></package>",
        "<n>".repeat(32),
        "x".repeat(2048),
        "</n>".repeat(32)
    );
    let root = parse(nested.as_bytes()).unwrap();
    let mut metadata = DocumentMetadata::default();
    let mut budget = LoweringBudget::with_limit(1024);
    assert_eq!(
        read_opf(&root, &mut metadata, &mut budget)
            .unwrap_err()
            .code,
        ErrorCode::OutputTooLarge
    );
    assert_eq!(metadata.title, None);
}

#[test]
fn optional_metadata_budget_failure_is_not_skipped() {
    let root = parse(b"<meta><title>0123456789</title></meta>").unwrap();
    let mut metadata = DocumentMetadata::default();
    let mut budget = LoweringBudget::with_limit(9);
    assert_eq!(
        read_dublin_core(&root, &mut metadata, &mut budget)
            .unwrap_err()
            .code,
        ErrorCode::OutputTooLarge
    );
}

#[test]
fn metadata_value_budget_is_inclusive_at_both_transient_and_retained_copies() {
    let root = parse(b"<title>abc</title>").unwrap();
    assert_eq!(
        metadata_value(&root, &mut LoweringBudget::with_limit(6)).unwrap(),
        Some("abc".to_owned())
    );
    assert_eq!(
        metadata_value(&root, &mut LoweringBudget::with_limit(5))
            .unwrap_err()
            .code,
        ErrorCode::OutputTooLarge
    );
}

#[test]
fn opf_and_optional_fallback_share_one_metadata_budget() {
    let opf = parse(
        format!(
            "<package><metadata><meta name=\"subject\">{}</meta></metadata></package>",
            "x".repeat(700)
        )
        .as_bytes(),
    )
    .unwrap();
    let optional =
        parse(format!("<meta><title>{}</title></meta>", "y".repeat(700)).as_bytes()).unwrap();
    let mut metadata = DocumentMetadata::default();
    let mut budget = LoweringBudget::with_limit(2_500);

    read_opf(&opf, &mut metadata, &mut budget).unwrap();
    assert_eq!(metadata.description.as_deref().map(str::len), Some(700));
    assert_eq!(
        read_dublin_core(&optional, &mut metadata, &mut budget)
            .unwrap_err()
            .code,
        ErrorCode::OutputTooLarge
    );
}
