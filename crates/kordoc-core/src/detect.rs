use std::io::Cursor;

use kordoc_ir::{ErrorCode, FileType, KordocError, ParseSuccess};

use crate::limits::{MAX_ARCHIVE_ENTRIES, MAX_UNCOMPRESSED_BYTES, validate_input_len};
use crate::parse::{ParserRegistry, assemble_success, try_parse_with_registry};
use kordoc_ir::ParseOptions;

const EOCD_SIGNATURE: u32 = 0x0605_4b50;
const ZIP64_EOCD_SIGNATURE: u32 = 0x0606_4b50;
const ZIP64_LOCATOR_SIGNATURE: u32 = 0x0706_4b50;
const CENTRAL_SIGNATURE: u32 = 0x0201_4b50;
const EOCD_FIXED_LEN: usize = 22;
const ZIP64_LOCATOR_LEN: usize = 20;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ZipPreflight {
    pub(crate) entry_count: u64,
    pub(crate) central_span: u64,
    pub(crate) total_uncompressed: u64,
}

fn zip_bomb(reason: &'static str) -> KordocError {
    KordocError::new(ErrorCode::ZipBomb, reason)
}

fn read_u16(bytes: &[u8], offset: usize) -> Option<u16> {
    Some(u16::from_le_bytes(
        bytes.get(offset..offset.checked_add(2)?)?.try_into().ok()?,
    ))
}

fn read_u32(bytes: &[u8], offset: usize) -> Option<u32> {
    Some(u32::from_le_bytes(
        bytes.get(offset..offset.checked_add(4)?)?.try_into().ok()?,
    ))
}

fn read_u64(bytes: &[u8], offset: usize) -> Option<u64> {
    Some(u64::from_le_bytes(
        bytes.get(offset..offset.checked_add(8)?)?.try_into().ok()?,
    ))
}

fn find_eocd(bytes: &[u8]) -> Result<usize, KordocError> {
    let search_start = bytes
        .len()
        .saturating_sub(EOCD_FIXED_LEN + u16::MAX as usize);
    let mut candidate = None;
    for offset in (search_start..=bytes.len().saturating_sub(EOCD_FIXED_LEN)).rev() {
        if read_u32(bytes, offset) != Some(EOCD_SIGNATURE) {
            continue;
        }
        candidate = Some(offset);
        let comment_len = read_u16(bytes, offset + 20)
            .ok_or_else(|| zip_bomb("Truncated ZIP end record"))?
            as usize;
        if offset
            .checked_add(EOCD_FIXED_LEN)
            .and_then(|end| end.checked_add(comment_len))
            == Some(bytes.len())
        {
            return Ok(offset);
        }
    }
    Err(if candidate.is_some() {
        zip_bomb("Malformed ZIP end record comment length")
    } else {
        zip_bomb("ZIP end record is missing")
    })
}

#[derive(Clone, Copy)]
struct DirectoryInfo {
    entries: u64,
    size: u64,
    offset: u64,
}

fn directory_info(bytes: &[u8], eocd: usize) -> Result<DirectoryInfo, KordocError> {
    let disk = read_u16(bytes, eocd + 4).ok_or_else(|| zip_bomb("Truncated ZIP end record"))?;
    let directory_disk =
        read_u16(bytes, eocd + 6).ok_or_else(|| zip_bomb("Truncated ZIP end record"))?;
    let disk_entries =
        read_u16(bytes, eocd + 8).ok_or_else(|| zip_bomb("Truncated ZIP end record"))?;
    let entries = read_u16(bytes, eocd + 10).ok_or_else(|| zip_bomb("Truncated ZIP end record"))?;
    let size = read_u32(bytes, eocd + 12).ok_or_else(|| zip_bomb("Truncated ZIP end record"))?;
    let offset = read_u32(bytes, eocd + 16).ok_or_else(|| zip_bomb("Truncated ZIP end record"))?;
    let needs_zip64 = size == u32::MAX || offset == u32::MAX;
    let locator_offset = eocd.checked_sub(ZIP64_LOCATOR_LEN);
    let has_locator = locator_offset.and_then(|position| read_u32(bytes, position))
        == Some(ZIP64_LOCATOR_SIGNATURE);

    if disk != 0 || directory_disk != 0 || disk_entries != entries {
        return Err(zip_bomb("Multi-disk ZIP archives are not supported"));
    }

    let (entries, size, offset, end_record_offset) = if needs_zip64 || has_locator {
        let locator = locator_offset
            .filter(|position| read_u32(bytes, *position) == Some(ZIP64_LOCATOR_SIGNATURE))
            .ok_or_else(|| zip_bomb("ZIP64 locator is missing or malformed"))?;
        let locator_disk =
            read_u32(bytes, locator + 4).ok_or_else(|| zip_bomb("Truncated ZIP64 locator"))?;
        let zip64_offset =
            read_u64(bytes, locator + 8).ok_or_else(|| zip_bomb("Truncated ZIP64 locator"))?;
        let disks =
            read_u32(bytes, locator + 16).ok_or_else(|| zip_bomb("Truncated ZIP64 locator"))?;
        let zip64_offset =
            usize::try_from(zip64_offset).map_err(|_| zip_bomb("ZIP64 record offset overflows"))?;
        if locator_disk != 0
            || disks != 1
            || zip64_offset.checked_add(56).is_none_or(|end| end > locator)
        {
            return Err(zip_bomb("Invalid multi-disk or out-of-bounds ZIP64 record"));
        }
        if read_u32(bytes, zip64_offset) != Some(ZIP64_EOCD_SIGNATURE) {
            return Err(zip_bomb("ZIP64 end record signature is invalid"));
        }
        let record_size = read_u64(bytes, zip64_offset + 4)
            .ok_or_else(|| zip_bomb("Truncated ZIP64 end record"))?;
        let record_end = (zip64_offset as u64)
            .checked_add(12)
            .and_then(|start| start.checked_add(record_size))
            .ok_or_else(|| zip_bomb("ZIP64 end record size overflows"))?;
        if record_size < 44 || record_end != locator as u64 {
            return Err(zip_bomb("ZIP64 end record span is invalid"));
        }
        let disk64 = read_u32(bytes, zip64_offset + 16)
            .ok_or_else(|| zip_bomb("Truncated ZIP64 end record"))?;
        let directory_disk64 = read_u32(bytes, zip64_offset + 20)
            .ok_or_else(|| zip_bomb("Truncated ZIP64 end record"))?;
        let disk_entries64 = read_u64(bytes, zip64_offset + 24)
            .ok_or_else(|| zip_bomb("Truncated ZIP64 end record"))?;
        let entries64 = read_u64(bytes, zip64_offset + 32)
            .ok_or_else(|| zip_bomb("Truncated ZIP64 end record"))?;
        let size64 = read_u64(bytes, zip64_offset + 40)
            .ok_or_else(|| zip_bomb("Truncated ZIP64 end record"))?;
        let offset64 = read_u64(bytes, zip64_offset + 48)
            .ok_or_else(|| zip_bomb("Truncated ZIP64 end record"))?;
        if disk64 != 0 || directory_disk64 != 0 || disk_entries64 != entries64 {
            return Err(zip_bomb("Multi-disk ZIP64 archives are not supported"));
        }
        if (disk_entries != u16::MAX && disk_entries as u64 != disk_entries64)
            || (entries != u16::MAX && entries as u64 != entries64)
            || (size != u32::MAX && size as u64 != size64)
            || (offset != u32::MAX && offset as u64 != offset64)
        {
            return Err(zip_bomb("Classic ZIP and ZIP64 directory values disagree"));
        }
        (entries64, size64, offset64, zip64_offset)
    } else {
        (entries as u64, size as u64, offset as u64, eocd)
    };

    if entries > MAX_ARCHIVE_ENTRIES {
        return Err(zip_bomb("ZIP entry count exceeds the configured limit"));
    }
    let offset_usize =
        usize::try_from(offset).map_err(|_| zip_bomb("ZIP central directory offset overflows"))?;
    let size_usize =
        usize::try_from(size).map_err(|_| zip_bomb("ZIP central directory size overflows"))?;
    let end = offset_usize
        .checked_add(size_usize)
        .ok_or_else(|| zip_bomb("ZIP central directory span overflows"))?;
    if end != end_record_offset || end > bytes.len() {
        return Err(zip_bomb(
            "ZIP central directory is outside its declared span",
        ));
    }
    Ok(DirectoryInfo {
        entries,
        size,
        offset,
    })
}

fn zip64_entry_sizes(
    extra: &[u8],
    uncomp32: u32,
    comp32: u32,
    offset32: u32,
    disk16: u16,
) -> Result<(u64, u64, u64), KordocError> {
    let need_uncomp = uncomp32 == u32::MAX;
    let need_comp = comp32 == u32::MAX;
    let need_offset = offset32 == u32::MAX;
    let need_disk = disk16 == u16::MAX;
    let mut cursor = 0usize;
    let mut zip64 = None;
    while cursor < extra.len() {
        let header_end = cursor
            .checked_add(4)
            .ok_or_else(|| zip_bomb("ZIP extra field length overflows"))?;
        if header_end > extra.len() {
            return Err(zip_bomb("Malformed ZIP extra field header"));
        }
        let tag =
            read_u16(extra, cursor).ok_or_else(|| zip_bomb("Malformed ZIP extra field header"))?;
        let len = read_u16(extra, cursor + 2)
            .ok_or_else(|| zip_bomb("Malformed ZIP extra field header"))?
            as usize;
        let value_start = header_end;
        let value_end = value_start
            .checked_add(len)
            .ok_or_else(|| zip_bomb("ZIP extra field length overflows"))?;
        if value_end > extra.len() {
            return Err(zip_bomb("ZIP extra field exceeds declared length"));
        }
        if tag == 0x0001 && zip64.replace(&extra[value_start..value_end]).is_some() {
            return Err(zip_bomb("Duplicate ZIP64 extra field"));
        }
        cursor = value_end;
    }
    let Some(field) = zip64 else {
        if need_uncomp || need_comp || need_offset || need_disk {
            return Err(zip_bomb("Required ZIP64 entry values are missing"));
        }
        return Ok((uncomp32 as u64, comp32 as u64, offset32 as u64));
    };
    let mut cursor = 0usize;
    let mut next_u64 = || -> Result<u64, KordocError> {
        let value =
            read_u64(field, cursor).ok_or_else(|| zip_bomb("Truncated ZIP64 entry values"))?;
        cursor += 8;
        Ok(value)
    };
    let uncomp = if need_uncomp {
        next_u64()?
    } else {
        uncomp32 as u64
    };
    let comp = if need_comp {
        next_u64()?
    } else {
        comp32 as u64
    };
    let offset = if need_offset {
        next_u64()?
    } else {
        offset32 as u64
    };
    if need_disk {
        if read_u32(field, cursor).is_none() {
            return Err(zip_bomb("Truncated ZIP64 disk number"));
        }
        if read_u32(field, cursor) != Some(0) {
            return Err(zip_bomb("Multi-disk ZIP entry is not supported"));
        }
        cursor += 4;
    }
    if cursor != field.len() {
        return Err(zip_bomb("ZIP64 entry values have an invalid length"));
    }
    Ok((uncomp, comp, offset))
}

pub(crate) fn preflight_zip(bytes: &[u8]) -> Result<ZipPreflight, KordocError> {
    let eocd = find_eocd(bytes)?;
    let info = directory_info(bytes, eocd)?;
    let start =
        usize::try_from(info.offset).map_err(|_| zip_bomb("ZIP directory offset overflows"))?;
    let end = start
        .checked_add(
            usize::try_from(info.size).map_err(|_| zip_bomb("ZIP directory size overflows"))?,
        )
        .ok_or_else(|| zip_bomb("ZIP central directory span overflows"))?;
    let mut cursor = start;
    let mut total = 0u64;
    for _ in 0..info.entries {
        if read_u32(bytes, cursor) != Some(CENTRAL_SIGNATURE) {
            return Err(zip_bomb("Malformed ZIP central directory entry"));
        }
        let fixed_end = cursor
            .checked_add(46)
            .ok_or_else(|| zip_bomb("ZIP entry offset overflows"))?;
        if fixed_end > end {
            return Err(zip_bomb("Truncated ZIP central directory entry"));
        }
        let name_len = read_u16(bytes, cursor + 28)
            .ok_or_else(|| zip_bomb("Truncated ZIP central entry"))?
            as usize;
        let extra_len = read_u16(bytes, cursor + 30)
            .ok_or_else(|| zip_bomb("Truncated ZIP central entry"))?
            as usize;
        let comment_len = read_u16(bytes, cursor + 32)
            .ok_or_else(|| zip_bomb("Truncated ZIP central entry"))?
            as usize;
        let disk =
            read_u16(bytes, cursor + 34).ok_or_else(|| zip_bomb("Truncated ZIP central entry"))?;
        let offset32 =
            read_u32(bytes, cursor + 42).ok_or_else(|| zip_bomb("Truncated ZIP central entry"))?;
        if disk != 0 && disk != u16::MAX {
            return Err(zip_bomb("Multi-disk ZIP entry is not supported"));
        }
        let record_end = fixed_end
            .checked_add(name_len)
            .and_then(|value| value.checked_add(extra_len))
            .and_then(|value| value.checked_add(comment_len))
            .ok_or_else(|| zip_bomb("ZIP entry name/extra/comment span overflows"))?;
        if record_end > end {
            return Err(zip_bomb(
                "ZIP entry name/extra/comment exceeds the central directory",
            ));
        }
        let extra_start = fixed_end
            .checked_add(name_len)
            .ok_or_else(|| zip_bomb("ZIP entry extra offset overflows"))?;
        let extra_end = extra_start
            .checked_add(extra_len)
            .ok_or_else(|| zip_bomb("ZIP entry extra span overflows"))?;
        let uncomp32 =
            read_u32(bytes, cursor + 24).ok_or_else(|| zip_bomb("Truncated ZIP central entry"))?;
        let comp32 =
            read_u32(bytes, cursor + 20).ok_or_else(|| zip_bomb("Truncated ZIP central entry"))?;
        let (uncompressed, compressed, local_offset) = zip64_entry_sizes(
            &bytes[extra_start..extra_end],
            uncomp32,
            comp32,
            offset32,
            disk,
        )?;
        let local_offset = usize::try_from(local_offset)
            .map_err(|_| zip_bomb("ZIP local header offset overflows"))?;
        let local_fixed_end = local_offset
            .checked_add(30)
            .ok_or_else(|| zip_bomb("ZIP local header span overflows"))?;
        if local_fixed_end > start || read_u32(bytes, local_offset) != Some(0x0403_4b50) {
            return Err(zip_bomb("ZIP local header is outside the data area"));
        }
        let local_name_len = read_u16(bytes, local_offset + 26)
            .ok_or_else(|| zip_bomb("Truncated ZIP local header"))?
            as usize;
        let local_extra_len = read_u16(bytes, local_offset + 28)
            .ok_or_else(|| zip_bomb("Truncated ZIP local header"))?
            as usize;
        let local_data_start = local_fixed_end
            .checked_add(local_name_len)
            .and_then(|position| position.checked_add(local_extra_len))
            .ok_or_else(|| zip_bomb("ZIP local data offset overflows"))?;
        if local_data_start > start
            || local_data_start
                .checked_add(
                    usize::try_from(compressed)
                        .map_err(|_| zip_bomb("ZIP compressed size overflows"))?,
                )
                .is_none_or(|local_end| local_end > start)
        {
            return Err(zip_bomb("ZIP compressed data exceeds the data area"));
        }
        total = total
            .checked_add(uncompressed)
            .ok_or_else(|| zip_bomb("ZIP uncompressed size total overflows"))?;
        if total > MAX_UNCOMPRESSED_BYTES {
            return Err(zip_bomb(
                "ZIP uncompressed size exceeds the configured limit",
            ));
        }
        cursor = record_end;
    }
    if cursor != end {
        return Err(zip_bomb(
            "ZIP central directory span does not match its entries",
        ));
    }
    Ok(ZipPreflight {
        entry_count: info.entries,
        central_span: info.size,
        total_uncompressed: total,
    })
}

fn is_zip(bytes: &[u8]) -> bool {
    bytes.starts_with(b"PK\x03\x04") || bytes.starts_with(b"PK\x05\x06")
}

fn refine_zip(bytes: &[u8]) -> Result<FileType, KordocError> {
    preflight_zip(bytes)?;
    let Ok(mut archive) = zip::ZipArchive::new(Cursor::new(bytes)) else {
        return Ok(FileType::Unknown);
    };
    let mut has_xlsx = false;
    let mut has_docx = false;
    let mut has_pptx = false;
    let mut has_hwpx = false;
    for index in 0..archive.len() {
        let Ok(file) = archive.by_index(index) else {
            return Ok(FileType::Unknown);
        };
        let name = file.name();
        has_xlsx |= name == "xl/workbook.xml";
        has_docx |= name == "word/document.xml";
        has_pptx |= name == "ppt/presentation.xml";
        has_hwpx |= name == "Contents/content.hpf" || name == "mimetype" || is_hwpx_section(name);
    }
    Ok(if has_xlsx {
        FileType::Xlsx
    } else if has_docx {
        FileType::Docx
    } else if has_pptx {
        FileType::Pptx
    } else if has_hwpx {
        FileType::Hwpx
    } else {
        FileType::Unknown
    })
}

fn is_hwpx_section(name: &str) -> bool {
    name.starts_with("Contents/")
}

fn refine_ole(bytes: &[u8]) -> FileType {
    let Ok(compound) = cfb::CompoundFile::open(Cursor::new(bytes)) else {
        return FileType::Unknown;
    };
    let mut xls = false;
    let mut hwp = false;
    for entry in compound.walk() {
        if !entry.is_stream() {
            continue;
        }
        let name = entry.name();
        let path = entry.path();
        if name == "Workbook" || name == "Book" {
            xls = true;
        }
        if name == "FileHeader" || name == "DocInfo" {
            hwp = true;
        }
        if name
            .strip_prefix("Section")
            .is_some_and(|tail| !tail.is_empty() && tail.bytes().all(|byte| byte.is_ascii_digit()))
            && path
                .parent()
                .and_then(|parent| parent.file_name())
                .is_some_and(|parent| parent == "BodyText")
        {
            hwp = true;
        }
    }
    if xls {
        FileType::Xls
    } else if hwp {
        FileType::Hwp
    } else {
        FileType::Unknown
    }
}

fn refine_ole_legacy(bytes: &[u8]) -> FileType {
    let Ok(compound) = cfb::CompoundFile::open(Cursor::new(bytes)) else {
        return FileType::Unknown;
    };
    let mut xls = false;
    let mut hwp = false;
    for entry in compound.walk() {
        let name = entry.name();
        xls |= name == "Workbook" || name == "Book";
        hwp |= name == "FileHeader" || name == "DocInfo" || name.starts_with("Section");
    }
    if xls {
        FileType::Xls
    } else if hwp {
        FileType::Hwp
    } else {
        FileType::Unknown
    }
}

pub fn detect_format(bytes: &[u8]) -> Result<FileType, KordocError> {
    validate_input_len(bytes.len())?;
    if bytes.starts_with(b"HWP Document File V3.00") {
        return Ok(FileType::Hwp3);
    }
    if is_zip(bytes) {
        return refine_zip(bytes);
    }
    if bytes.starts_with(b"\xd0\xcf\x11\xe0\xa1\xb1\x1a\xe1") {
        return Ok(refine_ole(bytes));
    }
    if bytes.starts_with(b"%PDF") {
        return Ok(FileType::Pdf);
    }
    let xml_prefix = bytes.get(..512).unwrap_or(bytes);
    let xml_prefix = xml_prefix
        .strip_prefix(&[0xef, 0xbb, 0xbf])
        .unwrap_or(xml_prefix);
    let declaration = xml_prefix
        .iter()
        .position(|byte| !byte.is_ascii_whitespace())
        .is_some_and(|offset| xml_prefix[offset..].starts_with(b"<?xml"));
    if declaration && xml_prefix.windows(6).any(|window| window == b"<HWPML") {
        return Ok(FileType::Hwpml);
    }
    if bytes.starts_with(b"\x89PNG\r\n\x1a\n")
        || bytes.starts_with(b"\xff\xd8\xff")
        || (bytes.starts_with(b"RIFF") && bytes.get(8..12) == Some(b"WEBP"))
    {
        return Ok(FileType::Image);
    }
    Ok(FileType::Unknown)
}

/// Legacy four-byte ZIP signature predicate retained for API compatibility.
pub fn is_zip_file(bytes: &[u8]) -> bool {
    bytes.starts_with(b"PK\x03\x04")
}

/// Legacy HWPX predicate. This is intentionally a ZIP-signature alias.
pub fn is_hwpx_file(bytes: &[u8]) -> bool {
    is_zip_file(bytes)
}

/// Legacy four-byte OLE2 signature predicate retained for compatibility.
pub fn is_old_hwp_file(bytes: &[u8]) -> bool {
    bytes.starts_with(b"\xd0\xcf\x11\xe0")
}

/// Legacy four-byte PDF signature predicate retained for compatibility.
pub fn is_pdf_file(bytes: &[u8]) -> bool {
    bytes.starts_with(b"%PDF")
}

/// Refine an OLE2 container while mapping malformed containers to `unknown`.
pub fn detect_ole2_format(bytes: &[u8]) -> FileType {
    if !is_old_hwp_file(bytes) {
        return FileType::Unknown;
    }
    refine_ole_legacy(bytes)
}

/// Refine a ZIP container while mapping malformed containers to `unknown`.
pub fn detect_zip_format(bytes: &[u8]) -> FileType {
    if !is_zip_file(bytes) {
        return FileType::Unknown;
    }
    refine_zip(bytes).unwrap_or(FileType::Unknown)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseDispatchError {
    pub file_type: FileType,
    pub code: ErrorCode,
    pub message: String,
}

impl std::fmt::Display for ParseDispatchError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", self.code, self.message)
    }
}

impl std::error::Error for ParseDispatchError {}

impl From<KordocError> for ParseDispatchError {
    fn from(error: KordocError) -> Self {
        Self {
            file_type: FileType::Unknown,
            code: error.code,
            message: error.message,
        }
    }
}

pub fn try_parse(bytes: &[u8]) -> Result<ParseSuccess, ParseDispatchError> {
    try_parse_with_options(bytes, &ParseOptions::default())
}

pub fn try_parse_with_options(
    bytes: &[u8],
    options: &ParseOptions,
) -> Result<ParseSuccess, ParseDispatchError> {
    let registry = ParserRegistry::built_in();
    try_parse_with_registry(bytes, &registry, options)
        .and_then(|(file_type, parsed)| assemble_success(file_type, parsed, options))
}

#[cfg(test)]
mod preflight_contract_tests {
    use super::preflight_zip;
    use crate::limits::{MAX_ARCHIVE_ENTRIES, MAX_UNCOMPRESSED_BYTES};
    use kordoc_ir::ErrorCode;
    use proptest::prelude::*;

    fn put_u16(bytes: &mut Vec<u8>, value: u16) {
        bytes.extend_from_slice(&value.to_le_bytes());
    }
    fn put_u32(bytes: &mut Vec<u8>, value: u32) {
        bytes.extend_from_slice(&value.to_le_bytes());
    }
    fn put_u64(bytes: &mut Vec<u8>, value: u64) {
        bytes.extend_from_slice(&value.to_le_bytes());
    }

    fn archive_with(entries: &[u32]) -> Vec<u8> {
        let mut bytes = Vec::new();
        put_u32(&mut bytes, 0x0403_4b50);
        let mut local_fixed = [0u8; 26];
        local_fixed[22..24].copy_from_slice(&1u16.to_le_bytes());
        bytes.extend_from_slice(&local_fixed);
        bytes.push(b'a');
        let central_offset = bytes.len() as u64;
        for size in entries {
            put_u32(&mut bytes, 0x0201_4b50);
            let mut fixed = [0u8; 42];
            fixed[16..20].copy_from_slice(&0u32.to_le_bytes());
            fixed[20..24].copy_from_slice(&size.to_le_bytes());
            fixed[24..26].copy_from_slice(&1u16.to_le_bytes());
            bytes.extend_from_slice(&fixed);
            bytes.push(b'a');
        }
        let central_size = bytes.len() as u64 - central_offset;
        if entries.len() > u16::MAX as usize {
            let zip64_offset = bytes.len() as u64;
            put_u32(&mut bytes, 0x0606_4b50);
            put_u64(&mut bytes, 44);
            put_u16(&mut bytes, 45);
            put_u16(&mut bytes, 45);
            put_u32(&mut bytes, 0);
            put_u32(&mut bytes, 0);
            put_u64(&mut bytes, entries.len() as u64);
            put_u64(&mut bytes, entries.len() as u64);
            put_u64(&mut bytes, central_size);
            put_u64(&mut bytes, central_offset);
            put_u32(&mut bytes, 0x0706_4b50);
            put_u32(&mut bytes, 0);
            put_u64(&mut bytes, zip64_offset);
            put_u32(&mut bytes, 1);
        }
        put_u32(&mut bytes, 0x0605_4b50);
        put_u16(&mut bytes, 0);
        put_u16(&mut bytes, 0);
        put_u16(
            &mut bytes,
            if entries.len() > u16::MAX as usize {
                u16::MAX
            } else {
                entries.len() as u16
            },
        );
        put_u16(
            &mut bytes,
            if entries.len() > u16::MAX as usize {
                u16::MAX
            } else {
                entries.len() as u16
            },
        );
        put_u32(
            &mut bytes,
            if entries.len() > u16::MAX as usize {
                u32::MAX
            } else {
                central_size as u32
            },
        );
        put_u32(&mut bytes, central_offset as u32);
        put_u16(&mut bytes, 0);
        bytes
    }

    fn archive_with_zip64_size(size: u64, local_offset: Option<u64>) -> Vec<u8> {
        let mut bytes = Vec::new();
        put_u32(&mut bytes, 0x0403_4b50);
        let mut local_fixed = [0u8; 26];
        local_fixed[22..24].copy_from_slice(&1u16.to_le_bytes());
        bytes.extend_from_slice(&local_fixed);
        bytes.push(b'a');
        let central_offset = bytes.len() as u64;
        put_u32(&mut bytes, 0x0201_4b50);
        let mut fixed = [0u8; 42];
        fixed[16..20].copy_from_slice(&u32::MAX.to_le_bytes());
        fixed[20..24].copy_from_slice(&u32::MAX.to_le_bytes());
        fixed[24..26].copy_from_slice(&1u16.to_le_bytes());
        fixed[26..28].copy_from_slice(&20u16.to_le_bytes());
        fixed[38..42]
            .copy_from_slice(&local_offset.unwrap_or(0).min(u32::MAX as u64).to_le_bytes()[..4]);
        if local_offset.is_some() {
            fixed[38..42].copy_from_slice(&u32::MAX.to_le_bytes());
        }
        bytes.extend_from_slice(&fixed);
        bytes.push(b'a');
        put_u16(&mut bytes, 1);
        put_u16(&mut bytes, if local_offset.is_some() { 24 } else { 16 });
        put_u64(&mut bytes, size);
        put_u64(&mut bytes, 0);
        if let Some(offset) = local_offset {
            put_u64(&mut bytes, offset);
        }
        let central_size = bytes.len() as u64 - central_offset;
        put_u32(&mut bytes, 0x0605_4b50);
        put_u16(&mut bytes, 0);
        put_u16(&mut bytes, 0);
        put_u16(&mut bytes, 1);
        put_u16(&mut bytes, 1);
        put_u32(&mut bytes, central_size as u32);
        put_u32(&mut bytes, central_offset as u32);
        put_u16(&mut bytes, 0);
        bytes
    }

    fn archive_with_mismatched_zip64_count() -> Vec<u8> {
        let mut bytes = archive_with(&[0]);
        let eocd = bytes.len() - 22;
        let central_size = u32::from_le_bytes(bytes[eocd + 12..eocd + 16].try_into().unwrap());
        let central_offset = u32::from_le_bytes(bytes[eocd + 16..eocd + 20].try_into().unwrap());
        let zip64_offset = eocd as u64;
        let mut zip64 = Vec::new();
        put_u32(&mut zip64, 0x0606_4b50);
        put_u64(&mut zip64, 44);
        put_u16(&mut zip64, 45);
        put_u16(&mut zip64, 45);
        put_u32(&mut zip64, 0);
        put_u32(&mut zip64, 0);
        put_u64(&mut zip64, 1);
        put_u64(&mut zip64, 2);
        put_u64(&mut zip64, central_size as u64);
        put_u64(&mut zip64, central_offset as u64);
        put_u32(&mut zip64, 0x0706_4b50);
        put_u32(&mut zip64, 0);
        put_u64(&mut zip64, zip64_offset);
        put_u32(&mut zip64, 1);
        bytes.splice(eocd..eocd, zip64);
        bytes
    }

    #[test]
    fn accepts_exact_entry_and_uncompressed_limits() {
        let entries = vec![0; MAX_ARCHIVE_ENTRIES as usize];
        let result = preflight_zip(&archive_with(&entries)).unwrap();
        assert_eq!(result.entry_count, MAX_ARCHIVE_ENTRIES);
        assert_eq!(result.total_uncompressed, 0);
        let result = preflight_zip(&archive_with(&[MAX_UNCOMPRESSED_BYTES as u32])).unwrap();
        assert_eq!(result.total_uncompressed, MAX_UNCOMPRESSED_BYTES);
        let classic_limit = vec![0; u16::MAX as usize];
        assert_eq!(
            preflight_zip(&archive_with(&classic_limit))
                .unwrap()
                .entry_count,
            u16::MAX as u64
        );
    }

    #[test]
    fn rejects_entry_and_uncompressed_limits_one_over() {
        let entries = vec![0; MAX_ARCHIVE_ENTRIES as usize + 1];
        assert_eq!(
            preflight_zip(&archive_with(&entries)).unwrap_err().code,
            ErrorCode::ZipBomb
        );
        let result = preflight_zip(&archive_with(&[MAX_UNCOMPRESSED_BYTES as u32, 1]));
        assert_eq!(result.unwrap_err().code, ErrorCode::ZipBomb);
    }

    #[test]
    fn rejects_inconsistent_central_directory_size() {
        let mut archive = archive_with(&[0]);
        let eocd = archive.len() - 22;
        archive[eocd + 12] = 0xff;
        assert_eq!(
            preflight_zip(&archive).unwrap_err().code,
            ErrorCode::ZipBomb
        );
    }

    #[test]
    fn supports_per_entry_zip64_sizes_and_checks_zip64_offsets() {
        let accepted = preflight_zip(&archive_with_zip64_size(7, None)).unwrap();
        assert_eq!(accepted.total_uncompressed, 7);
        assert_eq!(
            preflight_zip(&archive_with_zip64_size(MAX_UNCOMPRESSED_BYTES + 1, None))
                .unwrap_err()
                .code,
            ErrorCode::ZipBomb
        );
        assert_eq!(
            preflight_zip(&archive_with_zip64_size(7, Some(u64::MAX)))
                .unwrap_err()
                .code,
            ErrorCode::ZipBomb
        );
    }

    #[test]
    fn rejects_multidisk_and_malformed_name_extra_comment_lengths() {
        let mut multidisk = archive_with(&[0]);
        let eocd = multidisk.len() - 22;
        multidisk[eocd + 4] = 1;
        assert_eq!(
            preflight_zip(&multidisk).unwrap_err().code,
            ErrorCode::ZipBomb
        );

        let mut bad_name = archive_with(&[0]);
        bad_name[31 + 28..31 + 30].copy_from_slice(&u16::MAX.to_le_bytes());
        assert_eq!(
            preflight_zip(&bad_name).unwrap_err().code,
            ErrorCode::ZipBomb
        );

        let mut bad_extra = archive_with(&[0]);
        bad_extra[31 + 30..31 + 32].copy_from_slice(&u16::MAX.to_le_bytes());
        assert_eq!(
            preflight_zip(&bad_extra).unwrap_err().code,
            ErrorCode::ZipBomb
        );

        let mut bad_comment = archive_with(&[0]);
        bad_comment[31 + 32..31 + 34].copy_from_slice(&u16::MAX.to_le_bytes());
        assert_eq!(
            preflight_zip(&bad_comment).unwrap_err().code,
            ErrorCode::ZipBomb
        );

        let mut bad_compressed_span = archive_with(&[0]);
        bad_compressed_span[31 + 20..31 + 24].copy_from_slice(&1u32.to_le_bytes());
        assert_eq!(
            preflight_zip(&bad_compressed_span).unwrap_err().code,
            ErrorCode::ZipBomb
        );
        assert_eq!(
            preflight_zip(&archive_with_mismatched_zip64_count())
                .unwrap_err()
                .code,
            ErrorCode::ZipBomb
        );

        let mut missing_locator = archive_with(&vec![0; u16::MAX as usize + 1]);
        let locator = missing_locator.len() - 22 - 20;
        missing_locator[locator..locator + 4].copy_from_slice(&0u32.to_le_bytes());
        assert_eq!(
            preflight_zip(&missing_locator).unwrap_err().code,
            ErrorCode::ZipBomb
        );
    }

    proptest::proptest! {
        #[test]
        fn successful_preflight_obeys_declared_limits(sizes in proptest::collection::vec(0u32..1024, 0..64)) {
            let archive = archive_with(&sizes);
            let result = preflight_zip(&archive).unwrap();
            prop_assert!(result.entry_count <= MAX_ARCHIVE_ENTRIES);
            prop_assert!(result.central_span <= archive.len() as u64);
            prop_assert!(result.total_uncompressed <= MAX_UNCOMPRESSED_BYTES);
            prop_assert_eq!(result.entry_count, sizes.len() as u64);
        }
    }
}
