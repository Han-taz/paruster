//! Private one-shot framing for the supervised PDF.js worker.

use crate::v8_runtime::PdfJsProbe;
use kordoc_ir::{ErrorCode, KordocError};
use serde::de::{DeserializeSeed, Error as DeError, SeqAccess, Visitor};
use serde::{Deserialize, Serialize};
use std::fmt;
use std::io::{self, Read, Write};

pub(super) const MAX_REQUEST_BYTES: usize = 32 * 1024 * 1024;
pub(super) const MAX_RESPONSE_BYTES: usize = 4 * 1024 * 1024;
const MAX_WORKER_PAGES: usize = 200;
const MAX_ERROR_MESSAGE_CHARS: usize = 512;
const HEADER_BYTES: usize = 10;
const MAGIC: &[u8; 4] = b"KPDF";
const VERSION: u8 = 1;
const REQUEST_KIND: u8 = 1;
const RESPONSE_KIND: u8 = 2;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum ProtocolError {
    Io,
    MalformedHeader,
    UnsupportedVersion,
    WrongKind,
    TooLarge,
    Truncated,
    TrailingBytes,
    InvalidJson,
    AllocationFailed,
}

pub(super) fn write_request<W: Write>(writer: &mut W, bytes: &[u8]) -> Result<(), ProtocolError> {
    if bytes.len() > MAX_REQUEST_BYTES {
        return Err(ProtocolError::TooLarge);
    }
    let length = u32::try_from(bytes.len()).map_err(|_| ProtocolError::TooLarge)?;
    writer
        .write_all(&make_header(REQUEST_KIND, length))
        .and_then(|()| writer.write_all(bytes))
        .and_then(|()| writer.flush())
        .map_err(|_| ProtocolError::Io)
}

pub(super) fn read_request<R: Read>(reader: &mut R) -> Result<Vec<u8>, ProtocolError> {
    let payload = read_payload(reader, REQUEST_KIND, MAX_REQUEST_BYTES)?;
    require_eof(reader)?;
    Ok(payload)
}

pub(super) fn write_response<W: Write>(
    writer: &mut W,
    result: Result<&PdfJsProbe, &KordocError>,
) -> Result<(), ProtocolError> {
    let response = match result {
        Ok(result)
            if result.page_count as usize <= MAX_WORKER_PAGES
                && result.page_text.len() == result.page_count as usize =>
        {
            ResponseOut::Success { result }
        }
        Ok(_) => return Err(ProtocolError::InvalidJson),
        Err(error) => ResponseOut::Failure {
            error: WireError {
                code: error.code,
                message: sanitize_message(&error.message),
            },
        },
    };
    let mut payload = BoundedJson::new(MAX_RESPONSE_BYTES);
    if serde_json::to_writer(&mut payload, &response).is_err() {
        return Err(if payload.too_large {
            ProtocolError::TooLarge
        } else if payload.allocation_failed {
            ProtocolError::AllocationFailed
        } else {
            ProtocolError::Io
        });
    }
    let length = u32::try_from(payload.bytes.len()).map_err(|_| ProtocolError::TooLarge)?;
    writer
        .write_all(&make_header(RESPONSE_KIND, length))
        .and_then(|()| writer.write_all(&payload.bytes))
        .and_then(|()| writer.flush())
        .map_err(|_| ProtocolError::Io)
}

pub(super) fn read_response<R: Read>(
    reader: &mut R,
) -> Result<Result<PdfJsProbe, KordocError>, ProtocolError> {
    let payload = read_payload(reader, RESPONSE_KIND, MAX_RESPONSE_BYTES)?;
    require_eof(reader)?;
    let response: ResponseIn =
        serde_json::from_slice(&payload).map_err(|_| ProtocolError::InvalidJson)?;
    match (response.status, response.result, response.error) {
        (ResponseStatus::Success, Some(result), None) => {
            if result.page_count as usize > MAX_WORKER_PAGES
                || result.page_text.0.len() != result.page_count as usize
            {
                return Err(ProtocolError::InvalidJson);
            }
            Ok(Ok(PdfJsProbe {
                page_count: result.page_count,
                page_text: result.page_text.0,
            }))
        }
        (ResponseStatus::Failure, None, Some(error)) => {
            Ok(Err(KordocError::new(error.code, error.message.0)))
        }
        _ => Err(ProtocolError::InvalidJson),
    }
}

fn make_header(kind: u8, length: u32) -> [u8; HEADER_BYTES] {
    let mut header = [0; HEADER_BYTES];
    header[..4].copy_from_slice(MAGIC);
    header[4] = VERSION;
    header[5] = kind;
    header[6..].copy_from_slice(&length.to_be_bytes());
    header
}

fn read_payload<R: Read>(
    reader: &mut R,
    expected_kind: u8,
    limit: usize,
) -> Result<Vec<u8>, ProtocolError> {
    let mut header = [0; HEADER_BYTES];
    reader.read_exact(&mut header).map_err(map_read_error)?;
    if &header[..4] != MAGIC {
        return Err(ProtocolError::MalformedHeader);
    }
    if header[4] != VERSION {
        return Err(ProtocolError::UnsupportedVersion);
    }
    if header[5] != expected_kind {
        return Err(ProtocolError::WrongKind);
    }
    let length = u32::from_be_bytes(header[6..10].try_into().expect("header length")) as usize;
    if length > limit {
        return Err(ProtocolError::TooLarge);
    }
    let mut payload = Vec::new();
    payload
        .try_reserve_exact(length)
        .map_err(|_| ProtocolError::AllocationFailed)?;
    payload.resize(length, 0);
    reader.read_exact(&mut payload).map_err(map_read_error)?;
    Ok(payload)
}

fn map_read_error(error: io::Error) -> ProtocolError {
    if error.kind() == io::ErrorKind::UnexpectedEof {
        ProtocolError::Truncated
    } else {
        ProtocolError::Io
    }
}

fn require_eof<R: Read>(reader: &mut R) -> Result<(), ProtocolError> {
    let mut extra = [0];
    loop {
        match reader.read(&mut extra) {
            Ok(0) => return Ok(()),
            Ok(_) => return Err(ProtocolError::TrailingBytes),
            Err(error) if error.kind() == io::ErrorKind::Interrupted => continue,
            Err(_) => return Err(ProtocolError::Io),
        }
    }
}

fn sanitize_message(message: &str) -> String {
    message
        .chars()
        .take(MAX_ERROR_MESSAGE_CHARS)
        .map(|character| {
            if character.is_control() {
                ' '
            } else {
                character
            }
        })
        .collect()
}

#[derive(Serialize)]
#[serde(tag = "status", rename_all = "snake_case", deny_unknown_fields)]
enum ResponseOut<'a> {
    Success { result: &'a PdfJsProbe },
    Failure { error: WireError },
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ResponseIn {
    status: ResponseStatus,
    #[serde(default)]
    result: Option<ProbeWire>,
    #[serde(default)]
    error: Option<WireErrorIn>,
}

#[derive(Deserialize)]
enum ResponseStatus {
    #[serde(rename = "success")]
    Success,
    #[serde(rename = "failure")]
    Failure,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct WireError {
    code: ErrorCode,
    message: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ProbeWire {
    page_count: u32,
    page_text: BoundedPageText,
}

struct BoundedPageText(Vec<String>);

impl<'de> Deserialize<'de> for BoundedPageText {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserializer.deserialize_seq(BoundedPageTextVisitor)
    }
}

struct BoundedPageTextVisitor;

impl<'de> Visitor<'de> for BoundedPageTextVisitor {
    type Value = BoundedPageText;

    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "an array of at most {MAX_WORKER_PAGES} page strings"
        )
    }

    fn visit_seq<A>(self, mut sequence: A) -> Result<Self::Value, A::Error>
    where
        A: SeqAccess<'de>,
    {
        let mut pages = Vec::new();
        if let Some(size) = sequence.size_hint() {
            pages
                .try_reserve_exact(size.min(MAX_WORKER_PAGES))
                .map_err(|_| A::Error::custom("page allocation failed"))?;
        }
        while pages.len() < MAX_WORKER_PAGES {
            match sequence.next_element::<String>()? {
                Some(page) => {
                    pages.push(page);
                    #[cfg(test)]
                    PAGE_STRINGS_DESERIALIZED.with(|count| count.set(count.get() + 1));
                }
                None => return Ok(BoundedPageText(pages)),
            }
        }
        if sequence.next_element_seed(RejectExtraPage)?.is_some() {
            unreachable!("RejectExtraPage always returns an error for a present value");
        }
        Ok(BoundedPageText(pages))
    }
}

struct RejectExtraPage;

impl<'de> DeserializeSeed<'de> for RejectExtraPage {
    type Value = ();

    fn deserialize<D>(self, _deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        Err(D::Error::custom("too many pages"))
    }
}

#[cfg(test)]
std::thread_local! {
    static PAGE_STRINGS_DESERIALIZED: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct WireErrorIn {
    code: ErrorCode,
    message: BoundedErrorMessage,
}

struct BoundedErrorMessage(String);

impl<'de> Deserialize<'de> for BoundedErrorMessage {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserializer.deserialize_str(BoundedErrorMessageVisitor)
    }
}

struct BoundedErrorMessageVisitor;

impl Visitor<'_> for BoundedErrorMessageVisitor {
    type Value = BoundedErrorMessage;

    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("a diagnostic string")
    }

    fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
    where
        E: DeError,
    {
        Ok(BoundedErrorMessage(sanitize_message(value)))
    }

    fn visit_string<E>(self, value: String) -> Result<Self::Value, E>
    where
        E: DeError,
    {
        Ok(BoundedErrorMessage(sanitize_message(&value)))
    }
}

struct BoundedJson {
    bytes: Vec<u8>,
    limit: usize,
    too_large: bool,
    allocation_failed: bool,
}

impl BoundedJson {
    fn new(limit: usize) -> Self {
        Self {
            bytes: Vec::new(),
            limit,
            too_large: false,
            allocation_failed: false,
        }
    }
}

impl Write for BoundedJson {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        if self
            .bytes
            .len()
            .checked_add(bytes.len())
            .is_none_or(|length| length > self.limit)
        {
            self.too_large = true;
            return Err(io::Error::new(
                io::ErrorKind::WriteZero,
                "response cap exceeded",
            ));
        }
        if self.bytes.try_reserve(bytes.len()).is_err() {
            self.allocation_failed = true;
            return Err(io::Error::new(
                io::ErrorKind::OutOfMemory,
                "response allocation failed",
            ));
        }
        self.bytes.extend_from_slice(bytes);
        Ok(bytes.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use kordoc_ir::{ErrorCode, KordocError};
    use std::io::Cursor;

    fn probe(text: String) -> PdfJsProbe {
        PdfJsProbe {
            page_count: 1,
            page_text: vec![text],
        }
    }

    #[test]
    fn request_roundtrip_uses_fixed_header_and_exact_eof() {
        let bytes = b"%PDF-1.7\ncontents";
        let mut frame = Vec::new();
        write_request(&mut frame, bytes).unwrap();
        assert_eq!(&frame[..10], b"KPDF\x01\x01\x00\x00\x00\x11");
        assert_eq!(read_request(&mut Cursor::new(frame)).unwrap(), bytes);
    }

    #[test]
    fn response_roundtrip_carries_success_and_stable_error() {
        let expected = probe("V8 PDF.js probe 한글".to_owned());
        let mut frame = Vec::new();
        write_response(&mut frame, Ok(&expected)).unwrap();
        assert_eq!(
            read_response(&mut Cursor::new(frame)).unwrap(),
            Ok(expected)
        );

        let expected_error = KordocError::new(ErrorCode::ParseError, "malformed PDF");
        let mut frame = Vec::new();
        write_response(&mut frame, Err(&expected_error)).unwrap();
        assert_eq!(
            read_response(&mut Cursor::new(frame)).unwrap(),
            Err(expected_error)
        );
    }

    #[test]
    fn request_cap_is_inclusive_and_rejects_plus_one_before_writing() {
        let bytes = vec![0x5a; MAX_REQUEST_BYTES];
        let mut frame = Vec::new();
        write_request(&mut frame, &bytes).unwrap();
        assert_eq!(frame.len(), 10 + MAX_REQUEST_BYTES);
        assert_eq!(read_request(&mut Cursor::new(frame)).unwrap(), bytes);

        let mut frame = Vec::new();
        assert_eq!(
            write_request(&mut frame, &vec![0; MAX_REQUEST_BYTES + 1]),
            Err(ProtocolError::TooLarge)
        );
        assert!(frame.is_empty());
    }

    #[test]
    fn response_cap_is_inclusive_and_rejects_plus_one() {
        let empty = probe(String::new());
        let mut encoded_empty = Vec::new();
        write_response(&mut encoded_empty, Ok(&empty)).unwrap();
        let empty_payload_size = encoded_empty.len() - 10;
        let exact = probe("x".repeat(MAX_RESPONSE_BYTES - empty_payload_size));
        let mut frame = Vec::new();
        write_response(&mut frame, Ok(&exact)).unwrap();
        assert_eq!(frame.len(), 10 + MAX_RESPONSE_BYTES);
        assert_eq!(read_response(&mut Cursor::new(frame)).unwrap(), Ok(exact));

        let oversized = probe("x".repeat(MAX_RESPONSE_BYTES - empty_payload_size + 1));
        let mut frame = Vec::new();
        assert_eq!(
            write_response(&mut frame, Ok(&oversized)),
            Err(ProtocolError::TooLarge)
        );
        assert!(frame.is_empty());
    }

    #[test]
    fn malformed_truncated_wrong_kind_and_trailing_frames_fail() {
        let mut malformed = b"NOPE\x01\x01\x00\x00\x00\x00".to_vec();
        assert_eq!(
            read_request(&mut Cursor::new(malformed)),
            Err(ProtocolError::MalformedHeader)
        );

        malformed = b"KPDF\x02\x01\x00\x00\x00\x00".to_vec();
        assert_eq!(
            read_request(&mut Cursor::new(malformed)),
            Err(ProtocolError::UnsupportedVersion)
        );

        malformed = b"KPDF\x01\x02\x00\x00\x00\x00".to_vec();
        assert_eq!(
            read_request(&mut Cursor::new(malformed)),
            Err(ProtocolError::WrongKind)
        );

        malformed = b"KPDF\x01\x01\x00\x00\x00\x02x".to_vec();
        assert_eq!(
            read_request(&mut Cursor::new(malformed)),
            Err(ProtocolError::Truncated)
        );

        malformed = b"KPDF\x01\x01\x00\x00\x00\x00x".to_vec();
        assert_eq!(
            read_request(&mut Cursor::new(malformed)),
            Err(ProtocolError::TrailingBytes)
        );
    }

    #[test]
    fn request_header_rejects_oversize_without_payload_allocation() {
        let frame = [
            b"KPDF\x01\x01".as_slice(),
            &((MAX_REQUEST_BYTES as u32 + 1).to_be_bytes()),
        ]
        .concat();
        assert_eq!(
            read_request(&mut Cursor::new(frame)),
            Err(ProtocolError::TooLarge)
        );
    }

    #[test]
    fn response_requires_utf8_valid_json_and_exact_eof() {
        let frame = [b"KPDF\x01\x02".as_slice(), &3u32.to_be_bytes(), b"\xff{}"].concat();
        assert_eq!(
            read_response(&mut Cursor::new(frame)),
            Err(ProtocolError::InvalidJson)
        );

        let mut frame = Vec::new();
        write_response(&mut frame, Ok(&probe("x".into()))).unwrap();
        frame.push(0);
        assert_eq!(
            read_response(&mut Cursor::new(frame)),
            Err(ProtocolError::TrailingBytes)
        );
    }

    #[test]
    fn response_header_rejects_oversize_before_payload_allocation() {
        let header = [
            b"KPDF\x01\x02".as_slice(),
            &((MAX_RESPONSE_BYTES as u32 + 1).to_be_bytes()),
        ]
        .concat();
        assert_eq!(
            read_response(&mut Cursor::new(header)),
            Err(ProtocolError::TooLarge)
        );
    }

    fn response_frame(json: &[u8]) -> Vec<u8> {
        [
            make_header(RESPONSE_KIND, json.len() as u32).as_slice(),
            json,
        ]
        .concat()
    }

    #[test]
    fn response_rejects_page_count_mismatch_and_excess_pages() {
        let mismatch = br#"{"result":{"page_count":1,"page_text":["a","b"]},"status":"success"}"#;
        assert_eq!(
            read_response(&mut Cursor::new(response_frame(mismatch))),
            Err(ProtocolError::InvalidJson)
        );

        let too_many = format!(
            "{{\"result\":{{\"page_count\":201,\"page_text\":[{}]}},\"status\":\"success\"}}",
            std::iter::repeat_n("\"\"", 201)
                .collect::<Vec<_>>()
                .join(",")
        );
        assert_eq!(
            read_response(&mut Cursor::new(response_frame(too_many.as_bytes()))),
            Err(ProtocolError::InvalidJson)
        );
    }

    #[test]
    fn response_page_limit_is_inclusive() {
        let page_text = std::iter::repeat_n("\"\"", 200)
            .collect::<Vec<_>>()
            .join(",");
        let exact = format!(
            "{{\"status\":\"success\",\"result\":{{\"page_count\":200,\"page_text\":[{page_text}]}}}}"
        );
        let decoded = read_response(&mut Cursor::new(response_frame(exact.as_bytes()))).unwrap();
        let probe = decoded.unwrap();
        assert_eq!(probe.page_count, 200);
        assert_eq!(probe.page_text.len(), 200);
    }

    #[test]
    fn hostile_huge_page_array_is_rejected_with_the_small_page_budget() {
        let mut json = String::with_capacity(MAX_RESPONSE_BYTES);
        json.push_str(r#"{"result":{"page_count":200,"page_text":["""#);
        let suffix = r#"],"status":"success"}"#;
        while json.len() + 3 + suffix.len() <= MAX_RESPONSE_BYTES {
            json.push_str(",\"\"");
        }
        json.push_str(suffix);
        assert!(json.len() <= MAX_RESPONSE_BYTES);
        assert!(json.len() >= MAX_RESPONSE_BYTES - 2);
        PAGE_STRINGS_DESERIALIZED.with(|count| count.set(0));
        assert_eq!(
            read_response(&mut Cursor::new(response_frame(json.as_bytes()))),
            Err(ProtocolError::InvalidJson)
        );
        PAGE_STRINGS_DESERIALIZED.with(|count| assert_eq!(count.get(), MAX_WORKER_PAGES));
    }

    #[test]
    fn page_array_over_cap_is_rejected_when_status_field_is_last() {
        let mut json = String::from(r#"{"result":{"page_count":200,"page_text":["""#);
        for _ in 1..MAX_WORKER_PAGES {
            json.push_str(",\"\"");
        }
        json.push_str(",[]]},\"status\":\"success\"}");
        PAGE_STRINGS_DESERIALIZED.with(|count| count.set(0));
        let error = match serde_json::from_slice::<ResponseIn>(json.as_bytes()) {
            Ok(_) => panic!("expected the page budget to reject the response"),
            Err(error) => error,
        };
        assert!(error.to_string().contains("too many pages"));
        PAGE_STRINGS_DESERIALIZED.with(|count| assert_eq!(count.get(), MAX_WORKER_PAGES));
    }

    #[test]
    fn worker_refuses_invalid_page_count_before_serializing_response() {
        let invalid = PdfJsProbe {
            page_count: 201,
            page_text: vec![String::new(); 201],
        };
        let mut frame = Vec::new();
        assert_eq!(
            write_response(&mut frame, Ok(&invalid)),
            Err(ProtocolError::InvalidJson)
        );
        assert!(frame.is_empty());
    }

    #[test]
    fn failure_messages_are_sanitized_before_serialization() {
        let error = KordocError::new(ErrorCode::ParseError, format!("line\n{}", "x".repeat(900)));
        let mut frame = Vec::new();
        write_response(&mut frame, Err(&error)).unwrap();
        let decoded = read_response(&mut Cursor::new(frame)).unwrap().unwrap_err();
        assert!(!decoded.message.contains('\n'));
        assert!(decoded.message.chars().count() <= 512);
    }

    #[test]
    fn failure_messages_roundtrip_quotes_backslashes_and_unicode() {
        let error = KordocError::new(ErrorCode::ParseError, r#"bad "quoted" C:\foo 한글🧪"#);
        let mut frame = Vec::new();
        write_response(&mut frame, Err(&error)).unwrap();
        assert_eq!(read_response(&mut Cursor::new(frame)).unwrap(), Err(error));
    }

    #[test]
    fn incoming_failure_diagnostic_is_bounded_to_512_characters() {
        let message = "x".repeat(900);
        let json = format!(
            "{{\"status\":\"failure\",\"error\":{{\"code\":\"PARSE_ERROR\",\"message\":\"{message}\"}}}}"
        );
        let error = read_response(&mut Cursor::new(response_frame(json.as_bytes())))
            .unwrap()
            .unwrap_err();
        assert_eq!(error.message.chars().count(), 512);
    }
}
