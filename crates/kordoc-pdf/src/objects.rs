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

#[derive(Clone, Copy)]
enum FilterSpec<'a> {
    None,
    Single(&'a [u8]),
    Array,
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
        if trailer_contains(trailer, b"Encrypt") {
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
        let mut header_values = UnsignedIntIter::new(header);
        let mut member_id = None;
        let mut member_offset = None;
        for _ in 0..=index {
            member_id = Some(header_values.next_value()?.ok_or(PdfReadError::Corrupted)?);
            member_offset = Some(header_values.next_value()?.ok_or(PdfReadError::Corrupted)?);
        }
        if member_id != Some(usize::try_from(id.0).map_err(|_| PdfReadError::Corrupted)?) {
            return Err(PdfReadError::Corrupted);
        }
        let start_offset = member_offset.ok_or(PdfReadError::Corrupted)?;
        let end_offset = if index.checked_add(1).is_some_and(|next| next < count) {
            let _next_id = header_values.next_value()?.ok_or(PdfReadError::Corrupted)?;
            header_values.next_value()?.ok_or(PdfReadError::Corrupted)?
        } else {
            bytes
                .len()
                .checked_sub(first)
                .ok_or(PdfReadError::Corrupted)?
        };
        if end_offset < start_offset {
            return Err(PdfReadError::Corrupted);
        }
        let start = first
            .checked_add(start_offset)
            .ok_or(PdfReadError::Corrupted)?;
        let end = first
            .checked_add(end_offset)
            .ok_or(PdfReadError::Corrupted)?;
        let slice = bytes.get(start..end).ok_or(PdfReadError::Corrupted)?;
        let member = trim(slice);
        self.budget.charge_decoded(member.len() as u64)?;
        let owned = member.to_vec();
        self.budget.finish_stream();
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
            for next in references(structured_bytes(value.as_bytes())) {
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
        let output = match filter_spec(body)? {
            FilterSpec::None => crate::stream::copy_unfiltered(encoded, &mut self.budget)?,
            FilterSpec::Array => return Err(PdfReadError::UnsupportedFilter),
            FilterSpec::Single(b"ASCIIHexDecode") | FilterSpec::Single(b"AHx") => {
                crate::stream::decode_ascii_hex(encoded, &mut self.budget)?
            }
            FilterSpec::Single(_) => return Err(PdfReadError::UnsupportedFilter),
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

fn parse_xref<'a>(
    source: &'a [u8],
    budget: &mut PdfBudget,
) -> Result<(HashMap<u32, XrefEntry>, &'a [u8]), PdfReadError> {
    let Some(start) = find_last_tail(source, b"startxref", 64 * 1024) else {
        return Err(PdfReadError::Corrupted);
    };
    let offset = direct_integer(&source[start + 9..]).ok_or(PdfReadError::Corrupted)?;
    let mut visited = HashSet::new();
    let mut entries = HashMap::new();
    let mut trailer = None;
    parse_xref_at(
        source,
        offset,
        budget,
        &mut visited,
        &mut entries,
        &mut trailer,
    )?;
    Ok((entries, trailer.ok_or(PdfReadError::Corrupted)?))
}

fn parse_xref_at<'a>(
    source: &'a [u8],
    offset: usize,
    budget: &mut PdfBudget,
    visited: &mut HashSet<usize>,
    entries: &mut HashMap<u32, XrefEntry>,
    trailer: &mut Option<&'a [u8]>,
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

fn parse_classic_xref<'a>(
    source: &'a [u8],
    offset: usize,
    budget: &mut PdfBudget,
    visited: &mut HashSet<usize>,
    output: &mut HashMap<u32, XrefEntry>,
    trailer: &mut Option<&'a [u8]>,
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
            if trailer.is_none() {
                *trailer = Some(current);
            }
            if let Some(prev) = prev {
                parse_xref_at(source, prev, budget, visited, output, trailer)?;
            }
            break;
        }
        let mut fields = line
            .split(|b| b.is_ascii_whitespace())
            .filter(|field| !field.is_empty());
        let Some(first) = fields.next() else {
            continue;
        };
        let Some(second) = fields.next() else {
            continue;
        };
        let third = fields.next();
        if third.is_none()
            && first.iter().all(u8::is_ascii_digit)
            && second.iter().all(u8::is_ascii_digit)
        {
            object_id = u32::try_from(parse_usize(first)?).map_err(|_| PdfReadError::Corrupted)?;
        } else if let Some(kind) = third.filter(|kind| matches!(*kind, b"n" | b"f")) {
            let file_offset = parse_usize(first)?;
            let generation =
                u16::try_from(parse_usize(second)?).map_err(|_| PdfReadError::Corrupted)?;
            let entry = if kind == b"n" {
                XrefEntry::Normal {
                    offset: file_offset,
                    generation,
                }
            } else {
                XrefEntry::Free
            };
            if !output.contains_key(&object_id) {
                budget.charge_object()?;
            }
            output.entry(object_id).or_insert(entry);
            object_id = object_id.checked_add(1).ok_or(PdfReadError::Corrupted)?;
        }
    }
    Ok(())
}

fn parse_xref_stream_at<'a>(
    source: &'a [u8],
    offset: usize,
    budget: &mut PdfBudget,
    visited: &mut HashSet<usize>,
    output: &mut HashMap<u32, XrefEntry>,
    trailer: &mut Option<&'a [u8]>,
) -> Result<(), PdfReadError> {
    let raw = parse_raw_at_any_id(source, offset)?;
    let body = &source[raw.start..raw.end];
    if find(body, b"/Type /XRef").is_none() {
        return Err(PdfReadError::Corrupted);
    }
    let stream_marker = find_pdf_keyword(body, b"stream").ok_or(PdfReadError::Corrupted)?;
    let dictionary = &body[..stream_marker];
    let prev = dictionary_integer(dictionary, b"Prev");
    if trailer.is_none() {
        *trailer = Some(dictionary);
    }
    if !matches!(filter_spec(dictionary)?, FilterSpec::None) {
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
    let widths = fixed_integer_array::<3>(body, b"W").ok_or(PdfReadError::Corrupted)?;
    let index = if find_dictionary_key(body, b"Index").is_some() {
        Some(dictionary_array(body, b"Index").ok_or(PdfReadError::Corrupted)?)
    } else {
        None
    };
    let default_size = dictionary_integer(body, b"Size").ok_or(PdfReadError::Corrupted)?;
    let stride = widths
        .iter()
        .try_fold(0usize, |sum, width| sum.checked_add(*width))
        .ok_or(PdfReadError::Corrupted)?;
    if stride == 0 || stride > 16 || widths.iter().any(|width| *width > 8) {
        return Err(PdfReadError::Corrupted);
    }
    let mut ranges = IndexRangeIter::new(index, default_size);
    let mut records = 0usize;
    let mut previous_end = 0usize;
    while let Some((start, count)) = ranges.next_range()? {
        let end = start.checked_add(count).ok_or(PdfReadError::Corrupted)?;
        if start < previous_end || u32::try_from(end).is_err() {
            return Err(PdfReadError::Corrupted);
        }
        previous_end = end;
        records = records.checked_add(count).ok_or(PdfReadError::Corrupted)?;
    }
    if records
        .checked_mul(stride)
        .is_none_or(|required| required > data.len())
    {
        return Err(PdfReadError::Corrupted);
    }
    let mut required_objects = 0u64;
    let mut preflight_cursor = 0usize;
    let mut ranges = IndexRangeIter::new(index, default_size);
    while let Some((start, count)) = ranges.next_range()? {
        let range_end = start.checked_add(count).ok_or(PdfReadError::Corrupted)?;
        for object in start..range_end {
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
            if !output.contains_key(&object_id) {
                required_objects = required_objects
                    .checked_add(1)
                    .ok_or(PdfReadError::Corrupted)?;
            }
            preflight_cursor = record_end;
        }
    }
    budget.charge_objects(required_objects)?;
    let mut byte_cursor = 0usize;
    let mut ranges = IndexRangeIter::new(index, default_size);
    while let Some((start, count)) = ranges.next_range()? {
        let range_end = start.checked_add(count).ok_or(PdfReadError::Corrupted)?;
        for object in start..range_end {
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
    let trailer = find_pdf_keyword(section, b"trailer")? + 7;
    let tail = &section[trailer..];
    let mut start = 0usize;
    loop {
        while tail.get(start).is_some_and(u8::is_ascii_whitespace) {
            start += 1;
        }
        if tail.get(start) != Some(&b'%') {
            break;
        }
        while start < tail.len() && !matches!(tail[start], b'\n' | b'\r') {
            start += 1;
        }
    }
    if tail.get(start..start.checked_add(2)?) != Some(b"<<") {
        return None;
    }
    let end = dictionary_end(tail, start)?;
    Some(&tail[start..end])
}

fn dictionary_end(bytes: &[u8], start: usize) -> Option<usize> {
    if bytes.get(start..start.checked_add(2)?) != Some(b"<<") {
        return None;
    }
    let mut depth = 1usize;
    let mut index = start + 2;
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
                depth = depth.checked_add(1)?;
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
                depth = depth.checked_sub(1)?;
                index += 2;
                if depth == 0 {
                    return Some(index);
                }
            }
            _ => index += 1,
        }
    }
    None
}

fn structured_bytes(bytes: &[u8]) -> &[u8] {
    stream_marker_after_dictionary(bytes).map_or(bytes, |marker| &bytes[..marker])
}

struct ReferenceIter<'a> {
    bytes: &'a [u8],
    index: usize,
}

fn references(bytes: &[u8]) -> ReferenceIter<'_> {
    ReferenceIter { bytes, index: 0 }
}

impl Iterator for ReferenceIter<'_> {
    type Item = (u32, u16);

    fn next(&mut self) -> Option<Self::Item> {
        let bytes = self.bytes;
        while self.index < bytes.len() {
            match bytes[self.index] {
                b'%' => {
                    while self.index < bytes.len() && !matches!(bytes[self.index], b'\n' | b'\r') {
                        self.index += 1;
                    }
                    continue;
                }
                b'(' => {
                    self.index += 1;
                    let mut nesting = 1usize;
                    while self.index < bytes.len() && nesting != 0 {
                        match bytes[self.index] {
                            b'\\' => self.index = (self.index + 2).min(bytes.len()),
                            b'(' => {
                                nesting += 1;
                                self.index += 1;
                            }
                            b')' => {
                                nesting -= 1;
                                self.index += 1;
                            }
                            _ => self.index += 1,
                        }
                    }
                    continue;
                }
                b'<' if bytes.get(self.index + 1) == Some(&b'<') => {
                    self.index += 2;
                    continue;
                }
                b'<' => {
                    self.index += 1;
                    while self.index < bytes.len() && bytes[self.index] != b'>' {
                        self.index += 1;
                    }
                    self.index = (self.index + 1).min(bytes.len());
                    continue;
                }
                _ => {}
            }
            if !bytes[self.index].is_ascii_digit()
                || (self.index > 0 && is_token(bytes[self.index - 1]))
            {
                self.index += 1;
                continue;
            }
            let first_start = self.index;
            while self.index < bytes.len() && bytes[self.index].is_ascii_digit() {
                self.index += 1;
            }
            let first_end = self.index;
            skip_space(bytes, &mut self.index);
            let second_start = self.index;
            while self.index < bytes.len() && bytes[self.index].is_ascii_digit() {
                self.index += 1;
            }
            if second_start == self.index {
                continue;
            }
            let second_end = self.index;
            skip_space(bytes, &mut self.index);
            if bytes.get(self.index) != Some(&b'R')
                || bytes
                    .get(self.index + 1)
                    .is_some_and(|byte| is_token(*byte))
            {
                continue;
            }
            self.index += 1;
            if let (Ok(object), Ok(generation)) = (
                parse_usize(&bytes[first_start..first_end]),
                parse_usize(&bytes[second_start..second_end]),
            ) && let (Ok(object), Ok(generation)) =
                (u32::try_from(object), u16::try_from(generation))
            {
                return Some((object, generation));
            }
        }
        None
    }
}

fn filter_spec(dict: &[u8]) -> Result<FilterSpec<'_>, PdfReadError> {
    let Some(at) = find_dictionary_key(dict, b"Filter") else {
        return Ok(FilterSpec::None);
    };
    let part = trim(&dict[at..]);
    if part.first() == Some(&b'[') {
        return Ok(FilterSpec::Array);
    }
    let name = part.strip_prefix(b"/").ok_or(PdfReadError::Corrupted)?;
    let end = name
        .iter()
        .position(|byte| is_pdf_delimiter(*byte))
        .unwrap_or(name.len());
    if end == 0 {
        return Err(PdfReadError::Corrupted);
    }
    Ok(FilterSpec::Single(&name[..end]))
}

fn dictionary_integer(bytes: &[u8], key: &[u8]) -> Option<usize> {
    direct_integer(&bytes[find_dictionary_key(bytes, key)?..])
}
fn dictionary_array<'a>(bytes: &'a [u8], key: &[u8]) -> Option<&'a [u8]> {
    let at = find_dictionary_key(bytes, key)?;
    let rest = trim(&bytes[at..]).strip_prefix(b"[")?;
    Some(&rest[..rest.iter().position(|byte| *byte == b']')?])
}

#[derive(Clone)]
struct UnsignedIntIter<'a> {
    bytes: &'a [u8],
    index: usize,
}

impl<'a> UnsignedIntIter<'a> {
    fn new(bytes: &'a [u8]) -> Self {
        Self { bytes, index: 0 }
    }

    fn next_value(&mut self) -> Result<Option<usize>, PdfReadError> {
        skip_space(self.bytes, &mut self.index);
        if self.index == self.bytes.len() {
            return Ok(None);
        }
        let start = self.index;
        while self.index < self.bytes.len() && self.bytes[self.index].is_ascii_digit() {
            self.index += 1;
        }
        if self.index == start
            || self
                .bytes
                .get(self.index)
                .is_some_and(|byte| !byte.is_ascii_whitespace())
        {
            return Err(PdfReadError::Corrupted);
        }
        Ok(Some(parse_usize(&self.bytes[start..self.index])?))
    }
}

struct IndexRangeIter<'a> {
    explicit: Option<UnsignedIntIter<'a>>,
    default_size: Option<usize>,
}

impl<'a> IndexRangeIter<'a> {
    fn new(array: Option<&'a [u8]>, default_size: usize) -> Self {
        Self {
            explicit: array.map(UnsignedIntIter::new),
            default_size: array.is_none().then_some(default_size),
        }
    }

    fn next_range(&mut self) -> Result<Option<(usize, usize)>, PdfReadError> {
        if let Some(iter) = &mut self.explicit {
            let Some(start) = iter.next_value()? else {
                return Ok(None);
            };
            let count = iter.next_value()?.ok_or(PdfReadError::Corrupted)?;
            return Ok(Some((start, count)));
        }
        Ok(self.default_size.take().map(|size| (0, size)))
    }
}

fn fixed_integer_array<const N: usize>(bytes: &[u8], key: &[u8]) -> Option<[usize; N]> {
    let mut values = UnsignedIntIter::new(dictionary_array(bytes, key)?);
    let mut output = [0; N];
    for value in &mut output {
        *value = values.next_value().ok()??;
    }
    values.next_value().ok()?.is_none().then_some(output)
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
