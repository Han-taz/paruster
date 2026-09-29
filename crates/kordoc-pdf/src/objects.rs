use crate::limits::{BudgetError, PdfBudget};
use std::collections::{HashMap, HashSet};
use std::fmt;
use std::fs::File;
use std::io;
use std::path::Path;

#[derive(Debug)]
pub enum PdfReadError {
    Budget(BudgetError),
    Io(io::Error),
    Corrupted,
    Encrypted,
    MissingObject,
    GenerationMismatch,
    ReferenceCycle,
    UnsupportedFilter,
}

impl fmt::Display for PdfReadError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Budget(error) => error.fmt(f),
            Self::Io(_) => f.write_str("PDF input could not be read"),
            Self::Corrupted => f.write_str("PDF structure is corrupted"),
            Self::Encrypted => f.write_str("encrypted PDF is not supported by the object reader"),
            Self::MissingObject => f.write_str("PDF object is unavailable"),
            Self::GenerationMismatch => f.write_str("PDF object generation is invalid"),
            Self::ReferenceCycle => f.write_str("PDF object reference cycle detected"),
            Self::UnsupportedFilter => f.write_str("PDF stream filter is unsupported"),
        }
    }
}

impl std::error::Error for PdfReadError {}
impl From<BudgetError> for PdfReadError {
    fn from(value: BudgetError) -> Self {
        Self::Budget(value)
    }
}
impl From<io::Error> for PdfReadError {
    fn from(value: io::Error) -> Self {
        Self::Io(value)
    }
}
impl From<crate::stream::StreamDecodeError> for PdfReadError {
    fn from(value: crate::stream::StreamDecodeError) -> Self {
        match value {
            crate::stream::StreamDecodeError::Budget(error) => Self::Budget(error),
            crate::stream::StreamDecodeError::Malformed => Self::Corrupted,
        }
    }
}

#[derive(Debug, Clone)]
pub enum PdfValue<'a> {
    Dictionary(&'a [u8]),
    OwnedDictionary(Vec<u8>),
    Owned(Vec<u8>),
    Array(&'a [u8]),
    Number(i64),
    Name(&'a [u8]),
    Other(&'a [u8]),
}

impl PdfValue<'_> {
    pub fn as_bytes(&self) -> &[u8] {
        match self {
            Self::Dictionary(bytes)
            | Self::Array(bytes)
            | Self::Name(bytes)
            | Self::Other(bytes) => bytes,
            Self::OwnedDictionary(bytes) | Self::Owned(bytes) => bytes,
            Self::Number(_) => &[],
        }
    }
}

#[derive(Debug, Clone, Copy)]
struct RawObject {
    start: usize,
    end: usize,
}

#[derive(Debug, Clone, Copy)]
enum XrefEntry {
    Normal { offset: usize, generation: u16 },
    Compressed { stream: u32, index: usize },
    Free,
}

pub struct PdfObjectReader<'a> {
    source: &'a [u8],
    objects: HashMap<(u32, u16), RawObject>,
    xref: HashMap<u32, XrefEntry>,
    budget: PdfBudget,
    active: HashSet<(u32, u16)>,
}

impl<'a> PdfObjectReader<'a> {
    pub fn new(source: &'a [u8]) -> Result<Self, PdfReadError> {
        let mut budget = PdfBudget::default();
        budget.charge_source(source.len() as u64)?;
        if !source.starts_with(b"%PDF-") {
            return Err(PdfReadError::Corrupted);
        }
        let (xref, trailer) = parse_xref(source, &mut budget)?;
        if trailer_contains(&trailer, b"Encrypt") {
            return Err(PdfReadError::Encrypted);
        }
        if xref.is_empty() {
            return Err(PdfReadError::Corrupted);
        }
        Ok(Self {
            source,
            objects: HashMap::new(),
            xref,
            budget,
            active: HashSet::new(),
        })
    }

    pub fn preflight_file(path: &Path) -> Result<u64, PdfReadError> {
        let file = File::open(path)?;
        let size = file.metadata()?.len();
        PdfBudget::default().charge_source(size)?;
        Ok(size)
    }

    pub fn resolve(&mut self, id: (u32, u16)) -> Result<PdfValue<'a>, PdfReadError> {
        self.resolve_inner(id)
    }

    fn resolve_inner(&mut self, id: (u32, u16)) -> Result<PdfValue<'a>, PdfReadError> {
        self.resolve_scoped(id, true)
    }

    fn resolve_scoped(
        &mut self,
        id: (u32, u16),
        charge_depth: bool,
    ) -> Result<PdfValue<'a>, PdfReadError> {
        if !self.active.insert(id) {
            return Err(PdfReadError::ReferenceCycle);
        }
        if charge_depth && let Err(error) = self.budget.enter_object() {
            self.active.remove(&id);
            return Err(error.into());
        }
        let result = self.resolve_untracked(id);
        if charge_depth {
            self.budget.leave_object();
        }
        self.active.remove(&id);
        result
    }

    fn resolve_untracked(&mut self, id: (u32, u16)) -> Result<PdfValue<'a>, PdfReadError> {
        self.budget.charge_deref()?;
        if let Some(entry) = self.xref.get(&id.0) {
            match *entry {
                XrefEntry::Free => return Err(PdfReadError::MissingObject),
                XrefEntry::Normal { generation, offset } => {
                    if generation != id.1 {
                        return Err(PdfReadError::GenerationMismatch);
                    }
                    let raw = *self.objects.entry(id).or_insert(parse_raw_at(
                        self.source,
                        offset,
                        id.0,
                        id.1,
                    )?);
                    return value_for(&self.source[raw.start..raw.end]);
                }
                XrefEntry::Compressed { stream, index } => {
                    return self.resolve_compressed(id, stream, index);
                }
            }
        }
        let XrefEntry::Normal { offset, generation } =
            *self.xref.get(&id.0).ok_or(PdfReadError::MissingObject)?
        else {
            return Err(PdfReadError::MissingObject);
        };
        if generation != id.1 {
            return Err(PdfReadError::GenerationMismatch);
        }
        let raw = *self
            .objects
            .entry(id)
            .or_insert(parse_raw_at(self.source, offset, id.0, id.1)?);
        value_for(&self.source[raw.start..raw.end])
    }

    fn resolve_compressed(
        &mut self,
        id: (u32, u16),
        stream_id: u32,
        index: usize,
    ) -> Result<PdfValue<'a>, PdfReadError> {
        if id.1 != 0 {
            return Err(PdfReadError::GenerationMismatch);
        }
        let stream_value = self.resolve_inner((stream_id, 0))?;
        let dict = stream_value.as_bytes();
        let bytes = self.read_stream((stream_id, 0))?;
        let first = dictionary_integer(dict, b"First").ok_or(PdfReadError::Corrupted)?;
        let count = dictionary_integer(dict, b"N").ok_or(PdfReadError::Corrupted)?;
        if index >= count {
            return Err(PdfReadError::Corrupted);
        }
        let header = bytes.get(..first).ok_or(PdfReadError::Corrupted)?;
        let pairs = numbers(header);
        let at = index.checked_mul(2).ok_or(PdfReadError::Corrupted)?;
        if pairs.len() < at + 2 || pairs[at] != id.0 as usize {
            return Err(PdfReadError::Corrupted);
        }
        let start = first
            .checked_add(pairs[at + 1])
            .ok_or(PdfReadError::Corrupted)?;
        let end = if index + 1 < count {
            first
                .checked_add(*pairs.get(at + 3).ok_or(PdfReadError::Corrupted)?)
                .ok_or(PdfReadError::Corrupted)?
        } else {
            bytes.len()
        };
        let slice = bytes.get(start..end).ok_or(PdfReadError::Corrupted)?;
        let owned = trim(slice).to_vec();
        if owned.starts_with(b"<<") {
            Ok(PdfValue::OwnedDictionary(owned))
        } else {
            Ok(PdfValue::Owned(owned))
        }
    }

    pub fn resolve_graph(&mut self, id: (u32, u16)) -> Result<(), PdfReadError> {
        self.resolve_graph_inner(id, &mut HashSet::new())
    }

    fn resolve_graph_inner(
        &mut self,
        id: (u32, u16),
        seen: &mut HashSet<(u32, u16)>,
    ) -> Result<(), PdfReadError> {
        if !seen.insert(id) {
            return Err(PdfReadError::ReferenceCycle);
        }
        self.budget.enter_object()?;
        let result: Result<(), PdfReadError> = (|| {
            let value = self.resolve_scoped(id, false)?;
            let refs = references(value.as_bytes());
            for next in refs {
                self.resolve_graph_inner(next, seen)?;
            }
            Ok(())
        })();
        self.budget.leave_object();
        result?;
        seen.remove(&id);
        Ok(())
    }

    pub fn read_stream(&mut self, id: (u32, u16)) -> Result<Vec<u8>, PdfReadError> {
        let body = self.object_body_checked(id)?;
        let stream_pos = find_pdf_keyword(body, b"stream")
            .and_then(|index| index.checked_add(6))
            .ok_or(PdfReadError::Corrupted)?;
        let data_start = after_line_ending(body, stream_pos).ok_or(PdfReadError::Corrupted)?;
        let length = dictionary_integer(body, b"Length").ok_or(PdfReadError::Corrupted)?;
        let data_end = data_start
            .checked_add(length)
            .ok_or(PdfReadError::Corrupted)?;
        let encoded = body
            .get(data_start..data_end)
            .ok_or(PdfReadError::Corrupted)?;
        self.budget.finish_stream();
        let filter = filter_names(body);
        if filter.len() > 1 {
            return Err(PdfReadError::UnsupportedFilter);
        }
        let output = match filter.first().copied() {
            None => crate::stream::copy_unfiltered(encoded, &mut self.budget)?,
            Some(b"ASCIIHexDecode") | Some(b"AHx") => {
                crate::stream::decode_ascii_hex(encoded, &mut self.budget)?
            }
            Some(_) => return Err(PdfReadError::UnsupportedFilter),
        };
        self.budget.finish_stream();
        Ok(output)
    }

    fn object_body_checked(&mut self, id: (u32, u16)) -> Result<&'a [u8], PdfReadError> {
        self.budget.charge_deref()?;
        let XrefEntry::Normal { offset, generation } =
            *self.xref.get(&id.0).ok_or(PdfReadError::MissingObject)?
        else {
            return Err(PdfReadError::Corrupted);
        };
        if generation != id.1 {
            return Err(PdfReadError::GenerationMismatch);
        }
        let raw = *self
            .objects
            .entry(id)
            .or_insert(parse_raw_at(self.source, offset, id.0, id.1)?);
        Ok(&self.source[raw.start..raw.end])
    }
}

fn parse_xref(
    source: &[u8],
    budget: &mut PdfBudget,
) -> Result<(HashMap<u32, XrefEntry>, Vec<u8>), PdfReadError> {
    let Some(start) = find_last_tail(source, b"startxref", 64 * 1024) else {
        return Err(PdfReadError::Corrupted);
    };
    let offset = direct_integer(&source[start + 9..]).ok_or(PdfReadError::Corrupted)?;
    let mut visited = HashSet::new();
    let mut entries = HashMap::new();
    let mut trailer = Vec::new();
    parse_xref_at(
        source,
        offset,
        budget,
        &mut visited,
        &mut entries,
        &mut trailer,
    )?;
    Ok((entries, trailer))
}

fn parse_xref_at(
    source: &[u8],
    offset: usize,
    budget: &mut PdfBudget,
    visited: &mut HashSet<usize>,
    entries: &mut HashMap<u32, XrefEntry>,
    trailer: &mut Vec<u8>,
) -> Result<(), PdfReadError> {
    if !visited.insert(offset) || visited.len() > 64 {
        return Err(PdfReadError::Corrupted);
    }
    if source
        .get(offset..)
        .is_some_and(|tail| tail.starts_with(b"xref"))
    {
        parse_classic_xref(source, offset, budget, visited, entries, trailer)
    } else {
        parse_xref_stream_at(source, offset, budget, visited, entries, trailer)
    }
}

fn parse_classic_xref(
    source: &[u8],
    offset: usize,
    budget: &mut PdfBudget,
    visited: &mut HashSet<usize>,
    output: &mut HashMap<u32, XrefEntry>,
    trailer: &mut Vec<u8>,
) -> Result<(), PdfReadError> {
    const XREF_SCAN_LIMIT: usize = 32 * 1024 * 1024 + 64 * 1024;
    let section_end = offset.saturating_add(XREF_SCAN_LIMIT).min(source.len());
    let section = &source[offset..section_end];
    let mut object_id = 0u32;
    for line in section.split(|b| *b == b'\n').skip(1) {
        let line = trim(line);
        if line.starts_with(b"trailer") {
            let current = trailer_dictionary(section).ok_or(PdfReadError::Corrupted)?;
            let prev = dictionary_integer(current, b"Prev");
            if trailer.is_empty() {
                *trailer = current.to_vec();
            }
            if let Some(prev) = prev {
                parse_xref_at(source, prev, budget, visited, output, trailer)?;
            }
            break;
        }
        let fields: Vec<&[u8]> = line
            .split(|b| b.is_ascii_whitespace())
            .filter(|f| !f.is_empty())
            .collect();
        if fields.len() == 2 && fields.iter().all(|f| f.iter().all(u8::is_ascii_digit)) {
            object_id =
                u32::try_from(parse_usize(fields[0])?).map_err(|_| PdfReadError::Corrupted)?;
        } else if fields.len() >= 3 && matches!(fields[2], b"n" | b"f") {
            let file_offset = parse_usize(fields[0])?;
            let generation =
                u16::try_from(parse_usize(fields[1])?).map_err(|_| PdfReadError::Corrupted)?;
            let entry = if fields[2] == b"n" {
                XrefEntry::Normal {
                    offset: file_offset,
                    generation,
                }
            } else {
                XrefEntry::Free
            };
            if !output.contains_key(&object_id) && !matches!(entry, XrefEntry::Free) {
                budget.charge_object()?;
            }
            output.entry(object_id).or_insert(entry);
            object_id = object_id.checked_add(1).ok_or(PdfReadError::Corrupted)?;
        }
    }
    Ok(())
}

fn parse_xref_stream_at(
    source: &[u8],
    offset: usize,
    budget: &mut PdfBudget,
    visited: &mut HashSet<usize>,
    output: &mut HashMap<u32, XrefEntry>,
    trailer: &mut Vec<u8>,
) -> Result<(), PdfReadError> {
    let raw = parse_raw_at_any_id(source, offset)?;
    let body = &source[raw.start..raw.end];
    if find(body, b"/Type /XRef").is_none() {
        return Err(PdfReadError::Corrupted);
    }
    let stream_marker = find_pdf_keyword(body, b"stream").ok_or(PdfReadError::Corrupted)?;
    let dictionary = &body[..stream_marker];
    let prev = dictionary_integer(dictionary, b"Prev");
    if trailer.is_empty() {
        *trailer = dictionary.to_vec();
    }
    if !filter_names(dictionary).is_empty() {
        return Err(PdfReadError::UnsupportedFilter);
    }
    let data_start = stream_marker
        .checked_add(6)
        .and_then(|at| after_line_ending(body, at))
        .ok_or(PdfReadError::Corrupted)?;
    let length = dictionary_integer(body, b"Length").ok_or(PdfReadError::Corrupted)?;
    if length > 32 * 1024 * 1024 {
        return Err(PdfReadError::Budget(BudgetError::StreamBytes));
    }
    let data_end = data_start
        .checked_add(length)
        .ok_or(PdfReadError::Corrupted)?;
    let data = body
        .get(data_start..data_end)
        .ok_or(PdfReadError::Corrupted)?;
    budget.charge_decoded(length as u64)?;
    budget.finish_stream();
    let widths = integer_array(body, b"W").ok_or(PdfReadError::Corrupted)?;
    let index = if find_dictionary_key(body, b"Index").is_some() {
        integer_array(body, b"Index").ok_or(PdfReadError::Corrupted)?
    } else {
        vec![
            0,
            dictionary_integer(body, b"Size").ok_or(PdfReadError::Corrupted)?,
        ]
    };
    if widths.len() != 3 {
        return Err(PdfReadError::Corrupted);
    }
    let stride = widths
        .iter()
        .try_fold(0usize, |sum, width| sum.checked_add(*width))
        .ok_or(PdfReadError::Corrupted)?;
    if stride == 0 || stride > 16 || widths.iter().any(|width| *width > 8) {
        return Err(PdfReadError::Corrupted);
    }
    if index.len() % 2 != 0 {
        return Err(PdfReadError::Corrupted);
    }
    let records = index
        .chunks_exact(2)
        .try_fold(0usize, |sum, range| {
            let end = range[0].checked_add(range[1])?;
            let _ = u32::try_from(end).ok()?;
            sum.checked_add(range[1])
        })
        .ok_or(PdfReadError::Corrupted)?;
    if records
        .checked_mul(stride)
        .is_none_or(|required| required > data.len())
    {
        return Err(PdfReadError::Corrupted);
    }
    let mut previous_end = 0usize;
    let mut required_objects = 0u64;
    let mut preflight_cursor = 0usize;
    for range in index.chunks_exact(2) {
        let range_end = range[0]
            .checked_add(range[1])
            .ok_or(PdfReadError::Corrupted)?;
        if range[0] < previous_end {
            return Err(PdfReadError::Corrupted);
        }
        previous_end = range_end;
        for object in range[0]..range_end {
            let record_end = preflight_cursor
                .checked_add(stride)
                .ok_or(PdfReadError::Corrupted)?;
            let record = data
                .get(preflight_cursor..record_end)
                .ok_or(PdfReadError::Corrupted)?;
            let kind = if widths[0] == 0 {
                1
            } else {
                read_be(&record[..widths[0]]).ok_or(PdfReadError::Corrupted)?
            };
            if !matches!(kind, 0..=2) {
                return Err(PdfReadError::Corrupted);
            }
            let object_id = u32::try_from(object).map_err(|_| PdfReadError::Corrupted)?;
            if kind != 0 && !output.contains_key(&object_id) {
                required_objects = required_objects
                    .checked_add(1)
                    .ok_or(PdfReadError::Corrupted)?;
            }
            preflight_cursor = record_end;
        }
    }
    budget.charge_objects(required_objects)?;
    let mut byte_cursor = 0usize;
    for range in index.chunks_exact(2) {
        let range_end = range[0]
            .checked_add(range[1])
            .ok_or(PdfReadError::Corrupted)?;
        for object in range[0]..range_end {
            let record_end = byte_cursor
                .checked_add(stride)
                .ok_or(PdfReadError::Corrupted)?;
            let record = data
                .get(byte_cursor..record_end)
                .ok_or(PdfReadError::Corrupted)?;
            let widths01 = widths[0]
                .checked_add(widths[1])
                .ok_or(PdfReadError::Corrupted)?;
            let fields = [
                read_be(&record[..widths[0]]).ok_or(PdfReadError::Corrupted)?,
                read_be(&record[widths[0]..widths01]).ok_or(PdfReadError::Corrupted)?,
                read_be(&record[widths01..]).ok_or(PdfReadError::Corrupted)?,
            ];
            let kind = if widths[0] == 0 { 1 } else { fields[0] };
            let entry = match kind {
                0 => XrefEntry::Free,
                1 => XrefEntry::Normal {
                    offset: fields[1],
                    generation: u16::try_from(fields[2]).map_err(|_| PdfReadError::Corrupted)?,
                },
                2 => XrefEntry::Compressed {
                    stream: u32::try_from(fields[1]).map_err(|_| PdfReadError::Corrupted)?,
                    index: fields[2],
                },
                _ => return Err(PdfReadError::Corrupted),
            };
            let object = u32::try_from(object).map_err(|_| PdfReadError::Corrupted)?;
            output.entry(object).or_insert(entry);
            byte_cursor = record_end;
        }
    }
    if let Some(prev) = prev {
        parse_xref_at(source, prev, budget, visited, output, trailer)?;
    }
    Ok(())
}

fn value_for(bytes: &[u8]) -> Result<PdfValue<'_>, PdfReadError> {
    let bytes = trim(bytes);
    if bytes.starts_with(b"<<") {
        Ok(PdfValue::Dictionary(bytes))
    } else if bytes.starts_with(b"[") {
        Ok(PdfValue::Array(bytes))
    } else if bytes.first() == Some(&b'/') {
        Ok(PdfValue::Name(bytes))
    } else if let Ok(value) = std::str::from_utf8(bytes)
        .unwrap_or("")
        .trim()
        .parse::<i64>()
    {
        Ok(PdfValue::Number(value))
    } else if !bytes.is_empty() {
        Ok(PdfValue::Other(bytes))
    } else {
        Err(PdfReadError::Corrupted)
    }
}

fn parse_raw_at_any_id(source: &[u8], offset: usize) -> Result<RawObject, PdfReadError> {
    let tail = source.get(offset..).ok_or(PdfReadError::Corrupted)?;
    let header = tail
        .get(..tail.len().min(256))
        .ok_or(PdfReadError::Corrupted)?;
    let line_end = header
        .iter()
        .position(|b| *b == b'\n')
        .ok_or(PdfReadError::Corrupted)?;
    let fields: Vec<&[u8]> = header[..line_end]
        .split(|b| b.is_ascii_whitespace())
        .filter(|f| !f.is_empty())
        .collect();
    if fields.len() < 3 || fields[2] != b"obj" {
        return Err(PdfReadError::Corrupted);
    }
    let number = u32::try_from(parse_usize(fields[0])?).map_err(|_| PdfReadError::Corrupted)?;
    let generation = u16::try_from(parse_usize(fields[1])?).map_err(|_| PdfReadError::Corrupted)?;
    parse_raw_at(source, offset, number, generation)
}

fn parse_raw_at(
    source: &[u8],
    offset: usize,
    number: u32,
    generation: u16,
) -> Result<RawObject, PdfReadError> {
    let tail = source.get(offset..).ok_or(PdfReadError::Corrupted)?;
    let header = tail
        .get(..tail.len().min(256))
        .ok_or(PdfReadError::Corrupted)?;
    let line_end = header
        .iter()
        .position(|b| *b == b'\n')
        .ok_or(PdfReadError::Corrupted)?;
    let fields: Vec<&[u8]> = header[..line_end]
        .split(|b| b.is_ascii_whitespace())
        .filter(|f| !f.is_empty())
        .collect();
    if fields.len() < 3
        || fields[2] != b"obj"
        || parse_usize(fields[0])? != number as usize
        || parse_usize(fields[1])? != generation as usize
    {
        return Err(PdfReadError::GenerationMismatch);
    }
    let start = offset + line_end + 1;
    const OBJECT_SCAN_LIMIT: usize = 32 * 1024 * 1024 + 64 * 1024;
    let body_end = start.saturating_add(OBJECT_SCAN_LIMIT).min(source.len());
    let body = &source[start..body_end];
    let end = if let Some(stream_rel) = stream_marker_after_dictionary(body) {
        let length =
            dictionary_integer(&body[..stream_rel], b"Length").ok_or(PdfReadError::Corrupted)?;
        if length > 32 * 1024 * 1024 {
            return Err(PdfReadError::Budget(BudgetError::StreamBytes));
        }
        let after_marker = stream_rel.checked_add(6).ok_or(PdfReadError::Corrupted)?;
        let data_start_rel =
            after_line_ending(body, after_marker).ok_or(PdfReadError::Corrupted)?;
        let data_end_rel = data_start_rel
            .checked_add(length)
            .ok_or(PdfReadError::Corrupted)?;
        let after_data = body.get(data_end_rel..).ok_or(PdfReadError::Corrupted)?;
        let endstream_rel = after_line_ending_optional(after_data);
        if !endstream_rel.starts_with(b"endstream")
            || endstream_rel
                .get(9)
                .is_some_and(|byte| !is_pdf_delimiter(*byte))
        {
            return Err(PdfReadError::Corrupted);
        }
        let after_endstream = data_end_rel
            .checked_add(after_data.len() - endstream_rel.len() + 9)
            .ok_or(PdfReadError::Corrupted)?;
        let endobj_rel =
            find_pdf_keyword(&body[after_endstream..], b"endobj").ok_or(PdfReadError::Corrupted)?;
        start + after_endstream + endobj_rel
    } else {
        start + find_pdf_keyword(body, b"endobj").ok_or(PdfReadError::Corrupted)?
    };
    Ok(RawObject { start, end })
}

fn after_line_ending(bytes: &[u8], at: usize) -> Option<usize> {
    match bytes.get(at..at + 2) {
        Some(b"\r\n") => Some(at + 2),
        _ if bytes.get(at) == Some(&b'\n') || bytes.get(at) == Some(&b'\r') => Some(at + 1),
        _ => None,
    }
}

fn after_line_ending_optional(bytes: &[u8]) -> &[u8] {
    match bytes {
        [b'\r', b'\n', rest @ ..] => rest,
        [b'\n' | b'\r', rest @ ..] => rest,
        _ => bytes,
    }
}

fn stream_marker_after_dictionary(bytes: &[u8]) -> Option<usize> {
    if !bytes.starts_with(b"<<") {
        return None;
    }
    let mut index = 2usize;
    let mut depth = 1usize;
    while index + 1 < bytes.len() {
        match bytes[index] {
            b'%' => {
                while index < bytes.len() && !matches!(bytes[index], b'\n' | b'\r') {
                    index += 1;
                }
            }
            b'(' => {
                index += 1;
                let mut nesting = 1usize;
                while index < bytes.len() && nesting != 0 {
                    match bytes[index] {
                        b'\\' => index = (index + 2).min(bytes.len()),
                        b'(' => {
                            nesting += 1;
                            index += 1;
                        }
                        b')' => {
                            nesting -= 1;
                            index += 1;
                        }
                        _ => index += 1,
                    }
                }
            }
            b'<' if bytes.get(index + 1) == Some(&b'<') => {
                depth += 1;
                index += 2;
            }
            b'<' => {
                index += 1;
                while index < bytes.len() && bytes[index] != b'>' {
                    index += 1;
                }
                index = (index + 1).min(bytes.len());
            }
            b'>' if bytes.get(index + 1) == Some(&b'>') => {
                depth -= 1;
                index += 2;
                if depth == 0 {
                    while index < bytes.len() && bytes[index].is_ascii_whitespace() {
                        index += 1;
                    }
                    return bytes[index..]
                        .starts_with(b"stream")
                        .then_some(index)
                        .filter(|at| {
                            bytes
                                .get(at + 6)
                                .is_some_and(|byte| matches!(byte, b'\n' | b'\r' | b' ' | b'\t'))
                        });
                }
            }
            _ => index += 1,
        }
    }
    None
}

fn find_pdf_keyword(bytes: &[u8], keyword: &[u8]) -> Option<usize> {
    let mut index = 0usize;
    while index + keyword.len() <= bytes.len() {
        match bytes[index] {
            b'%' => {
                while index < bytes.len() && !matches!(bytes[index], b'\n' | b'\r') {
                    index += 1;
                }
            }
            b'(' => {
                index += 1;
                let mut nesting = 1usize;
                while index < bytes.len() && nesting != 0 {
                    match bytes[index] {
                        b'\\' => index = (index + 2).min(bytes.len()),
                        b'(' => {
                            nesting += 1;
                            index += 1;
                        }
                        b')' => {
                            nesting -= 1;
                            index += 1;
                        }
                        _ => index += 1,
                    }
                }
            }
            b'<' if bytes.get(index + 1) == Some(&b'<') => index += 2,
            b'<' => {
                index += 1;
                while index < bytes.len() && bytes[index] != b'>' {
                    index += 1;
                }
                index = (index + 1).min(bytes.len());
            }
            _ if bytes[index..].starts_with(keyword)
                && (index == 0 || is_pdf_delimiter(bytes[index - 1]))
                && bytes.get(index.wrapping_sub(1)) != Some(&b'/')
                && bytes
                    .get(index + keyword.len())
                    .is_none_or(|byte| is_pdf_delimiter(*byte)) =>
            {
                return Some(index);
            }
            _ => index += 1,
        }
    }
    None
}

fn trailer_contains(trailer: &[u8], key: &[u8]) -> bool {
    trailer_contains_dictionary(trailer, key)
}

fn trailer_contains_dictionary(dictionary: &[u8], key: &[u8]) -> bool {
    find_dictionary_key(dictionary, key).is_some()
}

fn trailer_dictionary(section: &[u8]) -> Option<&[u8]> {
    let trailer = find(section, b"trailer")? + 7;
    let tail = &section[trailer..];
    let start = find(tail, b"<<")?;
    let mut depth = 0usize;
    let mut index = start;
    while index + 1 < tail.len() {
        if tail[index..].starts_with(b"<<") {
            depth = depth.checked_add(1)?;
            index += 2;
        } else if tail[index..].starts_with(b">>") {
            depth = depth.checked_sub(1)?;
            index += 2;
            if depth == 0 {
                return Some(&tail[start..index]);
            }
        } else {
            index += 1;
        }
    }
    None
}

fn references(bytes: &[u8]) -> Vec<(u32, u16)> {
    let mut output = Vec::new();
    let mut index = 0;
    while index < bytes.len() {
        match bytes[index] {
            b'%' => {
                while index < bytes.len() && !matches!(bytes[index], b'\n' | b'\r') {
                    index += 1;
                }
                continue;
            }
            b'(' => {
                index += 1;
                let mut nesting = 1usize;
                while index < bytes.len() && nesting != 0 {
                    match bytes[index] {
                        b'\\' => index = (index + 2).min(bytes.len()),
                        b'(' => {
                            nesting += 1;
                            index += 1;
                        }
                        b')' => {
                            nesting -= 1;
                            index += 1;
                        }
                        _ => index += 1,
                    }
                }
                continue;
            }
            b'<' if bytes.get(index + 1) == Some(&b'<') => {
                index += 2;
                continue;
            }
            b'<' if bytes.get(index + 1) != Some(&b'<') => {
                index += 1;
                while index < bytes.len() && bytes[index] != b'>' {
                    index += 1;
                }
                index = (index + 1).min(bytes.len());
                continue;
            }
            _ => {}
        }
        if !bytes[index].is_ascii_digit() || (index > 0 && is_token(bytes[index - 1])) {
            index += 1;
            continue;
        }
        let first_start = index;
        while index < bytes.len() && bytes[index].is_ascii_digit() {
            index += 1;
        }
        let first_end = index;
        skip_space(bytes, &mut index);
        let second_start = index;
        while index < bytes.len() && bytes[index].is_ascii_digit() {
            index += 1;
        }
        if second_start == index {
            continue;
        }
        let second_end = index;
        skip_space(bytes, &mut index);
        if bytes.get(index) == Some(&b'R') && bytes.get(index + 1).is_none_or(|b| !is_token(*b)) {
            if let (Ok(object), Ok(generation)) = (
                parse_usize(&bytes[first_start..first_end]),
                parse_usize(&bytes[second_start..second_end]),
            ) && let (Ok(object), Ok(generation)) =
                (u32::try_from(object), u16::try_from(generation))
            {
                output.push((object, generation));
            }
            index += 1;
        }
    }
    output
}

fn filter_names(dict: &[u8]) -> Vec<&[u8]> {
    let Some(at) = find_dictionary_key(dict, b"Filter") else {
        return vec![];
    };
    let part = trim(&dict[at..]);
    let end = part
        .iter()
        .position(|b| *b == b']' || b.is_ascii_whitespace())
        .unwrap_or(part.len());
    if part.first() == Some(&b'[') {
        let inside = &part[1..];
        let close = inside
            .iter()
            .position(|b| *b == b']')
            .unwrap_or(inside.len());
        inside[..close]
            .split(|b| *b == b'/')
            .filter(|v| !v.is_empty())
            .map(|v| v.split(|b| b.is_ascii_whitespace()).next().unwrap_or(v))
            .collect()
    } else {
        let _ = end;
        vec![
            part.strip_prefix(b"/")
                .unwrap_or(part)
                .split(|b| b.is_ascii_whitespace() || *b == b'>')
                .next()
                .unwrap_or(part),
        ]
    }
}

fn dictionary_integer(bytes: &[u8], key: &[u8]) -> Option<usize> {
    direct_integer(&bytes[find_dictionary_key(bytes, key)?..])
}
fn integer_array(bytes: &[u8], key: &[u8]) -> Option<Vec<usize>> {
    let at = find_dictionary_key(bytes, key)?;
    let rest = trim(&bytes[at..]);
    let inner = rest.strip_prefix(b"[")?.split(|b| *b == b']').next()?;
    Some(numbers(inner))
}
fn find_dictionary_key(bytes: &[u8], key: &[u8]) -> Option<usize> {
    if !bytes.starts_with(b"<<") {
        return None;
    }
    let mut index = 2usize;
    let mut dictionaries = 1usize;
    let mut arrays = 0usize;
    while index < bytes.len() {
        match bytes[index] {
            b'%' => {
                while index < bytes.len() && !matches!(bytes[index], b'\n' | b'\r') {
                    index += 1;
                }
            }
            b'(' => {
                index += 1;
                let mut nesting = 1usize;
                while index < bytes.len() && nesting != 0 {
                    match bytes[index] {
                        b'\\' => index = (index + 2).min(bytes.len()),
                        b'(' => {
                            nesting += 1;
                            index += 1;
                        }
                        b')' => {
                            nesting -= 1;
                            index += 1;
                        }
                        _ => index += 1,
                    }
                }
            }
            b'<' if bytes.get(index + 1) == Some(&b'<') => {
                dictionaries += 1;
                index += 2;
            }
            b'<' => {
                index += 1;
                while index < bytes.len() && bytes[index] != b'>' {
                    index += 1;
                }
                index = (index + 1).min(bytes.len());
            }
            b'>' if bytes.get(index + 1) == Some(&b'>') => {
                dictionaries = dictionaries.saturating_sub(1);
                index += 2;
                if dictionaries == 0 {
                    break;
                }
            }
            b'[' => {
                arrays += 1;
                index += 1;
            }
            b']' => {
                arrays = arrays.saturating_sub(1);
                index += 1;
            }
            b'/' => {
                let start = index + 1;
                index = start;
                while index < bytes.len() && !is_pdf_delimiter(bytes[index]) {
                    index += 1;
                }
                if dictionaries == 1 && arrays == 0 && &bytes[start..index] == key {
                    return Some(index);
                }
            }
            _ => index += 1,
        }
    }
    None
}
fn numbers(bytes: &[u8]) -> Vec<usize> {
    let mut result = Vec::new();
    let mut current = Vec::new();
    for byte in bytes.iter().copied().chain(std::iter::once(b' ')) {
        if byte.is_ascii_digit() {
            current.push(byte);
        } else if !current.is_empty() {
            if let Ok(s) = std::str::from_utf8(&current)
                && let Ok(n) = s.parse()
            {
                result.push(n);
            }
            current.clear();
        }
    }
    result
}
fn direct_integer(bytes: &[u8]) -> Option<usize> {
    let mut index = 0;
    skip_space(bytes, &mut index);
    let start = index;
    while index < bytes.len() && bytes[index].is_ascii_digit() {
        index += 1;
    }
    if index == start {
        return None;
    }
    let value = std::str::from_utf8(&bytes[start..index])
        .ok()?
        .parse()
        .ok()?;
    let mut after = index;
    skip_space(bytes, &mut after);
    if bytes
        .get(after)
        .is_some_and(|byte| byte.is_ascii_digit() || *byte == b'R')
    {
        return None;
    }
    Some(value)
}
fn find(bytes: &[u8], needle: &[u8]) -> Option<usize> {
    bytes.windows(needle.len()).position(|w| w == needle)
}
fn find_last_tail(bytes: &[u8], needle: &[u8], max_tail: usize) -> Option<usize> {
    let start = bytes.len().saturating_sub(max_tail);
    bytes[start..]
        .windows(needle.len())
        .rposition(|window| window == needle)
        .map(|offset| start + offset)
}
fn trim(bytes: &[u8]) -> &[u8] {
    let start = bytes
        .iter()
        .position(|b| !b.is_ascii_whitespace())
        .unwrap_or(bytes.len());
    let end = bytes
        .iter()
        .rposition(|b| !b.is_ascii_whitespace())
        .map_or(start, |i| i + 1);
    &bytes[start..end]
}
fn read_be(bytes: &[u8]) -> Option<usize> {
    bytes.iter().try_fold(0usize, |value, byte| {
        value.checked_mul(256)?.checked_add(*byte as usize)
    })
}
fn parse_usize(bytes: &[u8]) -> Result<usize, PdfReadError> {
    std::str::from_utf8(bytes)
        .ok()
        .and_then(|s| s.trim().parse().ok())
        .ok_or(PdfReadError::Corrupted)
}
fn skip_space(bytes: &[u8], index: &mut usize) {
    while *index < bytes.len() && bytes[*index].is_ascii_whitespace() {
        *index += 1;
    }
}
fn is_token(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'.' | b'+' | b'-')
}
fn is_pdf_delimiter(byte: u8) -> bool {
    byte.is_ascii_whitespace()
        || matches!(
            byte,
            b'(' | b')' | b'<' | b'>' | b'[' | b']' | b'{' | b'}' | b'/' | b'%'
        )
}
