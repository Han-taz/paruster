#![cfg(feature = "pdfjs-v8")]

#[allow(
    dead_code,
    reason = "this integration test reuses the private V8 runtime module"
)]
#[path = "../src/v8_runtime/mod.rs"]
mod v8_runtime;
#[allow(
    dead_code,
    reason = "this integration test exercises its private framed result path"
)]
#[path = "../src/worker_protocol.rs"]
mod worker_protocol;

use v8_runtime::{PdfJsProbe, probe_pdf_text};
use worker_protocol::{read_response, write_response};

const UNICODE_PDF: &[u8] = include_bytes!("fixtures/unicode_probe/unicode_probe.pdf");

#[test]
fn pdfjs_extracts_tounicode_hangul_and_astral_character_as_utf8() {
    let result: PdfJsProbe = probe_pdf_text(UNICODE_PDF).unwrap();
    assert_eq!(result.page_count, 1);
    assert_eq!(result.page_text, ["한글🧪"]);
    assert_eq!(result.page_text[0].as_bytes(), "한글🧪".as_bytes());
}

#[test]
fn unicode_probe_roundtrips_over_the_worker_json_protocol() {
    let result = probe_pdf_text(UNICODE_PDF).unwrap();
    let mut framed = Vec::new();
    write_response(&mut framed, Ok(&result)).unwrap();
    let roundtripped = read_response(&mut std::io::Cursor::new(framed))
        .unwrap()
        .unwrap();
    assert_eq!(roundtripped, result);
    assert_eq!(roundtripped.page_text, ["한글🧪"]);
}

#[test]
fn checked_in_unicode_pdf_is_reproducible_from_its_rust_recipe() {
    assert_eq!(UNICODE_PDF, build_unicode_pdf());
}

fn build_unicode_pdf() -> Vec<u8> {
    fn stream(data: &[u8]) -> Vec<u8> {
        let mut output = format!("<< /Length {} >>\nstream\n", data.len()).into_bytes();
        output.extend_from_slice(data);
        output.extend_from_slice(b"\nendstream");
        output
    }

    let cmap = b"/CIDInit /ProcSet findresource begin\n12 dict begin\nbegincmap\n/CIDSystemInfo << /Registry (Adobe) /Ordering (UCS) /Supplement 0 >> def\n/CMapName /FixtureUnicode-UCS def\n/CMapType 2 def\n1 begincodespacerange\n<0000> <FFFF>\nendcodespacerange\n3 beginbfchar\n<0001> <D55C>\n<0002> <AE00>\n<0003> <D83EDDEA>\nendbfchar\nendcmap\nCMapName currentdict /CMap defineresource pop\nend\nend";
    let content = b"BT\n/F1 18 Tf\n72 720 Td\n<000100020003> Tj\nET";
    let objects = [
        b"<< /Type /Catalog /Pages 2 0 R >>".to_vec(),
        b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_vec(),
        b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] /Resources << /Font << /F1 4 0 R >> >> /Contents 8 0 R >>".to_vec(),
        b"<< /Type /Font /Subtype /Type0 /BaseFont /FixtureUnicode /Encoding /Identity-H /DescendantFonts [5 0 R] /ToUnicode 6 0 R >>".to_vec(),
        b"<< /Type /Font /Subtype /CIDFontType2 /BaseFont /FixtureUnicode /CIDSystemInfo << /Registry (Adobe) /Ordering (Identity) /Supplement 0 >> /FontDescriptor 7 0 R /DW 1000 /CIDToGIDMap /Identity >>".to_vec(),
        stream(cmap),
        b"<< /Type /FontDescriptor /FontName /FixtureUnicode /Flags 4 /FontBBox [0 -200 1000 900] /ItalicAngle 0 /Ascent 800 /Descent -200 /CapHeight 700 /StemV 80 >>".to_vec(),
        stream(content),
    ];

    let mut pdf = b"%PDF-1.7\n%\xe2\xe3\xcf\xd3\n".to_vec();
    let mut offsets = Vec::with_capacity(objects.len());
    for (index, object) in objects.iter().enumerate() {
        offsets.push(pdf.len());
        pdf.extend_from_slice(format!("{} 0 obj\n", index + 1).as_bytes());
        pdf.extend_from_slice(object);
        pdf.extend_from_slice(b"\nendobj\n");
    }
    let xref = pdf.len();
    pdf.extend_from_slice(format!("xref\n0 {}\n", offsets.len() + 1).as_bytes());
    pdf.extend_from_slice(b"0000000000 65535 f \n");
    for offset in offsets {
        pdf.extend_from_slice(format!("{offset:010} 00000 n \n").as_bytes());
    }
    pdf.extend_from_slice(
        format!(
            "trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF\n",
            objects.len() + 1
        )
        .as_bytes(),
    );
    pdf
}
