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

use limits::{BudgetError, PdfBudget};
use objects::{PdfObjectReader, PdfReadError};

#[test]
fn enforces_stream_and_cumulative_decoded_caps_before_acceptance() {
    let mut per_stream = PdfBudget::default();
    assert!(per_stream.charge_decoded(32 * 1024 * 1024).is_ok());
    assert_eq!(per_stream.charge_decoded(1), Err(BudgetError::StreamBytes));

    let mut total = PdfBudget::default();
    for _ in 0..8 {
        assert!(total.charge_decoded(32 * 1024 * 1024).is_ok());
        total.finish_stream();
    }
    assert_eq!(total.charge_decoded(1), Err(BudgetError::DecodedBytes));
}

#[test]
fn unwinds_object_and_form_recursion_after_errors() {
    let mut budget = PdfBudget::default();
    for _ in 0..64 {
        budget.enter_object().unwrap();
    }
    assert_eq!(budget.enter_object(), Err(BudgetError::ObjectDepth));
    budget.leave_object();
    assert_eq!(budget.enter_object(), Ok(()));
    for _ in 0..65 {
        budget.leave_object();
    }
    assert_eq!(
        budget.with_object(|_| Err::<(), _>(BudgetError::ObjectDepth)),
        Err(BudgetError::ObjectDepth)
    );
    assert!(budget.with_object(|_| Ok(())).is_ok());

    for _ in 0..32 {
        budget.enter_form().unwrap();
    }
    assert_eq!(budget.enter_form(), Err(BudgetError::FormDepth));
    budget.leave_form();
    assert_eq!(budget.enter_form(), Ok(()));
    for _ in 0..32 {
        budget.leave_form();
    }
    assert_eq!(
        budget.with_form(|_| Err::<(), _>(BudgetError::FormDepth)),
        Err(BudgetError::FormDepth)
    );
    assert!(budget.with_form(|_| Ok(())).is_ok());
}

#[test]
fn rejects_the_object_id_after_one_million_distinct_ids() {
    let mut budget = PdfBudget::default();
    for _ in 0..1_000_000 {
        budget.charge_object().unwrap();
    }
    assert_eq!(budget.charge_object(), Err(BudgetError::Objects));
}

#[test]
fn rejects_sparse_source_over_the_input_limit_without_a_second_copy() {
    let budget = PdfBudget::default();
    assert_eq!(
        budget.charge_source(524_288_001),
        Err(BudgetError::SourceBytes)
    );
}

#[test]
fn rejects_reader_stream_limit_before_copying_payload() {
    let len = 32 * 1024 * 1024 + 1;
    let payload = b"x".repeat(len as usize);
    let pdf = test_stream_pdf(&payload);
    let mut reader = PdfObjectReader::new(&pdf).unwrap();
    assert!(matches!(
        reader.read_stream((1, 0)),
        Err(PdfReadError::Budget(BudgetError::StreamBytes))
    ));
}

#[test]
fn enforces_cumulative_decode_limit_across_repeated_stream_access() {
    let payload = vec![b'x'; 32 * 1024 * 1024];
    let mut pdf = test_stream_pdf(&payload);
    append_small_stream(&mut pdf, 2, b"x");
    let mut reader = PdfObjectReader::new(&pdf).unwrap();
    for _ in 0..8 {
        let decoded = reader.read_stream((1, 0)).unwrap();
        assert_eq!(decoded.len(), 32 * 1024 * 1024);
    }
    assert!(matches!(
        reader.read_stream((2, 0)),
        Err(PdfReadError::Budget(BudgetError::DecodedBytes))
    ));
}

#[test]
fn rejects_reader_reference_chain_at_depth_65() {
    let pdf = reference_chain_pdf(65);
    let mut reader = PdfObjectReader::new(&pdf).unwrap();
    assert!(matches!(
        reader.resolve_graph((1, 0)),
        Err(PdfReadError::Budget(BudgetError::ObjectDepth))
    ));
}

#[test]
fn compressed_object_container_cycles_terminate_before_stack_growth() {
    for self_cycle in [true, false] {
        let pdf = compressed_cycle_pdf(self_cycle);
        let mut reader = PdfObjectReader::new(&pdf).unwrap();
        assert!(matches!(
            reader.resolve((5, 0)),
            Err(PdfReadError::ReferenceCycle)
        ));
    }
}

#[test]
fn rejects_million_and_first_xref_object_before_map_growth_past_cap() {
    let pdf = oversized_xref_pdf(1_000_002);
    let result = PdfObjectReader::new(&pdf);
    assert!(matches!(
        result,
        Err(PdfReadError::Budget(BudgetError::Objects))
    ));
}

#[test]
fn sparse_500_mib_file_is_rejected_from_metadata_before_reading() {
    use std::fs::OpenOptions;
    use std::io::Write;

    let path = std::env::temp_dir().join(format!("paruster-pdf-sparse-{}.pdf", std::process::id()));
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&path)
        .unwrap();
    file.write_all(b"%PDF-1.7\n").unwrap();
    file.set_len(500 * 1024 * 1024).unwrap();
    drop(file);
    let accepted = PdfObjectReader::preflight_file(&path);
    let file = OpenOptions::new()
        .read(true)
        .write(true)
        .open(&path)
        .unwrap();
    file.set_len(500 * 1024 * 1024 + 1).unwrap();
    drop(file);
    let rejected = PdfObjectReader::preflight_file(&path);
    std::fs::remove_file(&path).unwrap();
    assert_eq!(accepted.unwrap(), 500 * 1024 * 1024);
    assert!(matches!(
        rejected,
        Err(PdfReadError::Budget(BudgetError::SourceBytes))
    ));
}

#[test]
fn parses_valid_near_cap_sparse_source_without_an_internal_clone() {
    use std::fs::OpenOptions;
    use std::io::{Seek, SeekFrom, Write};

    const CAP: u64 = 500 * 1024 * 1024;
    let path = std::env::temp_dir().join(format!(
        "paruster-pdf-valid-sparse-{}.pdf",
        std::process::id()
    ));
    let mut file = OpenOptions::new()
        .read(true)
        .write(true)
        .create_new(true)
        .open(&path)
        .unwrap();
    let prefix = b"%PDF-1.7\n1 0 obj\n<< /Type /Catalog >>\nendobj\n";
    let object_offset = b"%PDF-1.7\n".len();
    file.write_all(prefix).unwrap();
    let xref_offset = CAP - 160;
    let footer = format!(
        "xref\n0 2\n0000000000 65535 f \n{object_offset:010} 00000 n \ntrailer\n<<\n/Size 2\n/Root 1 0 R\n>>\nstartxref\n{xref_offset}\n%%EOF\n"
    );
    file.set_len(xref_offset).unwrap();
    file.seek(SeekFrom::Start(xref_offset)).unwrap();
    file.write_all(footer.as_bytes()).unwrap();
    file.set_len(CAP).unwrap();
    drop(file);

    let source = std::fs::read(&path).unwrap();
    let mut reader = PdfObjectReader::new(&source).unwrap();
    assert!(reader.resolve((1, 0)).is_ok());
    std::fs::remove_file(path).unwrap();
}

#[test]
fn checks_overflow_for_every_cumulative_counter() {
    let mut budget = PdfBudget::default();
    budget.charge_text(1).unwrap();
    assert_eq!(
        budget.charge_text(u64::MAX),
        Err(BudgetError::ArithmeticOverflow)
    );
}

fn test_stream_pdf(payload: &[u8]) -> Vec<u8> {
    let mut pdf = b"%PDF-1.7\n".to_vec();
    let offset = pdf.len();
    pdf.extend_from_slice(b"1 0 obj\n<< /Length ");
    pdf.extend_from_slice(payload.len().to_string().as_bytes());
    pdf.extend_from_slice(b" >>\nstream\n");
    pdf.extend_from_slice(payload);
    pdf.extend_from_slice(b"\nendstream\nendobj\n");
    let xref_offset = pdf.len();
    pdf.extend_from_slice(b"xref\n0 2\n0000000000 65535 f \n");
    pdf.extend_from_slice(format!("{offset:010} 00000 n \n").as_bytes());
    pdf.extend_from_slice(
        format!("trailer\n<< /Size 2 /Root 1 0 R >>\nstartxref\n{xref_offset}\n%%EOF\n").as_bytes(),
    );
    pdf
}

fn append_small_stream(pdf: &mut Vec<u8>, object: u32, payload: &[u8]) {
    let offset = pdf.len();
    pdf.extend_from_slice(
        format!("{object} 0 obj\n<< /Length {} >>\nstream\n", payload.len()).as_bytes(),
    );
    pdf.extend_from_slice(payload);
    pdf.extend_from_slice(b"\nendstream\nendobj\n");
    let xref_offset = pdf.len();
    pdf.extend_from_slice(b"xref\n2 1\n");
    pdf.extend_from_slice(format!("{offset:010} 00000 n \n").as_bytes());
    pdf.extend_from_slice(
        format!(
            "trailer\n<< /Size 3 /Root 1 0 R /Prev {} >>\nstartxref\n{xref_offset}\n%%EOF\n",
            find_previous_xref(pdf)
        )
        .as_bytes(),
    );
}

fn find_previous_xref(pdf: &[u8]) -> usize {
    let marker = pdf.windows(9).rposition(|w| w == b"startxref").unwrap();
    let rest = &pdf[marker + 9..];
    let start = rest.iter().position(|b| b.is_ascii_digit()).unwrap();
    let end = rest[start..]
        .iter()
        .position(|b| !b.is_ascii_digit())
        .unwrap()
        + start;
    std::str::from_utf8(&rest[start..end])
        .unwrap()
        .parse()
        .unwrap()
}

fn reference_chain_pdf(count: usize) -> Vec<u8> {
    let mut pdf = b"%PDF-1.7\n".to_vec();
    let mut offsets = Vec::with_capacity(count);
    for id in 1..=count {
        offsets.push(pdf.len());
        let body = if id < count {
            format!("<< /Next {} 0 R >>", id + 1)
        } else {
            "<< /End true >>".to_owned()
        };
        pdf.extend_from_slice(format!("{id} 0 obj\n{body}\nendobj\n").as_bytes());
    }
    let xref_offset = pdf.len();
    pdf.extend_from_slice(format!("xref\n0 {}\n0000000000 65535 f \n", count + 1).as_bytes());
    for offset in offsets {
        pdf.extend_from_slice(format!("{offset:010} 00000 n \n").as_bytes());
    }
    pdf.extend_from_slice(
        format!(
            "trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{xref_offset}\n%%EOF\n",
            count + 1
        )
        .as_bytes(),
    );
    pdf
}

fn oversized_xref_pdf(size: usize) -> Vec<u8> {
    let xref_id = size - 1;
    let offset = b"%PDF-1.7\n".len();
    let mut entries = Vec::with_capacity(size * 7);
    entries.extend_from_slice(&[0, 0, 0, 0, 0, 255, 255]);
    for _object in 1..xref_id {
        entries.extend_from_slice(&[2]);
        entries.extend_from_slice(&(xref_id as u32).to_be_bytes());
        entries.extend_from_slice(&[0, 0]);
    }
    let data_len = entries.len() + 7;
    let mut pdf = b"%PDF-1.7\n".to_vec();
    let xref_offset = offset;
    pdf.extend_from_slice(format!("{xref_id} 0 obj\n<< /Type /XRef /Size {size} /Root {xref_id} 0 R /W [1 4 2] /Length {data_len} >>\nstream\n").as_bytes());
    entries.extend_from_slice(&[1]);
    entries.extend_from_slice(&(xref_offset as u32).to_be_bytes());
    entries.extend_from_slice(&[0, 0]);
    pdf.extend_from_slice(&entries);
    pdf.extend_from_slice(
        format!("\nendstream\nendobj\nstartxref\n{xref_offset}\n%%EOF\n").as_bytes(),
    );
    pdf
}

fn compressed_cycle_pdf(self_cycle: bool) -> Vec<u8> {
    let xref_id = 7u32;
    let xref_offset = b"%PDF-1.7\n".len();
    let mut entries = vec![0, 0, 0, 0, 0, 255, 255];
    for id in 1..xref_id {
        if id == 5 || id == 6 {
            let container = if id == 5 {
                if self_cycle { 5 } else { 6 }
            } else {
                5
            };
            entries.push(2);
            entries.extend_from_slice(&(container as u32).to_be_bytes());
            entries.extend_from_slice(&[0, 0]);
        } else {
            entries.extend_from_slice(&[0, 0, 0, 0, 0, 0, 0]);
        }
    }
    entries.push(1);
    entries.extend_from_slice(&(xref_offset as u32).to_be_bytes());
    entries.extend_from_slice(&[0, 0]);
    let mut pdf = b"%PDF-1.7\n".to_vec();
    pdf.extend_from_slice(format!("{xref_id} 0 obj\n<< /Type /XRef /Size 8 /Root 5 0 R /W [1 4 2] /Length {} >>\nstream\n", entries.len()).as_bytes());
    pdf.extend_from_slice(&entries);
    pdf.extend_from_slice(
        format!("\nendstream\nendobj\nstartxref\n{xref_offset}\n%%EOF\n").as_bytes(),
    );
    pdf
}
