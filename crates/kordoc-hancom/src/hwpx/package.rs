//! Validated, bounded access to HWPX ZIP package members.

// The manager wires this private substrate into consumers at the H3 integration join.
#![allow(
    dead_code,
    reason = "H1a package substrate remains private until the H3 integration join"
)]

use kordoc_ir::{ErrorCode, KordocError, ParseWarning, WarningCode};
use quick_xml::Reader;
use quick_xml::events::{BytesStart, Event};
use std::collections::{HashMap, HashSet};
use std::io::{Cursor, Read};
use zip::ZipArchive;

const MAX_RECORDS: usize = 500;
const MAX_PLAINTEXT: u64 = 256 * 1024 * 1024;
const MAX_CIPHERTEXT: u64 = 256 * 1024 * 1024;
const MAX_DECLARED_ZIP_BYTES: u64 = 1024 * 1024 * 1024;
const MAX_XML_DEPTH: usize = 200;
const EOCD: u32 = 0x0605_4b50;
const ZIP64_EOCD: u32 = 0x0606_4b50;
const ZIP64_LOCATOR: u32 = 0x0706_4b50;
const CENTRAL: u32 = 0x0201_4b50;
const LOCAL: u32 = 0x0403_4b50;

#[derive(Debug)]
struct Entry {
    name: String,
    uncompressed_size: u64,
    is_directory: bool,
}

struct CentralMember<'a> {
    compressed_size: u64,
    uncompressed_size: u64,
    crc32: u32,
    method: u16,
    central_offset: usize,
    name: &'a [u8],
    central_record: usize,
}

#[derive(Clone, Copy)]
enum MemberMeter {
    Plaintext,
    Ciphertext,
}

/// A ZIP whose central directory and every local data extent have been checked before use.
#[derive(Debug)]
pub(crate) struct Package<'a> {
    archive: ZipArchive<Cursor<&'a [u8]>>,
    entries: Vec<Entry>,
    by_name: HashMap<String, usize>,
    cache: HashMap<usize, Vec<u8>>,
    ciphertext_cache: HashMap<usize, Vec<u8>>,
    member_metered: HashMap<usize, u64>,
    ciphertext_metered: HashMap<usize, u64>,
    decrypted_metered: HashMap<usize, u64>,
    decrypted_finalized: HashSet<usize>,
    encrypted_members: HashSet<usize>,
    actual_plaintext: u64,
    actual_ciphertext: u64,
    warnings: Vec<ParseWarning>,
}

impl<'a> Package<'a> {
    pub(crate) fn open(bytes: &'a [u8]) -> Result<Self, KordocError> {
        let entries = validate_zip(bytes)?;
        if entries.len() > MAX_RECORDS {
            return Err(zip_bomb("HWPX package contains more than 500 ZIP records"));
        }

        let mut by_name = HashMap::with_capacity(entries.len());
        for (index, entry) in entries.iter().enumerate() {
            if by_name.insert(entry.name.clone(), index).is_some() {
                return Err(zip_bomb("HWPX package contains duplicate member names"));
            }
        }

        let archive = ZipArchive::new(Cursor::new(bytes))
            .map_err(|_| zip_bomb("HWPX ZIP central directory is invalid"))?;
        if archive.len() != entries.len() {
            return Err(zip_bomb("HWPX ZIP record count is inconsistent"));
        }

        Ok(Self {
            archive,
            entries,
            by_name,
            cache: HashMap::new(),
            ciphertext_cache: HashMap::new(),
            member_metered: HashMap::new(),
            ciphertext_metered: HashMap::new(),
            decrypted_metered: HashMap::new(),
            decrypted_finalized: HashSet::new(),
            encrypted_members: HashSet::new(),
            actual_plaintext: 0,
            actual_ciphertext: 0,
            warnings: Vec::new(),
        })
    }

    /// Returns a member's verified plaintext, charging each logical member once.
    pub(crate) fn read(&mut self, path: &str) -> Result<Option<Vec<u8>>, KordocError> {
        let Some(&entry_index) = self.by_name.get(path) else {
            return Ok(None);
        };
        if self.encrypted_members.contains(&entry_index) && !self.cache.contains_key(&entry_index) {
            return Err(KordocError::new(
                ErrorCode::Encrypted,
                "Encrypted HWPX member has not been decrypted",
            ));
        }
        self.read_index(entry_index, MemberMeter::Plaintext)
            .map(Some)
    }

    pub(crate) fn mark_encrypted_members(&mut self, paths: &[String]) -> Result<(), KordocError> {
        let mut seen = HashSet::with_capacity(paths.len());
        for path in paths {
            let Some(&index) = self.by_name.get(path) else {
                return Err(corrupted("Encrypted package member is missing"));
            };
            if self.entries[index].is_directory || !seen.insert(index) {
                return Err(corrupted("Encrypted package members are ambiguous"));
            }
        }
        self.encrypted_members.extend(seen);
        Ok(())
    }

    pub(crate) fn contains(&self, path: &str) -> bool {
        self.by_name
            .get(path)
            .is_some_and(|&index| !self.entries[index].is_directory)
    }

    pub(crate) fn file_paths(&self) -> Vec<String> {
        self.entries
            .iter()
            .filter(|entry| !entry.is_directory)
            .map(|entry| entry.name.clone())
            .collect()
    }

    pub(crate) fn file_count(&self) -> usize {
        self.entries
            .iter()
            .filter(|entry| !entry.is_directory)
            .count()
    }

    pub(crate) fn first_path(&self) -> Option<&str> {
        self.entries.first().map(|entry| entry.name.as_str())
    }

    /// Atomically publishes plaintext only after all members have been validated and charged.
    pub(crate) fn install_decrypted_batch(
        &mut self,
        plaintext: Vec<(String, Vec<u8>)>,
    ) -> Result<(), KordocError> {
        let mut seen = HashSet::with_capacity(plaintext.len());
        let mut staged = Vec::with_capacity(plaintext.len());
        for (path, bytes) in plaintext {
            let Some(&index) = self.by_name.get(&path) else {
                return Err(corrupted(
                    "Decrypted output does not identify a package member",
                ));
            };
            if self.entries[index].is_directory || !seen.insert(index) {
                return Err(corrupted("Decrypted package members are ambiguous"));
            }
            if !self.ciphertext_cache.contains_key(&index) {
                return Err(corrupted(
                    "Encrypted member ciphertext must be read before installation",
                ));
            }
            if !self.decrypted_finalized.contains(&index) {
                return Err(corrupted("Encrypted member plaintext was not finalized"));
            }
            let len = u64::try_from(bytes.len())
                .map_err(|_| decompression_bomb("HWPX decrypted member size overflows"))?;
            if self.decrypted_metered.get(&index).copied() != Some(len) {
                return Err(corrupted("Decrypted member byte count is inconsistent"));
            }
            staged.push((index, bytes));
        }
        if seen != self.encrypted_members {
            return Err(corrupted("Encrypted plaintext batch is incomplete"));
        }
        if self.actual_plaintext > MAX_PLAINTEXT {
            return Err(decompression_bomb(
                "HWPX package exceeds the plaintext byte limit",
            ));
        }
        for (index, bytes) in staged {
            self.cache.insert(index, bytes);
        }
        Ok(())
    }

    /// Reads encrypted-member ciphertext under its own cap, leaving plaintext budget untouched.
    pub(crate) fn read_ciphertext(&mut self, path: &str) -> Result<Option<Vec<u8>>, KordocError> {
        let Some(&entry_index) = self.by_name.get(path) else {
            return Ok(None);
        };
        self.read_index(entry_index, MemberMeter::Ciphertext)
            .map(Some)
    }

    /// Charges a decrypted output chunk before the caller appends it to an output buffer.
    pub(crate) fn charge_decrypted(
        &mut self,
        path: &str,
        amount: usize,
    ) -> Result<(), KordocError> {
        let Some(&entry_index) = self.by_name.get(path) else {
            return Err(corrupted(
                "Decrypted output does not identify a package member",
            ));
        };
        if !self.ciphertext_cache.contains_key(&entry_index) {
            return Err(corrupted(
                "Encrypted member ciphertext must be read before decryption",
            ));
        }
        if self.decrypted_finalized.contains(&entry_index) {
            return Err(corrupted(
                "Encrypted member plaintext was already finalized",
            ));
        }
        let amount = u64::try_from(amount)
            .map_err(|_| decompression_bomb("HWPX decrypted-byte increment overflows"))?;
        let member_total = self
            .decrypted_metered
            .get(&entry_index)
            .copied()
            .unwrap_or(0)
            .checked_add(amount)
            .ok_or_else(|| decompression_bomb("HWPX decrypted member size overflows"))?;
        let next = self
            .actual_plaintext
            .checked_add(amount)
            .ok_or_else(|| decompression_bomb("HWPX plaintext byte counter overflowed"))?;
        if next > MAX_PLAINTEXT {
            return Err(decompression_bomb(
                "HWPX package exceeds the plaintext byte limit",
            ));
        }
        self.decrypted_metered.insert(entry_index, member_total);
        self.actual_plaintext = next;
        Ok(())
    }

    /// Seals a successfully decrypted member against a second budget charge.
    pub(crate) fn finish_decrypted(&mut self, path: &str) -> Result<(), KordocError> {
        let Some(&entry_index) = self.by_name.get(path) else {
            return Err(corrupted(
                "Decrypted output does not identify a package member",
            ));
        };
        if !self.ciphertext_cache.contains_key(&entry_index) {
            return Err(corrupted(
                "Encrypted member ciphertext must be read before finalization",
            ));
        }
        self.decrypted_finalized.insert(entry_index);
        Ok(())
    }

    /// Removes plaintext metering from one failed PRF attempt before trying the fallback PRF.
    pub(crate) fn rollback_decrypted_attempt(&mut self, path: &str) -> Result<(), KordocError> {
        let Some(&entry_index) = self.by_name.get(path) else {
            return Err(corrupted(
                "Decrypted output does not identify a package member",
            ));
        };
        if self.decrypted_finalized.contains(&entry_index) {
            return Err(corrupted(
                "Finalized encrypted plaintext cannot be rolled back",
            ));
        }
        let charged = self.decrypted_metered.remove(&entry_index).unwrap_or(0);
        self.actual_plaintext = self
            .actual_plaintext
            .checked_sub(charged)
            .ok_or_else(|| corrupted("HWPX plaintext byte counter is inconsistent"))?;
        Ok(())
    }

    /// Reads an optional member, isolating only member-local corruption.
    pub(crate) fn read_optional(&mut self, path: &str) -> Result<Option<Vec<u8>>, KordocError> {
        match self.read(path) {
            Ok(value) => Ok(value),
            Err(error) if error.code == ErrorCode::Corrupted => {
                self.warnings.push(ParseWarning {
                    page: None,
                    message: format!("Optional HWPX member could not be read: {path}"),
                    code: WarningCode::BrokenZipRecovery,
                });
                Ok(None)
            }
            Err(error) => Err(error),
        }
    }

    pub(crate) fn take_warnings(&mut self) -> Vec<ParseWarning> {
        std::mem::take(&mut self.warnings)
    }

    fn read_index(
        &mut self,
        entry_index: usize,
        meter: MemberMeter,
    ) -> Result<Vec<u8>, KordocError> {
        let cache = match meter {
            MemberMeter::Plaintext => self.cache.get(&entry_index),
            MemberMeter::Ciphertext => self.ciphertext_cache.get(&entry_index),
        };
        if let Some(contents) = cache {
            return Ok(contents.clone());
        }
        let mut metered_total = match meter {
            MemberMeter::Plaintext => self.actual_plaintext,
            MemberMeter::Ciphertext => self.actual_ciphertext,
        };
        let limit = match meter {
            MemberMeter::Plaintext => MAX_PLAINTEXT,
            MemberMeter::Ciphertext => MAX_CIPHERTEXT,
        };
        let declared = self.entries[entry_index].uncompressed_size;
        let mut member = self
            .archive
            .by_index(entry_index)
            .map_err(|_| corrupted("HWPX member cannot be opened"))?;
        let mut output = Vec::new();
        let mut chunk = [0u8; 8192];
        loop {
            let count = member
                .read(&mut chunk)
                .map_err(|_| corrupted("HWPX member data or checksum is damaged"))?;
            if count == 0 {
                break;
            }
            let output_size = u64::try_from(output.len())
                .map_err(|_| decompression_bomb("HWPX member size overflows"))?;
            let increment = u64::try_from(count)
                .map_err(|_| decompression_bomb("HWPX member byte increment overflows"))?;
            let member_total = output_size
                .checked_add(increment)
                .ok_or_else(|| decompression_bomb("HWPX member byte counter overflowed"))?;
            let already_metered = match meter {
                MemberMeter::Plaintext => self.member_metered.get(&entry_index),
                MemberMeter::Ciphertext => self.ciphertext_metered.get(&entry_index),
            }
            .copied()
            .unwrap_or(0);
            let newly_metered = member_total.saturating_sub(already_metered);
            metered_total = metered_total
                .checked_add(newly_metered)
                .ok_or_else(|| decompression_bomb("HWPX byte counter overflowed"))?;
            if metered_total > limit {
                return Err(decompression_bomb("HWPX package exceeds its byte limit"));
            }
            let output_len = usize::try_from(member_total)
                .map_err(|_| decompression_bomb("HWPX member size overflows"))?;
            if member_total > declared {
                return Err(zip_bomb("HWPX member expanded beyond its declared size"));
            }
            match meter {
                MemberMeter::Plaintext => {
                    self.actual_plaintext = metered_total;
                    self.member_metered.insert(entry_index, member_total);
                }
                MemberMeter::Ciphertext => {
                    self.actual_ciphertext = metered_total;
                    self.ciphertext_metered.insert(entry_index, member_total);
                }
            }
            output.extend_from_slice(&chunk[..count]);
            debug_assert_eq!(output.len(), output_len);
        }
        if u64::try_from(output.len()).ok() != Some(declared) {
            return Err(corrupted(
                "HWPX member size does not match its directory record",
            ));
        }
        match meter {
            MemberMeter::Plaintext => self.cache.insert(entry_index, output.clone()),
            MemberMeter::Ciphertext => self.ciphertext_cache.insert(entry_index, output.clone()),
        };
        Ok(output)
    }

    /// Returns section members in manifest spine order, or numeric order without a manifest.
    pub(crate) fn section_paths(&mut self) -> Result<Vec<String>, KordocError> {
        if self.by_name.contains_key("Contents/content.hpf") {
            let bytes = self
                .read("Contents/content.hpf")?
                .ok_or_else(|| corrupted("HWPX content manifest is unavailable"))?;
            let Some(spine) = parse_spine(&bytes)? else {
                return self.numeric_section_paths();
            };
            let mut paths = Vec::with_capacity(spine.len());
            for href in spine {
                let path = manifest_path(&href)?;
                let Some(&entry_index) = self.by_name.get(&path) else {
                    return Err(corrupted("HWPX spine references a missing member"));
                };
                if self.entries[entry_index].is_directory {
                    return Err(corrupted("HWPX spine references a directory"));
                }
                if !paths.contains(&path) {
                    paths.push(path);
                }
            }
            return Ok(paths);
        }

        self.numeric_section_paths()
    }

    fn numeric_section_paths(&self) -> Result<Vec<String>, KordocError> {
        let mut paths: Vec<(String, String)> = self
            .entries
            .iter()
            .filter_map(|entry| {
                section_number(&entry.name).map(|number| (number, entry.name.clone()))
            })
            .collect();
        paths.sort_by(|(left_number, left_name), (right_number, right_name)| {
            natural_decimal_cmp(left_number, right_number).then_with(|| left_name.cmp(right_name))
        });
        Ok(paths.into_iter().map(|(_, path)| path).collect())
    }
}

fn parse_spine(bytes: &[u8]) -> Result<Option<Vec<String>>, KordocError> {
    let mut reader = Reader::from_reader(bytes);
    reader.config_mut().trim_text(true);
    let mut manifest = HashMap::<String, String>::new();
    let mut refs = Vec::<String>::new();
    let mut open_elements = Vec::<String>::new();
    let mut roots = 0usize;
    let mut buffer = Vec::new();
    loop {
        match reader.read_event_into(&mut buffer) {
            Ok(Event::Start(element)) => {
                if open_elements.len() >= MAX_XML_DEPTH {
                    return Err(corrupted(
                        "HWPX content manifest exceeds the XML depth limit",
                    ));
                }
                if open_elements.is_empty() {
                    roots += 1;
                    if roots > 1 {
                        return Err(corrupted("Malformed HWPX content manifest"));
                    }
                }
                open_elements.push(element.name().as_ref().to_owned());
                let in_spine = open_elements
                    .iter()
                    .take(open_elements.len().saturating_sub(1))
                    .any(|name| local_name(name) == "spine");
                record_manifest_element(&element, in_spine, &mut manifest, &mut refs)?;
            }
            Ok(Event::Empty(element)) => {
                if open_elements.is_empty() {
                    roots += 1;
                    if roots > 1 {
                        return Err(corrupted("Malformed HWPX content manifest"));
                    }
                }
                let in_spine = open_elements.iter().any(|name| local_name(name) == "spine");
                record_manifest_element(&element, in_spine, &mut manifest, &mut refs)?;
            }
            Ok(Event::End(element)) => {
                if open_elements.pop().as_deref() != Some(element.name().as_ref()) {
                    return Err(corrupted("Malformed HWPX content manifest"));
                }
            }
            Ok(Event::DocType(_)) => {
                return Err(corrupted("DTD is not allowed in HWPX content manifest"));
            }
            Ok(Event::Text(text))
                if open_elements.is_empty() && !text.as_ref().trim().is_empty() =>
            {
                return Err(corrupted("Malformed HWPX content manifest"));
            }
            Ok(Event::GeneralRef(_)) => {
                return Err(corrupted(
                    "Entities are not allowed in HWPX content manifest",
                ));
            }
            Ok(Event::Eof) => break,
            Ok(_) => {}
            Err(_) => return Err(corrupted("Malformed HWPX content manifest")),
        }
        buffer.clear();
    }
    if !open_elements.is_empty() || roots != 1 {
        return Err(corrupted("Malformed HWPX content manifest"));
    }
    if refs.is_empty() {
        return Ok(None);
    }
    refs.into_iter()
        .map(|id| {
            manifest
                .get(&id)
                .cloned()
                .ok_or_else(|| corrupted("HWPX spine references an unknown item"))
        })
        .collect::<Result<Vec<_>, KordocError>>()
        .map(Some)
}

fn record_manifest_element(
    element: &BytesStart<'_>,
    in_spine: bool,
    manifest: &mut HashMap<String, String>,
    refs: &mut Vec<String>,
) -> Result<(), KordocError> {
    let element_name = element.name();
    match local_name(element_name.as_ref()) {
        "item" => {
            let mut id = None;
            let mut href = None;
            for attribute in element.attributes().with_checks(true) {
                let attribute =
                    attribute.map_err(|_| corrupted("Malformed HWPX content manifest"))?;
                let key = local_name(attribute.key.as_ref());
                let value = attribute
                    .normalized_value(quick_xml::XmlVersion::Implicit1_0)
                    .map_err(|_| corrupted("Malformed HWPX content manifest"))?
                    .into_owned();
                if key == "id" {
                    id = Some(value);
                } else if key == "href" {
                    href = Some(value);
                }
            }
            if let (Some(id), Some(href)) = (id, href)
                && manifest.insert(id, href).is_some()
            {
                return Err(corrupted("Duplicate item in HWPX content manifest"));
            }
        }
        "itemref" if in_spine => {
            for attribute in element.attributes().with_checks(true) {
                let attribute =
                    attribute.map_err(|_| corrupted("Malformed HWPX content manifest"))?;
                if local_name(attribute.key.as_ref()) == "idref" {
                    refs.push(
                        attribute
                            .normalized_value(quick_xml::XmlVersion::Implicit1_0)
                            .map_err(|_| corrupted("Malformed HWPX content manifest"))?
                            .into_owned(),
                    );
                }
            }
        }
        _ => {}
    }
    Ok(())
}

fn local_name(name: &str) -> &str {
    name.rsplit(':').next().unwrap_or(name)
}

fn manifest_path(href: &str) -> Result<String, KordocError> {
    if href.is_empty() || href.starts_with('/') || href.contains(['\\', ':', '\0', '?', '#']) {
        return Err(zip_bomb("HWPX manifest contains an unsafe member path"));
    }
    let mut components = Vec::new();
    for component in href.split('/') {
        if component.is_empty() || component == "." || component == ".." {
            return Err(zip_bomb("HWPX manifest contains an unsafe member path"));
        }
        components.push(component);
    }
    let name = components.join("/");
    if name.starts_with("Contents/") {
        Ok(name)
    } else {
        Ok(format!("Contents/{name}"))
    }
}

fn section_number(path: &str) -> Option<String> {
    let basename = path.strip_prefix("Contents/")?;
    let digits = basename.strip_prefix("section")?.strip_suffix(".xml")?;
    if digits.is_empty() || !digits.bytes().all(|byte| byte.is_ascii_digit()) {
        return None;
    }
    Some(digits.trim_start_matches('0').to_owned().if_empty_then("0"))
}

trait EmptyFallback {
    fn if_empty_then(self, fallback: &str) -> Self;
}

impl EmptyFallback for String {
    fn if_empty_then(self, fallback: &str) -> Self {
        if self.is_empty() {
            fallback.to_owned()
        } else {
            self
        }
    }
}

fn natural_decimal_cmp(left: &str, right: &str) -> std::cmp::Ordering {
    left.len().cmp(&right.len()).then_with(|| left.cmp(right))
}

fn validate_zip(bytes: &[u8]) -> Result<Vec<Entry>, KordocError> {
    let (central_offset, central_size, record_count, central_end) = locate_directory(bytes)?;
    if record_count > MAX_RECORDS {
        return Err(zip_bomb("HWPX package contains more than 500 ZIP records"));
    }
    let offset = to_usize(central_offset, "ZIP central directory offset overflows")?;
    let size = to_usize(central_size, "ZIP central directory size overflows")?;
    let end = offset
        .checked_add(size)
        .filter(|end| *end == central_end)
        .ok_or_else(|| zip_bomb("ZIP central directory span is inconsistent"))?;
    let mut cursor = offset;
    let mut entries = Vec::with_capacity(record_count.min(MAX_RECORDS + 1));
    let mut declared_total = 0u64;
    let mut seen = HashSet::new();
    for _ in 0..record_count {
        if read_u32(bytes, cursor) != Some(CENTRAL) {
            return Err(zip_bomb("Malformed ZIP central directory entry"));
        }
        let fixed_end = cursor
            .checked_add(46)
            .ok_or_else(|| zip_bomb("ZIP central entry overflows"))?;
        if fixed_end > end {
            return Err(zip_bomb("Truncated ZIP central directory entry"));
        }
        let name_len = read_u16(bytes, cursor + 28).unwrap() as usize;
        let extra_len = read_u16(bytes, cursor + 30).unwrap() as usize;
        let comment_len = read_u16(bytes, cursor + 32).unwrap() as usize;
        let variable_end = fixed_end
            .checked_add(name_len)
            .and_then(|value| value.checked_add(extra_len))
            .and_then(|value| value.checked_add(comment_len))
            .filter(|value| *value <= end)
            .ok_or_else(|| zip_bomb("ZIP central entry fields exceed directory span"))?;
        let name_bytes = &bytes[fixed_end..fixed_end + name_len];
        let name = std::str::from_utf8(name_bytes)
            .map_err(|_| zip_bomb("HWPX ZIP member name is not valid UTF-8"))?
            .to_owned();
        validate_member_path(&name)?;
        if !seen.insert(name.clone()) {
            return Err(zip_bomb("HWPX package contains duplicate member names"));
        }
        let extra_start = fixed_end + name_len;
        let extra_end = extra_start + extra_len;
        let mut uncompressed = read_u32(bytes, cursor + 24).unwrap() as u64;
        let mut compressed = read_u32(bytes, cursor + 20).unwrap() as u64;
        let mut local_offset = read_u32(bytes, cursor + 42).unwrap() as u64;
        let disk_start = read_u16(bytes, cursor + 34).unwrap();
        if disk_start != 0 && disk_start != u16::MAX {
            return Err(zip_bomb("Multi-disk ZIP members are not supported"));
        }
        let needs_uncompressed = uncompressed == u32::MAX as u64;
        let needs_compressed = compressed == u32::MAX as u64;
        let needs_offset = local_offset == u32::MAX as u64;
        let needs_disk = disk_start == u16::MAX;
        let mut zip64_values = None;
        let mut extra_cursor = extra_start;
        while extra_cursor < extra_end {
            let header_end = extra_cursor
                .checked_add(4)
                .filter(|value| *value <= extra_end)
                .ok_or_else(|| zip_bomb("Malformed ZIP extra field"))?;
            let tag = read_u16(bytes, extra_cursor).unwrap();
            let field_len = read_u16(bytes, extra_cursor + 2).unwrap() as usize;
            let field_end = header_end
                .checked_add(field_len)
                .filter(|value| *value <= extra_end)
                .ok_or_else(|| zip_bomb("ZIP extra field exceeds central entry"))?;
            if tag == 0x0001 {
                if zip64_values.is_some() {
                    return Err(zip_bomb("Duplicate ZIP64 extra field"));
                }
                zip64_values = Some(&bytes[header_end..field_end]);
            }
            extra_cursor = field_end;
        }
        if needs_uncompressed || needs_compressed || needs_offset || needs_disk {
            let values =
                zip64_values.ok_or_else(|| zip_bomb("Required ZIP64 values are missing"))?;
            let mut at = 0;
            if needs_uncompressed {
                uncompressed = take_u64(values, &mut at)?;
            }
            if needs_compressed {
                compressed = take_u64(values, &mut at)?;
            }
            if needs_offset {
                local_offset = take_u64(values, &mut at)?;
            }
            if needs_disk {
                let disk = take_u32(values, &mut at)?;
                if disk != 0 {
                    return Err(zip_bomb("Multi-disk ZIP64 members are not supported"));
                }
            }
            if at != values.len() {
                return Err(zip_bomb("ZIP64 extra field has unexpected values"));
            }
        }
        declared_total = declared_total
            .checked_add(uncompressed)
            .ok_or_else(|| zip_bomb("ZIP declared plaintext size overflowed"))?;
        if declared_total > MAX_DECLARED_ZIP_BYTES {
            return Err(zip_bomb(
                "ZIP declared plaintext exceeds generic archive limit",
            ));
        }
        validate_local_extent(
            bytes,
            local_offset,
            CentralMember {
                compressed_size: compressed,
                uncompressed_size: uncompressed,
                crc32: read_u32(bytes, cursor + 16).unwrap(),
                method: read_u16(bytes, cursor + 10).unwrap(),
                central_offset: offset,
                name: name_bytes,
                central_record: cursor,
            },
        )?;
        let flags = read_u16(bytes, cursor + 8).unwrap();
        if flags & 1 != 0 {
            return Err(KordocError::new(
                ErrorCode::Encrypted,
                "Encrypted ZIP members require HWPX encryption metadata",
            ));
        }
        entries.push(Entry {
            is_directory: name.ends_with('/'),
            name,
            uncompressed_size: uncompressed,
        });
        cursor = variable_end;
    }
    if cursor != end {
        return Err(zip_bomb(
            "ZIP central directory span does not match its records",
        ));
    }
    Ok(entries)
}

fn locate_directory(bytes: &[u8]) -> Result<(u64, u64, usize, usize), KordocError> {
    let search_start = bytes.len().saturating_sub(65_557);
    let eocd = (search_start..bytes.len().saturating_sub(3))
        .rev()
        .find(|&position| read_u32(bytes, position) == Some(EOCD))
        .ok_or_else(|| zip_bomb("ZIP end record is missing"))?;
    if eocd.checked_add(22).is_none_or(|fixed| fixed > bytes.len()) {
        return Err(zip_bomb("Truncated ZIP end record"));
    }
    let comment_len = read_u16(bytes, eocd + 20).unwrap() as usize;
    if eocd.checked_add(22 + comment_len) != Some(bytes.len()) {
        return Err(zip_bomb("ZIP end record comment span is invalid"));
    }
    let disk = read_u16(bytes, eocd + 4).unwrap();
    let central_disk = read_u16(bytes, eocd + 6).unwrap();
    let entries_disk16 = read_u16(bytes, eocd + 8).unwrap();
    let entries_total16 = read_u16(bytes, eocd + 10).unwrap();
    let size32 = read_u32(bytes, eocd + 12).unwrap();
    let offset32 = read_u32(bytes, eocd + 16).unwrap();
    if disk != 0 || central_disk != 0 {
        return Err(zip_bomb("Multi-disk ZIP archives are not supported"));
    }
    let zip64_needed = entries_disk16 == u16::MAX
        || entries_total16 == u16::MAX
        || size32 == u32::MAX
        || offset32 == u32::MAX;
    if !zip64_needed {
        if entries_disk16 != entries_total16 {
            return Err(zip_bomb("ZIP entry count differs across disks"));
        }
        let offset = offset32 as u64;
        let size = size32 as u64;
        let end = offset
            .checked_add(size)
            .ok_or_else(|| zip_bomb("ZIP central directory overflows"))?;
        if end != eocd as u64 {
            return Err(zip_bomb(
                "ZIP central directory does not meet its end record",
            ));
        }
        return Ok((offset, size, entries_total16 as usize, eocd));
    }
    let locator = eocd
        .checked_sub(20)
        .ok_or_else(|| zip_bomb("ZIP64 locator is missing"))?;
    if read_u32(bytes, locator) != Some(ZIP64_LOCATOR)
        || read_u32(bytes, locator + 4) != Some(0)
        || read_u32(bytes, locator + 16) != Some(1)
    {
        return Err(zip_bomb("Invalid or multi-disk ZIP64 locator"));
    }
    let zip64_offset =
        read_u64(bytes, locator + 8).ok_or_else(|| zip_bomb("Truncated ZIP64 locator"))?;
    let zip64_at = to_usize(zip64_offset, "ZIP64 end record offset overflows")?;
    if read_u32(bytes, zip64_at) != Some(ZIP64_EOCD) {
        return Err(zip_bomb("ZIP64 end record is missing"));
    }
    let record_size =
        read_u64(bytes, zip64_at + 4).ok_or_else(|| zip_bomb("Truncated ZIP64 end record"))?;
    let zip64_end = zip64_offset
        .checked_add(12)
        .and_then(|value| value.checked_add(record_size))
        .ok_or_else(|| zip_bomb("ZIP64 end record span overflows"))?;
    if zip64_end != locator as u64 || record_size < 44 {
        return Err(zip_bomb("ZIP64 end record span is invalid"));
    }
    if read_u32(bytes, zip64_at + 16) != Some(0) || read_u32(bytes, zip64_at + 20) != Some(0) {
        return Err(zip_bomb("Multi-disk ZIP64 archives are not supported"));
    }
    let entries_disk = read_u64(bytes, zip64_at + 24).unwrap();
    let entries_total = read_u64(bytes, zip64_at + 32).unwrap();
    if entries_disk != entries_total {
        return Err(zip_bomb("ZIP64 entry count differs across disks"));
    }
    if entries_total > MAX_RECORDS as u64 + 1 {
        return Err(zip_bomb("ZIP record count exceeds HWPX package limit"));
    }
    if entries_disk16 != u16::MAX && entries_disk16 as u64 != entries_disk
        || entries_total16 != u16::MAX && entries_total16 as u64 != entries_total
        || size32 != u32::MAX && size32 as u64 != read_u64(bytes, zip64_at + 40).unwrap()
        || offset32 != u32::MAX && offset32 as u64 != read_u64(bytes, zip64_at + 48).unwrap()
    {
        return Err(zip_bomb("ZIP and ZIP64 end records disagree"));
    }
    Ok((
        read_u64(bytes, zip64_at + 48).unwrap(),
        read_u64(bytes, zip64_at + 40).unwrap(),
        usize::try_from(entries_total).map_err(|_| zip_bomb("ZIP64 entry count overflows"))?,
        zip64_at,
    ))
}

fn validate_local_extent(
    bytes: &[u8],
    local_offset: u64,
    central: CentralMember<'_>,
) -> Result<(), KordocError> {
    let offset = to_usize(local_offset, "ZIP local header offset overflows")?;
    let fixed_end = offset
        .checked_add(30)
        .ok_or_else(|| zip_bomb("ZIP local header overflows"))?;
    if fixed_end > central.central_offset || read_u32(bytes, offset) != Some(LOCAL) {
        return Err(zip_bomb("ZIP local header is outside its data area"));
    }
    let name_len = read_u16(bytes, offset + 26).unwrap() as usize;
    let extra_len = read_u16(bytes, offset + 28).unwrap() as usize;
    let name_end = fixed_end
        .checked_add(name_len)
        .filter(|end| *end <= central.central_offset)
        .ok_or_else(|| zip_bomb("ZIP local name exceeds its data area"))?;
    if bytes.get(fixed_end..name_end) != Some(central.name) {
        return Err(zip_bomb("ZIP local and central member names differ"));
    }
    let data_start = name_end
        .checked_add(extra_len)
        .filter(|end| *end <= central.central_offset)
        .ok_or_else(|| zip_bomb("ZIP local extra field exceeds data area"))?;
    let data_end = (data_start as u64)
        .checked_add(central.compressed_size)
        .ok_or_else(|| zip_bomb("ZIP compressed data span overflows"))?;
    if data_end > central.central_offset as u64 {
        return Err(zip_bomb("ZIP compressed data exceeds its data area"));
    }
    let local_flags = read_u16(bytes, offset + 6).unwrap();
    let central_flags = read_u16(bytes, central.central_record + 8).unwrap();
    if local_flags != central_flags || read_u16(bytes, offset + 8).unwrap() != central.method {
        return Err(zip_bomb("ZIP local and central flags differ"));
    }
    if local_flags & 0x0008 == 0 {
        if read_u32(bytes, offset + 14) != Some(central.crc32) {
            return Err(zip_bomb("ZIP local and central checksums differ"));
        }
        let local_compressed = read_u32(bytes, offset + 18).unwrap();
        let local_uncompressed = read_u32(bytes, offset + 22).unwrap();
        let (local_uncompressed, local_compressed) =
            if local_compressed == u32::MAX || local_uncompressed == u32::MAX {
                local_zip64_sizes(
                    bytes,
                    name_end,
                    extra_len,
                    local_uncompressed,
                    local_compressed,
                )?
            } else {
                (local_uncompressed as u64, local_compressed as u64)
            };
        if local_compressed != central.compressed_size
            || local_uncompressed != central.uncompressed_size
        {
            return Err(zip_bomb("ZIP local and central member sizes differ"));
        }
    }
    Ok(())
}

fn local_zip64_sizes(
    bytes: &[u8],
    extra_start: usize,
    extra_len: usize,
    uncompressed32: u32,
    compressed32: u32,
) -> Result<(u64, u64), KordocError> {
    let extra_end = extra_start
        .checked_add(extra_len)
        .filter(|end| *end <= bytes.len())
        .ok_or_else(|| zip_bomb("ZIP local extra field exceeds input"))?;
    let needs_uncompressed = uncompressed32 == u32::MAX;
    let needs_compressed = compressed32 == u32::MAX;
    let mut cursor = extra_start;
    while cursor < extra_end {
        let header_end = cursor
            .checked_add(4)
            .filter(|end| *end <= extra_end)
            .ok_or_else(|| zip_bomb("Malformed ZIP local extra field"))?;
        let tag = read_u16(bytes, cursor).unwrap();
        let field_len = read_u16(bytes, cursor + 2).unwrap() as usize;
        let field_end = header_end
            .checked_add(field_len)
            .filter(|end| *end <= extra_end)
            .ok_or_else(|| zip_bomb("ZIP local extra field exceeds its header"))?;
        if tag == 0x0001 {
            let values = &bytes[header_end..field_end];
            let mut at = 0;
            let uncompressed = if needs_uncompressed {
                take_u64(values, &mut at)?
            } else {
                uncompressed32 as u64
            };
            let compressed = if needs_compressed {
                take_u64(values, &mut at)?
            } else {
                compressed32 as u64
            };
            if at != values.len() {
                return Err(zip_bomb(
                    "ZIP local ZIP64 extra field has unexpected values",
                ));
            }
            return Ok((uncompressed, compressed));
        }
        cursor = field_end;
    }
    Err(zip_bomb("Required local ZIP64 sizes are missing"))
}

fn validate_member_path(path: &str) -> Result<(), KordocError> {
    if path.is_empty() || path.starts_with('/') || path.contains(['\\', ':', '\0']) {
        return Err(zip_bomb("HWPX ZIP member path is not canonical"));
    }
    let mut components = path.split('/').peekable();
    while let Some(component) = components.next() {
        let final_empty = component.is_empty() && components.peek().is_none();
        if component == "." || component == ".." || component.is_empty() && !final_empty {
            return Err(zip_bomb("HWPX ZIP member path is not canonical"));
        }
    }
    Ok(())
}

fn take_u64(bytes: &[u8], cursor: &mut usize) -> Result<u64, KordocError> {
    let end = cursor
        .checked_add(8)
        .ok_or_else(|| zip_bomb("ZIP64 values overflow"))?;
    let value = read_u64(bytes, *cursor)
        .filter(|_| end <= bytes.len())
        .ok_or_else(|| zip_bomb("Truncated ZIP64 extra field"))?;
    *cursor = end;
    Ok(value)
}

fn take_u32(bytes: &[u8], cursor: &mut usize) -> Result<u32, KordocError> {
    let end = cursor
        .checked_add(4)
        .ok_or_else(|| zip_bomb("ZIP64 values overflow"))?;
    let value = read_u32(bytes, *cursor)
        .filter(|_| end <= bytes.len())
        .ok_or_else(|| zip_bomb("Truncated ZIP64 extra field"))?;
    *cursor = end;
    Ok(value)
}

fn to_usize(value: u64, message: &str) -> Result<usize, KordocError> {
    usize::try_from(value).map_err(|_| zip_bomb(message))
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

fn zip_bomb(message: &str) -> KordocError {
    KordocError::new(ErrorCode::ZipBomb, message)
}

fn decompression_bomb(message: &str) -> KordocError {
    KordocError::new(ErrorCode::DecompressionBomb, message)
}

fn corrupted(message: &str) -> KordocError {
    KordocError::new(ErrorCode::Corrupted, message)
}

#[cfg(test)]
mod tests;
