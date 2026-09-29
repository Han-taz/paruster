use kordoc_ir::{ErrorCode, KordocError};

const MAX_HTML_TABLE_BYTES: usize = 8 * 1024 * 1024;
const MAX_ENTITY_LOOKAHEAD: usize = 16;
const MAX_HTML_DEPTH: usize = 64;
const MAX_TABLE_ROWS: usize = 10_000;
const MAX_TABLE_COLUMNS: usize = 200;
const MAX_TABLE_CELLS: usize = 2_000_000;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MarkdownUnitKind {
    Text,
    GfmTable,
    HtmlTable,
    Separator,
    Image,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MarkdownUnit {
    pub kind: MarkdownUnitKind,
    pub raw: String,
    pub lines: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HtmlCell {
    pub inner: String,
    pub col_span: u32,
    pub row_span: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HtmlRow {
    pub header: bool,
    pub cells: Vec<HtmlCell>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HtmlCellLines {
    pub lines: Vec<String>,
    pub had_non_text: bool,
    pub img_srcs: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CellTableSplit {
    /// Text before each table and after the final table.
    pub texts: Vec<String>,
    pub tables: Vec<String>,
}

/// Split the Markdown emitted by the table builder into text and table units.
pub fn split_markdown_units(markdown: &str) -> Result<Vec<MarkdownUnit>, KordocError> {
    split_markdown_units_with_budget(
        markdown,
        crate::markdown::MAX_MARKDOWN_BYTES,
        crate::markdown::MAX_MARKDOWN_BYTES,
    )
}

fn split_markdown_units_with_budget(
    markdown: &str,
    max_input_bytes: usize,
    max_output_bytes: usize,
) -> Result<Vec<MarkdownUnit>, KordocError> {
    if markdown.len() > max_input_bytes {
        return Err(output_too_large(
            "Markdown input exceeds the configured byte limit",
        ));
    }
    let mut output_budget = OutputBudget {
        used: 0,
        limit: max_output_bytes,
    };
    let lines: Vec<&str> = markdown.split('\n').collect();
    let mut units = Vec::new();
    let mut index = 0;
    while index < lines.len() {
        let line = lines[index];
        let trimmed = line.trim();
        if trimmed.is_empty() {
            index += 1;
            continue;
        }

        if trimmed.starts_with("<table") && starts_tag(trimmed, "table") {
            let start = index;
            let mut table_lines = Vec::new();
            let mut depth = 0usize;
            let mut found_root = false;
            let mut bytes = 0usize;
            while index < lines.len() {
                let current = lines[index];
                bytes = bytes
                    .checked_add(current.len())
                    .and_then(|total| total.checked_add(usize::from(index > start)))
                    .ok_or_else(|| output_too_large("Markdown HTML table size overflowed"))?;
                if bytes > MAX_HTML_TABLE_BYTES {
                    return Err(output_too_large("Markdown HTML table exceeds 8 MiB"));
                }
                scan_table_depth(current, &mut depth, &mut found_root)?;
                table_lines.push(output_budget.copy(current)?);
                index += 1;
                if found_root && depth == 0 {
                    break;
                }
            }
            output_budget.charge(joined_len(&table_lines)?)?;
            let raw = table_lines.join("\n");
            parse_html_table(&raw)?;
            units.push(MarkdownUnit {
                kind: MarkdownUnitKind::HtmlTable,
                raw,
                lines: table_lines,
            });
            continue;
        }

        if line.trim_start().starts_with('|') {
            let mut table_lines = Vec::new();
            while index < lines.len() && lines[index].trim_start().starts_with('|') {
                table_lines.push(output_budget.copy(lines[index])?);
                index += 1;
            }
            output_budget.charge(joined_len(&table_lines)?)?;
            parse_gfm_table(&table_lines)?;
            let raw = table_lines.join("\n");
            units.push(MarkdownUnit {
                kind: MarkdownUnitKind::GfmTable,
                raw,
                lines: table_lines,
            });
            continue;
        }

        if is_separator(trimmed) {
            let raw = output_budget.copy(trimmed)?;
            let line = output_budget.copy(trimmed)?;
            units.push(MarkdownUnit {
                kind: MarkdownUnitKind::Separator,
                raw,
                lines: vec![line],
            });
            index += 1;
            continue;
        }

        if is_standalone_image(trimmed) {
            let raw = output_budget.copy(trimmed)?;
            let line = output_budget.copy(trimmed)?;
            units.push(MarkdownUnit {
                kind: MarkdownUnitKind::Image,
                raw,
                lines: vec![line],
            });
            index += 1;
            continue;
        }

        let mut text_lines = Vec::new();
        while index < lines.len() {
            let current = lines[index];
            let current_trimmed = current.trim();
            if current_trimmed.is_empty()
                || current.trim_start().starts_with('|')
                || (current_trimmed.starts_with("<table") && starts_tag(current_trimmed, "table"))
            {
                break;
            }
            text_lines.push(output_budget.copy(current_trimmed)?);
            index += 1;
        }
        let raw_len = joined_len(&text_lines)?;
        output_budget.charge(raw_len)?;
        let raw = text_lines.join("\n");
        units.push(MarkdownUnit {
            kind: MarkdownUnitKind::Text,
            raw,
            lines: text_lines,
        });
    }
    Ok(units)
}

struct OutputBudget {
    used: usize,
    limit: usize,
}

impl OutputBudget {
    fn charge(&mut self, bytes: usize) -> Result<(), KordocError> {
        self.used = self
            .used
            .checked_add(bytes)
            .filter(|used| *used <= self.limit)
            .ok_or_else(|| {
                output_too_large("Markdown units exceed the configured output byte limit")
            })?;
        Ok(())
    }

    fn copy(&mut self, input: &str) -> Result<String, KordocError> {
        self.charge(input.len())?;
        Ok(input.to_owned())
    }
}

fn joined_len(lines: &[String]) -> Result<usize, KordocError> {
    let content = lines
        .iter()
        .try_fold(0usize, |total, line| total.checked_add(line.len()))
        .ok_or_else(|| output_too_large("Markdown unit output size overflowed"))?;
    content
        .checked_add(lines.len().saturating_sub(1))
        .ok_or_else(|| output_too_large("Markdown unit output size overflowed"))
}

/// Parse GFM rows, retaining escaped pipes and inline `<br>` markup verbatim.
pub fn parse_gfm_table(lines: &[String]) -> Result<Vec<Vec<String>>, KordocError> {
    let mut rows = Vec::new();
    let mut total_cells = 0usize;
    for line in lines {
        let Some(cells) = split_gfm_row_with_limit(line, MAX_TABLE_COLUMNS)? else {
            continue;
        };
        if cells.is_empty() {
            continue;
        }
        if cells.iter().all(|cell| is_gfm_delimiter(cell)) {
            continue;
        }
        total_cells = total_cells
            .checked_add(cells.len())
            .ok_or_else(|| output_too_large("GFM table cell count overflowed"))?;
        if total_cells > MAX_TABLE_CELLS || rows.len() >= MAX_TABLE_ROWS {
            return Err(output_too_large(
                "GFM table exceeds the configured cell budget",
            ));
        }
        rows.push(cells);
    }
    Ok(rows)
}

fn split_gfm_row_with_limit(
    line: &str,
    max_columns: usize,
) -> Result<Option<Vec<String>>, KordocError> {
    let trimmed = line.trim();
    if !trimmed.starts_with('|') {
        return Ok(None);
    }
    let bytes = trimmed.as_bytes();
    let mut cells = Vec::new();
    let mut start = 1usize;
    let mut index = 1usize;
    while index < bytes.len() {
        if bytes[index] == b'|' && !is_escaped_pipe(bytes, index) {
            if cells.len() >= max_columns {
                return Err(output_too_large(
                    "GFM table row exceeds the configured column budget",
                ));
            }
            cells.push(trimmed[start..index].trim().to_owned());
            start = index + 1;
        }
        index += 1;
    }
    if start <= bytes.len() {
        let final_cell = trimmed[start..].trim();
        if !final_cell.is_empty() || !trimmed.ends_with('|') {
            if cells.len() >= max_columns {
                return Err(output_too_large(
                    "GFM table row exceeds the configured column budget",
                ));
            }
            cells.push(final_cell.to_owned());
        }
    }
    Ok(Some(cells))
}

/// Parse one balanced HTML table, preserving nested markup inside each cell.
pub fn parse_html_table(raw: &str) -> Result<Vec<HtmlRow>, KordocError> {
    ensure_html_size(raw)?;
    let mut rows: Vec<HtmlRow> = Vec::new();
    let mut open: Vec<OpenTag> = Vec::new();
    let mut table_depth = 0usize;
    let mut root_seen = false;
    let mut root_closed = false;
    let mut current_row: Option<Vec<HtmlCell>> = None;
    let mut current_cell: Option<(usize, String, u32, u32)> = None;
    let mut total_cells = 0usize;
    let mut cursor = 0usize;

    while let Some(tag) = next_tag(raw, cursor)? {
        cursor = tag.end;
        match tag.name.as_str() {
            "table" => {
                if tag.closing {
                    pop_expected(&mut open, "table")?;
                    table_depth = table_depth.checked_sub(1).ok_or_else(malformed_html)?;
                    if table_depth == 0 {
                        root_closed = true;
                    }
                } else {
                    if table_depth == 0 {
                        if root_seen || !raw[..tag.start].trim().is_empty() {
                            return Err(malformed_html());
                        }
                        root_seen = true;
                    }
                    table_depth += 1;
                    if table_depth > MAX_HTML_DEPTH {
                        return Err(output_too_large("HTML table nesting exceeds 64 levels"));
                    }
                    if !tag.self_closing {
                        open.push(OpenTag {
                            name: "table".to_owned(),
                        });
                    } else {
                        table_depth -= 1;
                        if table_depth == 0 {
                            root_closed = true;
                        }
                    }
                }
            }
            "tr" if table_depth == 1 => {
                if tag.closing {
                    pop_expected(&mut open, "tr")?;
                    if let Some(cells) = current_row.take() {
                        if rows.len() >= MAX_TABLE_ROWS {
                            return Err(output_too_large("HTML table exceeds 10,000 rows"));
                        }
                        rows.push(HtmlRow {
                            header: rows.is_empty(),
                            cells,
                        });
                    }
                } else {
                    if current_row.is_some() {
                        return Err(malformed_html());
                    }
                    current_row = Some(Vec::new());
                    if !tag.self_closing {
                        open.push(OpenTag {
                            name: "tr".to_owned(),
                        });
                    }
                }
            }
            "td" | "th" if table_depth == 1 => {
                if tag.closing {
                    pop_expected(&mut open, &tag.name)?;
                    let (start, name, col_span, row_span) =
                        current_cell.take().ok_or_else(malformed_html)?;
                    if name != tag.name {
                        return Err(malformed_html());
                    }
                    let row = current_row.as_mut().ok_or_else(malformed_html)?;
                    row.push(HtmlCell {
                        inner: raw[start..tag.start].to_owned(),
                        col_span,
                        row_span,
                    });
                    total_cells = total_cells
                        .checked_add(1)
                        .ok_or_else(|| output_too_large("HTML table cell count overflowed"))?;
                    if row.len() > MAX_TABLE_COLUMNS || total_cells > MAX_TABLE_CELLS {
                        return Err(output_too_large(
                            "HTML table exceeds the configured cell budget",
                        ));
                    }
                } else {
                    if current_cell.is_some() {
                        return Err(malformed_html());
                    }
                    let (col_span, row_span) = parse_spans(&tag.attrs)?;
                    current_cell = Some((tag.end, tag.name.clone(), col_span, row_span));
                    if !tag.self_closing {
                        open.push(OpenTag { name: tag.name });
                    } else {
                        let row = current_row.as_mut().ok_or_else(malformed_html)?;
                        row.push(HtmlCell {
                            inner: String::new(),
                            col_span,
                            row_span,
                        });
                        current_cell = None;
                        total_cells = total_cells
                            .checked_add(1)
                            .ok_or_else(|| output_too_large("HTML table cell count overflowed"))?;
                        if row.len() > MAX_TABLE_COLUMNS || total_cells > MAX_TABLE_CELLS {
                            return Err(output_too_large(
                                "HTML table exceeds the configured cell budget",
                            ));
                        }
                    }
                }
            }
            _ => {}
        }
    }
    if !root_seen
        || !root_closed
        || table_depth != 0
        || !open.is_empty()
        || current_row.is_some()
        || current_cell.is_some()
    {
        return Err(malformed_html());
    }
    if !raw[cursor..].trim().is_empty() {
        return Err(malformed_html());
    }
    Ok(rows)
}

/// Convert supported HTML cell output to text lines while retaining image presence.
pub fn html_cell_inner_to_lines(inner: &str) -> Result<HtmlCellLines, KordocError> {
    ensure_html_size(inner)?;
    let split = split_cell_by_top_level_tables(inner)?;
    let mut had_non_text = !split.tables.is_empty();
    let without_tables = split.texts.concat();
    let (without_images, img_srcs, found_images) = remove_images(&without_tables)?;
    had_non_text |= found_images;
    let mut lines = Vec::new();
    for part in split_br_tags(&without_images)? {
        let decoded = decode_html_entities(part)?.trim().to_owned();
        if !decoded.is_empty() {
            lines.push(decoded);
        }
    }
    Ok(HtmlCellLines {
        lines,
        had_non_text,
        img_srcs,
    })
}

/// Split cell markup around balanced top-level table elements in source order.
pub fn split_cell_by_top_level_tables(html: &str) -> Result<CellTableSplit, KordocError> {
    ensure_html_size(html)?;
    let mut tables = Vec::new();
    let mut texts = Vec::new();
    let mut table_depth = 0usize;
    let mut start = 0usize;
    let mut cursor = 0usize;
    let mut segment_start = None;
    while let Some(tag) = next_tag(html, cursor)? {
        cursor = tag.end;
        if tag.name != "table" {
            continue;
        }
        if !tag.closing {
            if table_depth == 0 {
                texts.push(html[start..tag.start].to_owned());
                segment_start = Some(tag.start);
            }
            table_depth += 1;
            if table_depth > MAX_HTML_DEPTH {
                return Err(output_too_large("HTML table nesting exceeds 64 levels"));
            }
            if tag.self_closing {
                table_depth -= 1;
                if table_depth == 0 {
                    let table_start = segment_start.take().ok_or_else(malformed_html)?;
                    tables.push(html[table_start..tag.end].to_owned());
                    start = tag.end;
                }
            }
        } else {
            table_depth = table_depth.checked_sub(1).ok_or_else(malformed_html)?;
            if table_depth == 0 {
                let table_start = segment_start.take().ok_or_else(malformed_html)?;
                tables.push(html[table_start..tag.end].to_owned());
                start = tag.end;
            }
        }
    }
    if table_depth != 0 {
        return Err(malformed_html());
    }
    texts.push(html[start..].to_owned());
    Ok(CellTableSplit { texts, tables })
}

#[derive(Debug)]
struct TagToken {
    start: usize,
    end: usize,
    name: String,
    attrs: String,
    closing: bool,
    self_closing: bool,
}

#[derive(Debug)]
struct OpenTag {
    name: String,
}

fn next_tag(input: &str, from: usize) -> Result<Option<TagToken>, KordocError> {
    let bytes = input.as_bytes();
    let mut start = from;
    loop {
        while start < bytes.len() && bytes[start] != b'<' {
            start += 1;
        }
        if start == bytes.len() {
            return Ok(None);
        }
        if input[start..].starts_with("<!--") {
            let Some(end) = input[start + 4..].find("-->") else {
                return Err(malformed_html());
            };
            start += 4 + end + 3;
            continue;
        }
        let mut end = start + 1;
        let mut quote = None;
        while end < bytes.len() {
            let byte = bytes[end];
            match (quote, byte) {
                (Some(q), b) if q == b => quote = None,
                (None, b'\'' | b'"') => quote = Some(byte),
                (None, b'>') => break,
                _ => {}
            }
            end += 1;
        }
        if end == bytes.len() || quote.is_some() {
            return Err(malformed_html());
        }
        let body = input[start + 1..end].trim();
        if body.is_empty() || body.starts_with('!') || body.starts_with('?') {
            start = end + 1;
            continue;
        }
        let closing = body.starts_with('/');
        let body = body.strip_prefix('/').unwrap_or(body).trim_start();
        let name_end = body
            .find(|ch: char| ch.is_ascii_whitespace() || ch == '/')
            .unwrap_or(body.len());
        let name = body[..name_end].to_ascii_lowercase();
        let attrs = body[name_end..]
            .trim()
            .trim_end_matches('/')
            .trim()
            .to_owned();
        if name.is_empty() || !name.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-') {
            return Err(malformed_html());
        }
        let self_closing = !closing && body.trim_end().ends_with('/');
        return Ok(Some(TagToken {
            start,
            end: end + 1,
            name,
            attrs,
            closing,
            self_closing,
        }));
    }
}

fn pop_expected(open: &mut Vec<OpenTag>, name: &str) -> Result<(), KordocError> {
    if open.last().is_some_and(|tag| tag.name == name) {
        open.pop();
        Ok(())
    } else {
        Err(malformed_html())
    }
}

fn parse_spans(attrs: &str) -> Result<(u32, u32), KordocError> {
    let mut col_span = 1u32;
    let mut row_span = 1u32;
    let mut cursor = 0usize;
    while cursor < attrs.len() {
        while cursor < attrs.len()
            && (attrs.as_bytes()[cursor].is_ascii_whitespace() || attrs.as_bytes()[cursor] == b'/')
        {
            cursor += 1;
        }
        if cursor >= attrs.len() {
            break;
        }
        let name_start = cursor;
        while cursor < attrs.len()
            && (attrs.as_bytes()[cursor].is_ascii_alphanumeric()
                || attrs.as_bytes()[cursor] == b'-'
                || attrs.as_bytes()[cursor] == b'_')
        {
            cursor += 1;
        }
        if name_start == cursor {
            return Err(malformed_html());
        }
        let name = attrs[name_start..cursor].to_ascii_lowercase();
        while cursor < attrs.len() && attrs.as_bytes()[cursor].is_ascii_whitespace() {
            cursor += 1;
        }
        if cursor >= attrs.len() || attrs.as_bytes()[cursor] != b'=' {
            continue;
        }
        cursor += 1;
        while cursor < attrs.len() && attrs.as_bytes()[cursor].is_ascii_whitespace() {
            cursor += 1;
        }
        if cursor >= attrs.len() {
            return Err(malformed_html());
        }
        let value = if matches!(attrs.as_bytes()[cursor], b'\'' | b'"') {
            let quote = attrs.as_bytes()[cursor];
            cursor += 1;
            let value_start = cursor;
            while cursor < attrs.len() && attrs.as_bytes()[cursor] != quote {
                cursor += 1;
            }
            if cursor == attrs.len() {
                return Err(malformed_html());
            }
            let value = &attrs[value_start..cursor];
            cursor += 1;
            value
        } else {
            let value_start = cursor;
            while cursor < attrs.len() && !attrs.as_bytes()[cursor].is_ascii_whitespace() {
                cursor += 1;
            }
            &attrs[value_start..cursor]
        };
        if name == "colspan" || name == "rowspan" {
            let parsed = value.parse::<u64>().map_err(|_| malformed_html())?;
            if parsed == 0 {
                return Err(malformed_html());
            }
            let maximum = if name == "colspan" {
                MAX_TABLE_COLUMNS
            } else {
                MAX_TABLE_ROWS
            };
            if parsed > maximum as u64 {
                return Err(output_too_large(if name == "colspan" {
                    "HTML table cell colspan exceeds 200 columns"
                } else {
                    "HTML table cell rowspan exceeds 10,000 rows"
                }));
            }
            let parsed = parsed as u32;
            if name == "colspan" {
                col_span = parsed;
            } else {
                row_span = parsed;
            }
        }
    }
    Ok((col_span, row_span))
}

fn scan_table_depth(
    line: &str,
    depth: &mut usize,
    found_root: &mut bool,
) -> Result<(), KordocError> {
    let mut cursor = 0usize;
    while let Some(tag) = next_tag(line, cursor)? {
        cursor = tag.end;
        if tag.name != "table" {
            continue;
        }
        if tag.closing {
            *depth = depth.checked_sub(1).ok_or_else(malformed_html)?;
        } else {
            *found_root = true;
            *depth += 1;
            if *depth > MAX_HTML_DEPTH {
                return Err(output_too_large("HTML table nesting exceeds 64 levels"));
            }
            if tag.self_closing {
                *depth -= 1;
            }
        }
    }
    Ok(())
}

fn starts_tag(input: &str, name: &str) -> bool {
    let rest = input.strip_prefix('<').unwrap_or("");
    rest.get(..name.len())
        .is_some_and(|candidate| candidate.eq_ignore_ascii_case(name))
        && rest
            .as_bytes()
            .get(name.len())
            .is_some_and(|b| b.is_ascii_whitespace() || *b == b'>' || *b == b'/')
}

fn is_escaped_pipe(bytes: &[u8], at: usize) -> bool {
    at > 0 && bytes[at - 1] == b'\\'
}

fn is_gfm_delimiter(cell: &str) -> bool {
    let trimmed = cell.trim();
    let inner = trimmed.strip_prefix(':').unwrap_or(trimmed);
    let inner = inner.strip_suffix(':').unwrap_or(inner);
    inner.len() >= 3 && inner.bytes().all(|byte| byte == b'-')
}

fn is_separator(line: &str) -> bool {
    let trimmed = line.trim();
    trimmed.len() >= 3 && trimmed.bytes().all(|byte| byte == b'-')
}

fn is_standalone_image(line: &str) -> bool {
    let Some(rest) = line.strip_prefix("![image](") else {
        return false;
    };
    rest.find(')')
        .is_some_and(|close| rest[close + 1..].trim().is_empty())
}

fn split_br_tags(input: &str) -> Result<Vec<&str>, KordocError> {
    let mut parts = Vec::new();
    let mut start = 0usize;
    let mut cursor = 0usize;
    while let Some(tag) = next_tag(input, cursor)? {
        cursor = tag.end;
        if tag.name == "br" && !tag.closing {
            parts.push(&input[start..tag.start]);
            start = tag.end;
        }
    }
    parts.push(&input[start..]);
    Ok(parts)
}

fn remove_images(input: &str) -> Result<(String, Vec<String>, bool), KordocError> {
    let mut output = String::new();
    let mut image_sources = Vec::new();
    let mut found = false;
    let mut last = 0usize;
    let mut cursor = 0usize;
    while let Some(tag) = next_tag(input, cursor)? {
        cursor = tag.end;
        if tag.name == "img" && !tag.closing {
            found = true;
            output.push_str(&input[last..tag.start]);
            if let Some(src) = attribute_value(&tag.attrs, "src")? {
                image_sources.push(decode_html_entities(src)?);
            }
            last = tag.end;
        }
    }
    output.push_str(&input[last..]);
    Ok((output, image_sources, found))
}

// These helpers are intentionally tiny; HTML is treated as inert text, never loaded or evaluated.
fn attribute_value<'a>(attrs: &'a str, sought: &str) -> Result<Option<&'a str>, KordocError> {
    let mut cursor = 0usize;
    while cursor < attrs.len() {
        while cursor < attrs.len()
            && (attrs.as_bytes()[cursor].is_ascii_whitespace() || attrs.as_bytes()[cursor] == b'/')
        {
            cursor += 1;
        }
        if cursor == attrs.len() {
            break;
        }
        let start = cursor;
        while cursor < attrs.len()
            && (attrs.as_bytes()[cursor].is_ascii_alphanumeric()
                || attrs.as_bytes()[cursor] == b'-'
                || attrs.as_bytes()[cursor] == b'_')
        {
            cursor += 1;
        }
        if start == cursor {
            return Err(malformed_html());
        }
        let name = &attrs[start..cursor];
        while cursor < attrs.len() && attrs.as_bytes()[cursor].is_ascii_whitespace() {
            cursor += 1;
        }
        if cursor == attrs.len() || attrs.as_bytes()[cursor] != b'=' {
            continue;
        }
        cursor += 1;
        while cursor < attrs.len() && attrs.as_bytes()[cursor].is_ascii_whitespace() {
            cursor += 1;
        }
        if cursor == attrs.len() {
            return Err(malformed_html());
        }
        let value = if matches!(attrs.as_bytes()[cursor], b'\'' | b'"') {
            let quote = attrs.as_bytes()[cursor];
            cursor += 1;
            let value_start = cursor;
            while cursor < attrs.len() && attrs.as_bytes()[cursor] != quote {
                cursor += 1;
            }
            if cursor == attrs.len() {
                return Err(malformed_html());
            }
            let value = &attrs[value_start..cursor];
            cursor += 1;
            value
        } else {
            let value_start = cursor;
            while cursor < attrs.len() && !attrs.as_bytes()[cursor].is_ascii_whitespace() {
                cursor += 1;
            }
            &attrs[value_start..cursor]
        };
        if name.eq_ignore_ascii_case(sought) {
            return Ok(Some(value));
        }
    }
    Ok(None)
}

fn decode_html_entities(input: &str) -> Result<String, KordocError> {
    let mut output = String::with_capacity(input.len());
    let mut cursor = 0usize;
    let mut copied_through = 0usize;
    let bytes = input.as_bytes();
    while cursor < bytes.len() {
        if bytes[cursor] != b'&' {
            cursor += 1;
            continue;
        }
        output.push_str(&input[copied_through..cursor]);
        let Some(semi) = entity_terminator_within_limit(bytes, cursor) else {
            output.push('&');
            cursor += 1;
            copied_through = cursor;
            continue;
        };
        let entity = &input[cursor + 1..semi];
        let decoded = match entity {
            "amp" => Some('&'),
            "lt" => Some('<'),
            "gt" => Some('>'),
            "quot" => Some('"'),
            "apos" | "#39" => Some('\''),
            "nbsp" => Some('\u{a0}'),
            _ if entity.starts_with("#x") || entity.starts_with("#X") => {
                u32::from_str_radix(&entity[2..], 16)
                    .ok()
                    .and_then(char::from_u32)
            }
            _ if entity.starts_with('#') => {
                entity[1..].parse::<u32>().ok().and_then(char::from_u32)
            }
            _ => None,
        };
        if let Some(ch) = decoded {
            output.push(ch);
        } else {
            output.push_str(&input[cursor..=semi]);
        }
        cursor = semi + 1;
        copied_through = cursor;
    }
    output.push_str(&input[copied_through..]);
    Ok(output)
}

fn entity_terminator_within_limit(input: &[u8], ampersand: usize) -> Option<usize> {
    if input.get(ampersand) != Some(&b'&') {
        return None;
    }
    let start = ampersand.checked_add(1)?;
    let end = start.checked_add(MAX_ENTITY_LOOKAHEAD)?.min(input.len());
    input[start..end]
        .iter()
        .position(|byte| *byte == b';')
        .map(|relative| start + relative)
}

fn ensure_html_size(input: &str) -> Result<(), KordocError> {
    if input.len() > MAX_HTML_TABLE_BYTES {
        Err(output_too_large("HTML table or cell exceeds 8 MiB"))
    } else {
        Ok(())
    }
}
fn malformed_html() -> KordocError {
    KordocError::new(ErrorCode::ParseError, "Malformed Markdown HTML table")
}
fn output_too_large(message: &str) -> KordocError {
    KordocError::new(ErrorCode::OutputTooLarge, message)
}

#[cfg(test)]
mod tests {
    use super::{
        MarkdownUnitKind, entity_terminator_within_limit, html_cell_inner_to_lines,
        parse_gfm_table, parse_html_table, split_cell_by_top_level_tables,
        split_gfm_row_with_limit, split_markdown_units, split_markdown_units_with_budget,
    };
    use kordoc_ir::ErrorCode;

    #[test]
    fn units_split_text_gfm_html_image_separator() {
        let units = split_markdown_units(
            "intro\nline\n\n| A | B |\n| --- | --- |\n| x | y |\n\n<table>\n<tr><td>z</td></tr>\n</table>\n\n---\n\n![image](asset.png)",
        )
        .expect("valid markdown units");

        assert_eq!(
            units.iter().map(|unit| unit.kind).collect::<Vec<_>>(),
            [
                MarkdownUnitKind::Text,
                MarkdownUnitKind::GfmTable,
                MarkdownUnitKind::HtmlTable,
                MarkdownUnitKind::Separator,
                MarkdownUnitKind::Image,
            ]
        );
        assert_eq!(units[0].raw, "intro\nline");
        assert_eq!(units[2].lines.len(), 3);
    }

    #[test]
    fn gfm_cells_preserve_escaped_pipe_and_br() {
        let rows = parse_gfm_table(&[
            "| left\\|right | two<br>lines |".into(),
            "| :--- | ---: |".into(),
            "| - | value |".into(),
        ])
        .expect("valid GFM table");

        assert_eq!(
            rows,
            [vec!["left\\|right", "two<br>lines"], vec!["-", "value"]]
        );
    }

    #[test]
    fn gfm_pipe_is_escaped_after_any_immediate_backslash() {
        let rows = parse_gfm_table(&["| left\\\\|right | tail |".into()]).unwrap();

        assert_eq!(rows, [vec!["left\\\\|right", "tail"]]);
    }

    #[test]
    fn markdown_unit_input_and_aggregate_output_obey_byte_budgets() {
        assert_eq!(
            split_markdown_units_with_budget("abcd", 3, 100)
                .unwrap_err()
                .code,
            ErrorCode::OutputTooLarge
        );
        // The output owns both the raw unit and its line, so one input byte costs two.
        assert_eq!(
            split_markdown_units_with_budget("x", 10, 1)
                .unwrap_err()
                .code,
            ErrorCode::OutputTooLarge
        );
        assert_eq!(
            split_markdown_units_with_budget("x", 10, 2).unwrap().len(),
            1
        );
    }

    #[test]
    fn entity_terminator_search_is_bounded_to_supported_length() {
        assert_eq!(entity_terminator_within_limit(b"&amp;", 0), Some(4));
        assert_eq!(
            entity_terminator_within_limit(b"&1234567890123456;", 0),
            None
        );
    }

    #[test]
    fn html_rows_preserve_nested_table_order_and_spans() {
        let rows = parse_html_table(
            "<table><tr><th colspan='2'>head</th></tr><tr><td>A<table><tr><td>inner</td></tr></table>B</td><td rowspan=2>tail</td></tr><tr><td>end</td></tr></table>",
        )
        .expect("valid nested table");

        assert_eq!(rows.len(), 3);
        assert_eq!(rows[0].cells[0].col_span, 2);
        assert_eq!(
            rows[1].cells[0].inner,
            "A<table><tr><td>inner</td></tr></table>B"
        );
        assert_eq!(rows[1].cells[1].row_span, 2);
        assert_eq!(rows[2].cells[0].inner, "end");
    }

    #[test]
    fn html_cell_lines_preserve_image_presence() {
        let parsed = html_cell_inner_to_lines(
            "first<br/>second &amp; third<img src='asset&amp;1.png'><table><tr><td>nested</td></tr></table>",
        )
        .expect("valid cell HTML");

        assert_eq!(parsed.lines, ["first", "second & third"]);
        assert!(parsed.had_non_text);
        assert_eq!(parsed.img_srcs, ["asset&1.png"]);
    }

    #[test]
    fn html_rejects_unbalanced_or_oversized_input() {
        let unbalanced = parse_html_table("<table><tr><td>lost</table>");
        assert_eq!(unbalanced.unwrap_err().code, ErrorCode::ParseError);

        let oversized = format!("<table>{}</table>", "x".repeat(8 * 1024 * 1024));
        let error = parse_html_table(&oversized).unwrap_err();
        assert_eq!(error.code, ErrorCode::OutputTooLarge);
    }

    #[test]
    fn html_does_not_execute_or_fetch() {
        let rows = parse_html_table(
            "<table><tr><td><script>not executed</script><img src='https://invalid.test/pixel'></td></tr></table>",
        )
        .expect("markup is parsed as inert text");
        assert!(rows[0].cells[0].inner.contains("not executed"));
        let cell = html_cell_inner_to_lines(&rows[0].cells[0].inner).unwrap();
        assert_eq!(cell.img_srcs, ["https://invalid.test/pixel"]);
    }

    #[test]
    fn html_rejects_mismatched_closes_and_adversarial_depth() {
        assert_eq!(
            parse_html_table("<table><tr><td>x</th></tr></table>")
                .unwrap_err()
                .code,
            ErrorCode::ParseError
        );
        let deep = format!("{}x{}", "<table>".repeat(65), "</table>".repeat(65));
        assert_eq!(
            parse_html_table(&deep).unwrap_err().code,
            ErrorCode::OutputTooLarge
        );
    }

    #[test]
    fn html_and_gfm_tables_enforce_cell_limits() {
        let html_cells = (0..201).map(|_| "<td>x</td>").collect::<String>();
        let html = format!("<table><tr>{html_cells}</tr></table>");
        assert_eq!(
            parse_html_table(&html).unwrap_err().code,
            ErrorCode::OutputTooLarge
        );

        let gfm_cells = (0..201).map(|_| "x").collect::<Vec<_>>().join(" | ");
        let gfm = vec![format!("| {gfm_cells} |")];
        assert_eq!(
            parse_gfm_table(&gfm).unwrap_err().code,
            ErrorCode::OutputTooLarge
        );
    }

    #[test]
    fn gfm_row_stops_at_column_budget_before_scanning_tail() {
        let mut row = String::from("| a | b | c |");
        row.push_str(&" unused |".repeat(100_000));
        assert_eq!(
            split_gfm_row_with_limit(&row, 2).unwrap_err().code,
            ErrorCode::OutputTooLarge
        );
    }

    #[test]
    fn html_rejects_spans_above_configured_maxima() {
        for html in [
            "<table><tr><td colspan=201>x</td></tr></table>",
            "<table><tr><td rowspan=10001>x</td></tr></table>",
        ] {
            assert_eq!(
                parse_html_table(html).unwrap_err().code,
                ErrorCode::OutputTooLarge
            );
        }
    }

    #[test]
    fn cell_table_split_preserves_interleaved_text_order() {
        let split = split_cell_by_top_level_tables(
            "before<table><tr><td>nested<table><tr><td>deep</td></tr></table></td></tr></table>middle<table><tr><td>last</td></tr></table>after",
        )
        .expect("well-formed cell HTML");

        assert_eq!(split.texts, ["before", "middle", "after"]);
        assert_eq!(split.tables.len(), 2);
        assert!(split.tables[0].contains("deep"));
    }
}
