#[path = "../src/limits.rs"]
#[allow(
    dead_code,
    reason = "integration test path-includes staged private Task 0 module"
)]
mod limits;
#[path = "../src/objects.rs"]
#[allow(
    dead_code,
    reason = "integration test path-includes staged private Task 0 module"
)]
mod objects;
#[path = "../src/stream.rs"]
#[allow(
    dead_code,
    reason = "integration test path-includes staged private Task 0 module"
)]
mod stream;

use objects::{PdfObjectReader, PdfReadError, PdfValue};

const CLASSIC: &[u8] = include_bytes!("../../../tests/golden/document/pdf/classic_xref.pdf");
const XREF_STREAM: &[u8] = include_bytes!("../../../tests/golden/document/pdf/xref_stream.pdf");
const OBJECT_STREAM: &[u8] = include_bytes!("../../../tests/golden/document/pdf/object_stream.pdf");
const INCREMENTAL: &[u8] =
    include_bytes!("../../../tests/golden/document/pdf/incremental_revision.pdf");
const GENERATION: &[u8] =
    include_bytes!("../../../tests/golden/document/pdf/generation_mismatch.pdf");
const CYCLE: &[u8] = include_bytes!("../../../tests/golden/document/pdf/reference_cycle.pdf");
const MIXED: &[u8] = include_bytes!("../../../tests/golden/document/pdf/mixed_filters.pdf");
const ASCII_HEX: &[u8] = include_bytes!("../../../tests/golden/document/pdf/ascii_hex.pdf");
const BAD_LENGTH: &[u8] = include_bytes!("../../../tests/golden/document/pdf/malformed_length.pdf");
const ENCRYPTED: &[u8] = include_bytes!("../../../tests/golden/document/pdf/encrypted_trailer.pdf");

#[test]
fn reads_indirect_object_from_classic_xref() {
    let mut reader = PdfObjectReader::new(CLASSIC).unwrap();
    let result = reader.resolve((1, 0));
    assert!(matches!(result, Ok(PdfValue::Dictionary(_))), "{result:?}");
}

#[test]
fn reads_objects_referenced_by_xref_stream() {
    let mut reader = PdfObjectReader::new(XREF_STREAM).unwrap();
    let result = reader.resolve((1, 0));
    assert!(matches!(result, Ok(PdfValue::Dictionary(_))), "{result:?}");
}

#[test]
fn rejects_xref_stream_field_widths_that_do_not_fit_machine_integer() {
    let malformed = replace_once(XREF_STREAM, b"/W [1 4 2]", b"/W [1 9 2]");
    assert!(matches!(
        PdfObjectReader::new(&malformed),
        Err(PdfReadError::Corrupted)
    ));
}

#[test]
fn expands_unfiltered_object_stream_with_bounded_offsets() {
    let mut reader = PdfObjectReader::new(OBJECT_STREAM).unwrap();
    assert!(matches!(
        reader.resolve((5, 0)),
        Ok(PdfValue::OwnedDictionary(_))
    ));
    assert!(matches!(reader.resolve((6, 0)), Ok(PdfValue::Owned(_))));
}

#[test]
fn uses_latest_incremental_revision() {
    let mut reader = PdfObjectReader::new(INCREMENTAL).unwrap();
    let page = reader.resolve((3, 0)).unwrap();
    assert!(page.as_bytes().windows(3).any(|window| window == b"300"));
}

#[test]
fn rejects_generation_mismatch_and_reference_cycles() {
    let mut mismatch = PdfObjectReader::new(GENERATION).unwrap();
    assert!(matches!(
        mismatch.resolve((3, 0)),
        Err(PdfReadError::GenerationMismatch)
    ));
    let mut cycle = PdfObjectReader::new(CYCLE).unwrap();
    assert!(matches!(
        cycle.resolve_graph((1, 0)),
        Err(PdfReadError::ReferenceCycle)
    ));
}

#[test]
fn decodes_supported_mixed_filters_or_rejects_before_expansion() {
    let mut reader = PdfObjectReader::new(MIXED).unwrap();
    assert!(matches!(
        reader.read_stream((1, 0)),
        Err(PdfReadError::UnsupportedFilter)
    ));
}

#[test]
fn bounds_ascii_hex_decode_while_producing_bytes() {
    let mut reader = PdfObjectReader::new(ASCII_HEX).unwrap();
    assert_eq!(reader.read_stream((1, 0)).unwrap(), b"AB");
}

#[test]
fn rejects_malformed_stream_length_and_reports_encryption_without_secret_data() {
    let mut malformed = PdfObjectReader::new(BAD_LENGTH).unwrap();
    assert!(matches!(
        malformed.read_stream((1, 0)),
        Err(PdfReadError::Corrupted)
    ));
    assert!(matches!(
        PdfObjectReader::new(ENCRYPTED),
        Err(PdfReadError::Encrypted)
    ));
}

#[test]
fn does_not_treat_endobj_inside_a_literal_string_as_the_object_end() {
    let pdf = single_object_pdf(b"<< /Note (endobj is data) /After true >>");
    let mut reader = PdfObjectReader::new(&pdf).unwrap();
    let value = reader.resolve((1, 0)).unwrap();
    assert!(
        value
            .as_bytes()
            .windows(6)
            .any(|window| window == b"/After")
    );
}

#[test]
fn ignores_reference_shaped_text_inside_a_literal_string() {
    let pdf = single_object_pdf(b"<< /Text (999 0 R) >>");
    let mut reader = PdfObjectReader::new(&pdf).unwrap();
    assert!(reader.resolve_graph((1, 0)).is_ok());
}

#[test]
fn does_not_mistake_encrypt_text_inside_an_object_for_trailer_encryption() {
    let pdf = single_object_pdf(b"<< /Note (/Encrypt) >>");
    assert!(PdfObjectReader::new(&pdf).is_ok());
}

#[test]
fn detects_encrypt_key_after_dictionary_close_text_in_a_literal_string() {
    let pdf = single_object_pdf_with_trailer(
        b"<< /Type /Catalog >>",
        b"/Note (>>) /Encrypt 2 0 R /Root 1 0 R",
    );
    assert!(matches!(
        PdfObjectReader::new(&pdf),
        Err(PdfReadError::Encrypted)
    ));
}

#[test]
fn detects_encrypt_key_after_comment_between_trailer_and_dictionary() {
    let mut pdf =
        single_object_pdf_with_trailer(b"<< /Type /Catalog >>", b"/Encrypt 2 0 R /Root 1 0 R");
    let marker = b"trailer\n<<";
    let at = pdf
        .windows(marker.len())
        .position(|window| window == marker)
        .unwrap();
    pdf.splice(
        at..at + marker.len(),
        b"trailer\n% << >>\n<<".iter().copied(),
    );

    assert!(matches!(
        PdfObjectReader::new(&pdf),
        Err(PdfReadError::Encrypted)
    ));
}

#[test]
fn does_not_scan_stream_payload_for_object_references() {
    let pdf = single_object_pdf(b"<< /Length 5 >>\nstream\n1 0 R\nendstream");
    let mut reader = PdfObjectReader::new(&pdf).unwrap();
    assert!(reader.resolve_graph((1, 0)).is_ok());
}

fn single_object_pdf(body: &[u8]) -> Vec<u8> {
    single_object_pdf_with_trailer(body, b"/Root 1 0 R")
}

fn single_object_pdf_with_trailer(body: &[u8], trailer: &[u8]) -> Vec<u8> {
    let mut pdf = b"%PDF-1.7\n".to_vec();
    let offset = pdf.len();
    pdf.extend_from_slice(b"1 0 obj\n");
    pdf.extend_from_slice(body);
    pdf.extend_from_slice(b"\nendobj\n");
    let xref = pdf.len();
    pdf.extend_from_slice(b"xref\n0 2\n0000000000 65535 f \n");
    pdf.extend_from_slice(format!("{offset:010} 00000 n \n").as_bytes());
    pdf.extend_from_slice(b"trailer\n<< /Size 2 ");
    pdf.extend_from_slice(trailer);
    pdf.extend_from_slice(format!(" >>\nstartxref\n{xref}\n%%EOF\n").as_bytes());
    pdf
}

fn replace_once(source: &[u8], needle: &[u8], replacement: &[u8]) -> Vec<u8> {
    assert_eq!(needle.len(), replacement.len());
    let at = source
        .windows(needle.len())
        .position(|window| window == needle)
        .unwrap();
    let mut result = source.to_vec();
    result[at..at + needle.len()].copy_from_slice(replacement);
    result
}
