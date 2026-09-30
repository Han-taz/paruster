use kordoc_ir::{ErrorCode, IrBlock, KordocError, ParseOptions, ParseSuccess};
use regex::{Regex, RegexBuilder};
use std::sync::OnceLock;

pub(crate) fn apply(result: &mut ParseSuccess, options: &ParseOptions) -> Result<(), KordocError> {
    if options.script_tags == Some(false) {
        strip_script_tags(&mut result.markdown);
        if let Some(pages) = &mut result.pages {
            for page in pages {
                strip_script_tags(&mut page.markdown);
            }
        }
        strip_block_scripts(&mut result.blocks, 0)?;
    }
    if options.plain == Some(true) {
        transform_result_markdown(result, true, false)?;
    }
    if options.html_tables == Some(true) {
        transform_result_markdown(result, false, true)?;
    }
    Ok(())
}

fn transform_result_markdown(
    result: &mut ParseSuccess,
    plain: bool,
    html_tables: bool,
) -> Result<(), KordocError> {
    transform_text(&mut result.markdown, plain, html_tables)?;
    if let Some(pages) = &mut result.pages {
        for page in pages {
            transform_text(&mut page.markdown, plain, html_tables)?;
        }
    }
    Ok(())
}

fn transform_text(text: &mut String, plain: bool, html_tables: bool) -> Result<(), KordocError> {
    if text.len() > crate::markdown::MAX_MARKDOWN_BYTES {
        return Err(output_too_large());
    }
    let source = std::mem::take(text);
    let mut next = if plain {
        plain_markdown(&source, crate::markdown::MAX_MARKDOWN_BYTES)?
    } else {
        source
    };
    if html_tables {
        next = transform_html_tables(&next, crate::markdown::MAX_MARKDOWN_BYTES)?;
    }
    *text = next;
    Ok(())
}

fn output_too_large() -> KordocError {
    KordocError::new(
        ErrorCode::OutputTooLarge,
        "Postprocessed Markdown exceeds the output limit",
    )
}

struct BoundedText {
    bytes: Vec<u8>,
    limit: usize,
}

impl BoundedText {
    fn new(input: &str, limit: usize) -> Result<Self, KordocError> {
        if input.len() > limit {
            return Err(output_too_large());
        }
        Ok(Self {
            bytes: Vec::with_capacity(input.len().min(limit)),
            limit,
        })
    }
    fn append(&mut self, value: &str) -> Result<(), KordocError> {
        if self
            .bytes
            .len()
            .checked_add(value.len())
            .is_none_or(|len| len > self.limit)
        {
            return Err(output_too_large());
        }
        self.bytes.extend_from_slice(value.as_bytes());
        Ok(())
    }
    fn byte(&mut self, value: u8) -> Result<(), KordocError> {
        if self.bytes.len() >= self.limit {
            return Err(output_too_large());
        }
        self.bytes.push(value);
        Ok(())
    }
    fn finish(self) -> String {
        String::from_utf8(self.bytes).expect("postprocessing preserves UTF-8")
    }
}

fn plain_markdown(input: &str, limit: usize) -> Result<String, KordocError> {
    let mut current = plain_scripts(input, limit)?;
    current = remove_image_markdown(&current, limit)?;
    current = remove_img_tags(&current, limit)?;
    current = unwrap_links(&current, limit)?;
    current = remove_exact_tags(&current, "<u>", "</u>", limit)?;
    current = unwrap_bold(&current, limit)?;
    current = remove_table_breaks(&current, limit)?;
    current = clean_whitespace(&current, limit)?;
    Ok(current.trim().to_owned())
}

fn plain_scripts(input: &str, limit: usize) -> Result<String, KordocError> {
    let simple =
        SCRIPT_SIMPLE.get_or_init(|| Regex::new(r"^(?:[+\-−]?[\p{L}\p{N}]+|[*∗†‡§¶]+)$").unwrap());
    let pairs = SCRIPT_TAG_PAIR
        .get_or_init(|| Regex::new(r"<sup>([^<\n]*)</sup>|<sub>([^<\n]*)</sub>").unwrap());
    let mut out = BoundedText::new(input, limit)?;
    let mut copied = 0;
    for captures in pairs.captures_iter(input) {
        let matched = captures.get(0).unwrap();
        out.append(&input[copied..matched.start()])?;
        let (kind, body) = if let Some(body) = captures.get(1) {
            ("^", body.as_str())
        } else {
            ("_", captures.get(2).unwrap().as_str())
        };
        let mark = kind;
        out.append(mark)?;
        if simple.is_match(body) {
            out.append(body)?;
        } else {
            out.append("(")?;
            out.append(body)?;
            out.append(")")?;
        }
        copied = matched.end();
    }
    out.append(&input[copied..])?;
    Ok(out.finish())
}

static SCRIPT_SIMPLE: OnceLock<Regex> = OnceLock::new();
static SCRIPT_TAG_PAIR: OnceLock<Regex> = OnceLock::new();
static IMAGE_MARKDOWN: OnceLock<Regex> = OnceLock::new();
static IMAGE_TAG: OnceLock<Regex> = OnceLock::new();
static LINK_MARKDOWN: OnceLock<Regex> = OnceLock::new();
static TABLE_BREAKS: OnceLock<Regex> = OnceLock::new();
static TABLE_TAG: OnceLock<Regex> = OnceLock::new();

fn remove_image_markdown(input: &str, limit: usize) -> Result<String, KordocError> {
    remove_matches(
        input,
        limit,
        IMAGE_MARKDOWN.get_or_init(|| Regex::new(r"!\[[^\]\n]*\]\([^\)\n]*\)").unwrap()),
    )
}

fn remove_img_tags(input: &str, limit: usize) -> Result<String, KordocError> {
    remove_matches(
        input,
        limit,
        IMAGE_TAG.get_or_init(|| {
            RegexBuilder::new(r"<img\b[^>]*>")
                .case_insensitive(true)
                .build()
                .unwrap()
        }),
    )
}

fn remove_matches(input: &str, limit: usize, regex: &Regex) -> Result<String, KordocError> {
    let mut out = BoundedText::new(input, limit)?;
    let mut copied = 0;
    for m in regex.find_iter(input) {
        out.append(&input[copied..m.start()])?;
        copied = m.end();
    }
    out.append(&input[copied..])?;
    Ok(out.finish())
}

fn unwrap_links(input: &str, limit: usize) -> Result<String, KordocError> {
    let mut out = BoundedText::new(input, limit)?;
    let mut copied = 0;
    let re = LINK_MARKDOWN
        .get_or_init(|| Regex::new(r"\[[^\]\n]*\]\((?:https?|mailto|ftp):[^)\s]*\)").unwrap());
    for m in re.find_iter(input) {
        if m.start() > 0 && input.as_bytes()[m.start() - 1] == b'!' {
            continue;
        }
        out.append(&input[copied..m.start()])?;
        let label_end = input[m.start()..m.end()].find("](").unwrap() + m.start();
        out.append(&input[m.start() + 1..label_end])?;
        copied = m.end();
    }
    out.append(&input[copied..])?;
    Ok(out.finish())
}

fn remove_exact_tags(
    input: &str,
    opening: &str,
    closing: &str,
    limit: usize,
) -> Result<String, KordocError> {
    let mut out = BoundedText::new(input, limit)?;
    let mut pos = 0;
    let mut copied = 0;
    while pos < input.len() {
        let rest = &input[pos..];
        let tag = if rest.starts_with(opening) {
            Some(opening)
        } else if rest.starts_with(closing) {
            Some(closing)
        } else {
            None
        };
        if let Some(tag) = tag {
            out.append(&input[copied..pos])?;
            pos += tag.len();
            copied = pos;
        } else {
            pos += rest.chars().next().unwrap().len_utf8();
        }
    }
    out.append(&input[copied..])?;
    Ok(out.finish())
}

fn unwrap_bold(input: &str, limit: usize) -> Result<String, KordocError> {
    let b = input.as_bytes();
    let mut out = BoundedText::new(input, limit)?;
    let mut copied = 0;
    let mut pending = None;
    let mut pos = 0;
    while pos + 1 < b.len() {
        if b[pos] == b'\n' {
            pending = None;
            pos += 1;
            continue;
        }
        if b[pos..].starts_with(b"**") {
            let escaped = pos > 0 && b[pos - 1] == b'\\';
            let prev_nonspace =
                pos > 0 && !input[..pos].chars().next_back().unwrap().is_whitespace();
            let next_nonspace =
                pos + 2 < b.len() && !input[pos + 2..].chars().next().unwrap().is_whitespace();
            if !escaped {
                if let Some(open) = pending.filter(|_| prev_nonspace) {
                    out.append(&input[copied..open])?;
                    out.append(&input[open + 2..pos])?;
                    copied = pos + 2;
                    pending = None;
                    pos += 2;
                    continue;
                }
                if next_nonspace && pending.is_none() {
                    pending = Some(pos);
                }
            }
            pos += 2;
            continue;
        }
        pos += input[pos..].chars().next().unwrap().len_utf8();
    }
    out.append(&input[copied..])?;
    Ok(out.finish())
}

fn remove_table_breaks(input: &str, limit: usize) -> Result<String, KordocError> {
    let re = TABLE_BREAKS.get_or_init(|| Regex::new(r"(?:<br>)+(</t[dh]>)").unwrap());
    let mut out = BoundedText::new(input, limit)?;
    let mut copied = 0;
    for c in re.captures_iter(input) {
        let m = c.get(0).unwrap();
        out.append(&input[copied..m.start()])?;
        out.append(c.get(1).unwrap().as_str())?;
        copied = m.end();
    }
    out.append(&input[copied..])?;
    Ok(out.finish())
}

fn clean_whitespace(input: &str, limit: usize) -> Result<String, KordocError> {
    let mut out = BoundedText::new(input, limit)?;
    let bytes = input.as_bytes();
    let mut pos = 0;
    while pos < bytes.len() {
        if bytes[pos] == b' ' || bytes[pos] == b'\t' {
            let start = pos;
            while pos < bytes.len() && (bytes[pos] == b' ' || bytes[pos] == b'\t') {
                pos += 1;
            }
            if pos == bytes.len() || bytes[pos] == b'\n' {
                continue;
            }
            out.append(&input[start..pos])?;
            continue;
        }
        if bytes[pos] == b'\n' {
            let start = pos;
            while pos < bytes.len() && bytes[pos] == b'\n' {
                pos += 1;
            }
            out.append(if pos - start >= 3 {
                "\n\n"
            } else {
                &input[start..pos]
            })?;
            continue;
        }
        let c = input[pos..].chars().next().unwrap();
        out.append(&input[pos..pos + c.len_utf8()])?;
        pos += c.len_utf8();
    }
    Ok(out.finish())
}

fn transform_html_tables(input: &str, limit: usize) -> Result<String, KordocError> {
    if input.len() > limit {
        return Err(output_too_large());
    }
    let mut line_out = BoundedText::new(input, limit)?;
    let mut pos = 0;
    let mut first = true;
    while pos <= input.len() {
        let (line, next, _) = line_at(input, pos);
        if !first {
            line_out.byte(b'\n')?;
        }
        first = false;
        if next <= input.len() && is_pipe_row(line) {
            let (separator, body_start, _) = line_at(input, next);
            if is_separator(separator) {
                pos = append_pipe_table(input, line, body_start, &mut line_out)?;
                continue;
            }
        }
        line_out.append(line)?;
        pos = next;
    }
    pretty_html_table_blocks(&line_out.finish(), limit)
}

fn line_at(input: &str, start: usize) -> (&str, usize, bool) {
    debug_assert!(start <= input.len());
    if let Some(relative) = input[start..].find('\n') {
        let end = start + relative;
        (&input[start..end], end + 1, true)
    } else {
        (&input[start..], input.len() + 1, false)
    }
}

fn is_pipe_row(line: &str) -> bool {
    let t = line.trim();
    t.starts_with('|') && t.ends_with('|')
}
fn is_separator(line: &str) -> bool {
    let t = line.trim();
    if !t.starts_with('|') || !t.ends_with('|') {
        return false;
    }
    let inner = &t[1..t.len() - 1];
    !inner.is_empty()
        && inner.split('|').all(|cell| {
            let c = cell.trim();
            let c = c.strip_prefix(':').unwrap_or(c);
            let c = c.strip_suffix(':').unwrap_or(c);
            c.len() >= 3 && c.bytes().all(|b| b == b'-')
        })
}

fn split_pipe_row(line: &str) -> impl Iterator<Item = &str> {
    let t = line.trim();
    let t = t.strip_prefix('|').unwrap_or(t);
    let t = if t.ends_with('|') && !t[..t.len() - 1].ends_with('\\') {
        &t[..t.len() - 1]
    } else {
        t
    };
    let mut start = 0;
    let mut finished = false;
    std::iter::from_fn(move || {
        if finished {
            return None;
        }
        let bytes = t.as_bytes();
        let scan_from = start;
        for i in scan_from..bytes.len() {
            if bytes[i] == b'|' && (i == 0 || bytes[i - 1] != b'\\') {
                let cell = t[start..i].trim();
                start = i + 1;
                return Some(cell);
            }
        }
        finished = true;
        Some(t[start..].trim())
    })
}

fn append_pipe_table(
    input: &str,
    header: &str,
    mut pos: usize,
    out: &mut BoundedText,
) -> Result<usize, KordocError> {
    out.append("<table>")?;
    append_pipe_row(header, "th", out)?;
    while pos <= input.len() {
        let (line, next, _) = line_at(input, pos);
        if !is_pipe_row(line) {
            break;
        }
        append_pipe_row(line, "td", out)?;
        pos = next;
    }
    out.append("</table>")?;
    Ok(pos)
}

fn append_pipe_row(line: &str, tag: &str, out: &mut BoundedText) -> Result<(), KordocError> {
    out.append("<tr>")?;
    for cell in split_pipe_row(line) {
        out.append("<")?;
        out.append(tag)?;
        out.append(">")?;
        pipe_cell(cell, out)?;
        out.append("</")?;
        out.append(tag)?;
        out.append(">")?;
    }
    out.append("</tr>")
}

fn special_inline_tag(s: &str) -> Option<usize> {
    if s.starts_with("<br>") {
        return Some(4);
    }
    for t in ["<u>", "</u>", "<sup>", "</sup>", "<sub>", "</sub>"] {
        if s.starts_with(t) {
            return Some(t.len());
        }
    }
    if s.starts_with("<img")
        && s.as_bytes()
            .get(4)
            .is_some_and(|b| !b.is_ascii_alphanumeric() && *b != b'_')
    {
        return s.find('>').map(|i| i + 1);
    }
    None
}
fn pipe_cell(cell: &str, out: &mut BoundedText) -> Result<(), KordocError> {
    let mut pos = 0;
    while pos < cell.len() {
        if let Some(n) = special_inline_tag(&cell[pos..]) {
            out.append(&cell[pos..pos + n])?;
            pos += n;
            continue;
        }
        let b = cell.as_bytes();
        if b[pos] == b'\\'
            && b.get(pos + 1).is_some_and(|c| {
                matches!(
                    *c,
                    b'\\'
                        | b'`'
                        | b'*'
                        | b'_'
                        | b'{'
                        | b'}'
                        | b'['
                        | b']'
                        | b'('
                        | b')'
                        | b'#'
                        | b'+'
                        | b'-'
                        | b'.'
                        | b'!'
                        | b'|'
                        | b'~'
                        | b'<'
                        | b'>'
                        | b'$'
                )
            })
        {
            pos += 1;
        }
        let c = cell[pos..].chars().next().unwrap();
        match c {
            '&' => out.append("&amp;")?,
            '<' => out.append("&lt;")?,
            '>' => out.append("&gt;")?,
            _ => out.append(&cell[pos..pos + c.len_utf8()])?,
        }
        pos += c.len_utf8();
    }
    Ok(())
}

fn pretty_html_table_blocks(input: &str, limit: usize) -> Result<String, KordocError> {
    let mut out = BoundedText {
        bytes: Vec::with_capacity(input.len().min(limit)),
        limit,
    };
    let mut first = true;
    let mut pos = 0;
    while pos <= input.len() {
        let (line, next, _has_newline) = line_at(input, pos);
        if !first {
            out.byte(b'\n')?;
        }
        first = false;
        if !starts_table_tag(line.trim_start()) {
            out.append(line)?;
            pos = next;
            continue;
        }
        let mut depth = 0i32;
        let mut gathered = String::new();
        let mut cursor = pos;
        loop {
            let (table_line, after, line_has_newline) = line_at(input, cursor);
            append_bounded_string(&mut gathered, table_line, limit)?;
            if line_has_newline {
                append_bounded_string(&mut gathered, "\n", limit)?;
            }
            depth += table_balance(table_line);
            cursor = after;
            if depth <= 0 || after > input.len() {
                break;
            }
        }
        let pretty = pretty_table(&gathered, limit)?;
        out.append(&pretty)?;
        pos = cursor;
    }
    Ok(out.finish())
}

fn starts_table_tag(line: &str) -> bool {
    let b = line.as_bytes();
    b.len() >= 6
        && b[..6].eq_ignore_ascii_case(b"<table")
        && b.get(6)
            .is_some_and(|c| !c.is_ascii_alphanumeric() && *c != b'_')
}

fn append_bounded_string(out: &mut String, s: &str, limit: usize) -> Result<(), KordocError> {
    if out.len().checked_add(s.len()).is_none_or(|n| n > limit) {
        return Err(output_too_large());
    }
    out.push_str(s);
    Ok(())
}
fn table_balance(line: &str) -> i32 {
    let b = line.as_bytes();
    let mut i = 0;
    let mut balance = 0;
    while i < b.len() {
        if b[i] == b'<' {
            let tail = &line[i..];
            if tail.len() >= 6
                && tail.as_bytes()[..6].eq_ignore_ascii_case(b"<table")
                && tail
                    .as_bytes()
                    .get(6)
                    .is_some_and(|x| !x.is_ascii_alphanumeric() && *x != b'_')
            {
                balance += 1;
            } else if tail.len() >= 7
                && tail.as_bytes()[..7].eq_ignore_ascii_case(b"</table")
                && tail
                    .as_bytes()
                    .get(7)
                    .is_some_and(|x| !x.is_ascii_alphanumeric() && *x != b'_')
            {
                balance -= 1;
            }
        }
        i += line[i..].chars().next().unwrap().len_utf8();
    }
    balance
}

fn pretty_table(input: &str, limit: usize) -> Result<String, KordocError> {
    let mut out = BoundedText {
        bytes: Vec::with_capacity(input.len().min(limit)),
        limit,
    };
    let mut depth = 0usize;
    let mut pos = 0;
    let mut first = true;
    while pos < input.len() {
        if let Some((start, end, closing)) = next_table_tag(input, pos) {
            append_pretty_text(&mut out, &input[pos..start], depth, &mut first)?;
            if closing {
                depth = depth.saturating_sub(1);
            }
            emit_pretty_line(&mut out, &input[start..end], depth, &mut first)?;
            if !closing {
                depth += 1;
                if depth > 64 {
                    return Err(output_too_large());
                }
            }
            pos = end;
        } else {
            append_pretty_text(&mut out, &input[pos..], depth, &mut first)?;
            break;
        }
    }
    Ok(out.finish())
}

fn next_table_tag(input: &str, from: usize) -> Option<(usize, usize, bool)> {
    let re = TABLE_TAG.get_or_init(|| {
        RegexBuilder::new(r"</?(?:table|tr|th|td)\b[^>]*>")
            .case_insensitive(true)
            .build()
            .unwrap()
    });
    re.find_at(input, from).map(|m| {
        (
            m.start(),
            m.end(),
            input.as_bytes().get(m.start() + 1) == Some(&b'/'),
        )
    })
}

fn append_pretty_text(
    out: &mut BoundedText,
    text: &str,
    depth: usize,
    first: &mut bool,
) -> Result<(), KordocError> {
    let mut collapsed = String::new();
    let mut chars = text.chars().peekable();
    while let Some(ch) = chars.next() {
        if ch == '\n' {
            while collapsed
                .chars()
                .next_back()
                .is_some_and(char::is_whitespace)
            {
                collapsed.pop();
            }
            while chars.peek().is_some_and(|c| c.is_whitespace()) {
                chars.next();
            }
            if collapsed.len() >= out.limit {
                return Err(output_too_large());
            }
            collapsed.push(' ');
            continue;
        }
        if collapsed
            .len()
            .checked_add(ch.len_utf8())
            .is_none_or(|n| n > out.limit)
        {
            return Err(output_too_large());
        }
        collapsed.push(ch);
    }
    let t = collapsed.trim();
    if !t.is_empty() {
        emit_pretty_line(out, t, depth, first)?;
    }
    Ok(())
}

fn emit_pretty_line(
    out: &mut BoundedText,
    text: &str,
    depth: usize,
    first: &mut bool,
) -> Result<(), KordocError> {
    if !*first {
        out.byte(b'\n')?;
    }
    *first = false;
    for _ in 0..depth {
        out.byte(b' ')?;
    }
    out.append(text)
}

// Compact in place: tag removal cannot expand text or allocate another string.
fn strip_script_tags(text: &mut String) {
    if !text.contains("<su") && !text.contains("</su") {
        return;
    }
    let mut bytes = std::mem::take(text).into_bytes();
    let mut read = 0;
    let mut write = 0;
    while read < bytes.len() {
        if let Some(tag) = [b"<sup>".as_slice(), b"</sup>", b"<sub>", b"</sub>"]
            .into_iter()
            .find(|tag| bytes[read..].starts_with(tag))
        {
            read += tag.len();
        } else {
            bytes[write] = bytes[read];
            write += 1;
            read += 1;
        }
    }
    bytes.truncate(write);
    *text = String::from_utf8(bytes).expect("removing complete ASCII tags preserves UTF-8");
}

fn strip_optional(text: &mut Option<String>) {
    if let Some(text) = text {
        strip_script_tags(text);
    }
}

fn strip_block_scripts(blocks: &mut [IrBlock], depth: usize) -> Result<(), KordocError> {
    if depth > 64 {
        return Err(KordocError::new(
            ErrorCode::OutputTooLarge,
            "Block nesting exceeds the output limit",
        ));
    }
    for block in blocks {
        strip_optional(&mut block.text);
        strip_optional(&mut block.footnote_text);
        if let Some(spans) = &mut block.spans {
            for span in spans {
                strip_script_tags(&mut span.text);
            }
        }
        if let Some(children) = &mut block.children {
            strip_block_scripts(children, depth + 1)?;
        }
        if let Some(table) = &mut block.table {
            strip_optional(&mut table.caption);
            if let Some(caption) = &mut table.caption_blocks {
                strip_block_scripts(caption, depth + 1)?;
            }
            for cell in table.cells.iter_mut().flatten() {
                strip_script_tags(&mut cell.text);
                if let Some(blocks) = &mut cell.blocks {
                    strip_block_scripts(blocks, depth + 1)?;
                }
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn scripted_result() -> ParseSuccess {
        serde_json::from_value(json!({
            "success": true, "fileType": "hwpx", "markdown": "H<sub>2</sub>O<sup>+</sup>",
            "pages": [{"pageNumber":1,"markdown":"H<sub>2</sub>O<sup>+</sup>"}],
            "blocks": [{"type":"paragraph","text":"H<sub>2</sub>O","footnoteText":"n<sup>1</sup>",
                "spans":[{"text":"x<sup>2</sup>","bold":true}],
                "children":[{"type":"paragraph","text":"c<sub>i</sub>"}],
                "table":{"rows":1,"cols":1,"hasHeader":false,
                    "caption":"T<sup>1</sup>",
                    "captionBlocks":[{"type":"paragraph","text":"cap<sub>x</sub>"}],
                    "cells":[[{"text":"a<sub>j</sub>","rowSpan":1,"colSpan":1,
                        "blocks":[{"type":"paragraph","text":"b<sup>k</sup>"}]}]]}}]
        }))
        .unwrap()
    }

    #[test]
    fn script_tags_false_removes_only_tags_in_all_result_text_surfaces() {
        let mut result = scripted_result();
        apply(
            &mut result,
            &ParseOptions {
                script_tags: Some(false),
                ..ParseOptions::default()
            },
        )
        .unwrap();
        assert_eq!(result.markdown, "H2O+");
        assert_eq!(result.pages.as_ref().unwrap()[0].markdown, "H2O+");
        let root = &result.blocks[0];
        assert_eq!(root.text.as_deref(), Some("H2O"));
        assert_eq!(root.footnote_text.as_deref(), Some("n1"));
        assert_eq!(root.spans.as_ref().unwrap()[0].text, "x2");
        assert_eq!(root.spans.as_ref().unwrap()[0].bold, Some(true));
        assert_eq!(
            root.children.as_ref().unwrap()[0].text.as_deref(),
            Some("ci")
        );
        let table = root.table.as_ref().unwrap();
        assert_eq!(table.caption.as_deref(), Some("T1"));
        assert_eq!(
            table.caption_blocks.as_ref().unwrap()[0].text.as_deref(),
            Some("capx")
        );
        assert_eq!(table.cells[0][0].text, "aj");
        assert_eq!(
            table.cells[0][0].blocks.as_ref().unwrap()[0]
                .text
                .as_deref(),
            Some("bk")
        );
    }

    #[test]
    fn omitted_and_true_script_options_preserve_the_original_result() {
        for script_tags in [None, Some(true)] {
            let expected = scripted_result();
            let mut actual = expected.clone();
            apply(
                &mut actual,
                &ParseOptions {
                    script_tags,
                    ..ParseOptions::default()
                },
            )
            .unwrap();
            assert_eq!(actual, expected);
        }
    }

    #[test]
    fn stripping_preserves_unicode_unrelated_markup_and_tag_case() {
        let mut text =
            "한글<sup>四</sup> <u>밑줄</u> <SUP>X</SUP> <sub>i\n</sub> </sub>".to_owned();
        strip_script_tags(&mut text);
        assert_eq!(text, "한글四 <u>밑줄</u> <SUP>X</SUP> i\n ");
    }

    #[test]
    fn plain_option_cleans_markdown_and_each_page_without_changing_ir() {
        let original_ir = scripted_result().blocks;
        let mut result = scripted_result();
        result.markdown = "![diagram](image.png) [링크](https://example.org/a) **bold** <u>under</u> H<sub>2</sub>O<sup>10</sup> <img src=\"x\">\n\n\n| A | B |\n| --- | --- |\n| x | y |   ".into();
        result.pages.as_mut().unwrap()[0].markdown = "![p](p.png) **쪽** H<sub>2</sub>".into();

        apply(
            &mut result,
            &ParseOptions {
                plain: Some(true),
                ..ParseOptions::default()
            },
        )
        .unwrap();

        assert_eq!(
            result.markdown,
            "링크 bold under H_2O^10\n\n| A | B |\n| --- | --- |\n| x | y |"
        );
        assert_eq!(result.pages.as_ref().unwrap()[0].markdown, "쪽 H_2");
        assert_eq!(result.blocks, original_ir);
    }

    #[test]
    fn omitted_false_and_true_options_preserve_non_plain_markdown() {
        for options in [
            ParseOptions::default(),
            ParseOptions {
                plain: Some(false),
                html_tables: Some(false),
                ..ParseOptions::default()
            },
        ] {
            let mut result = scripted_result();
            let expected = result.clone();
            apply(&mut result, &options).unwrap();
            assert_eq!(result, expected);
        }
    }

    #[test]
    fn html_tables_convert_escaped_pipe_rows_and_preserve_unicode() {
        let mut result = scripted_result();
        result.markdown = "Before\n| 이름 | a\\|b |\n| --- | --- |\n| 한글 | x\\|y |\nAfter".into();
        apply(
            &mut result,
            &ParseOptions {
                html_tables: Some(true),
                ..ParseOptions::default()
            },
        )
        .unwrap();
        assert_eq!(
            result.markdown,
            "Before\n<table>\n <tr>\n  <th>\n   이름\n  </th>\n  <th>\n   a|b\n  </th>\n </tr>\n <tr>\n  <td>\n   한글\n  </td>\n  <td>\n   x|y\n  </td>\n </tr>\n</table>\nAfter"
        );
    }

    #[test]
    fn html_tables_prettify_nested_html_and_combination_order_is_script_then_plain_then_html() {
        let mut result = scripted_result();
        result.markdown = "H<sub>2</sub>\n<table><tr><td>outer<table><tr><td>inner</td></tr></table></td></tr></table>\n| X | Y |\n| --- | --- |\n| a | b |".into();
        apply(
            &mut result,
            &ParseOptions {
                script_tags: Some(false),
                plain: Some(true),
                html_tables: Some(true),
                ..ParseOptions::default()
            },
        )
        .unwrap();
        assert!(result.markdown.starts_with("H2\n<table>\n <tr>\n  <td>\n   outer\n   <table>\n    <tr>\n     <td>\n      inner\n     </td>\n    </tr>\n   </table>\n  </td>\n </tr>\n</table>"), "actual: {:?}", result.markdown);
        assert!(result.markdown.contains("<th>\n   X\n  </th>"));
        assert_eq!(result.blocks[0].text.as_deref(), Some("H2O"));
    }

    #[test]
    fn plain_and_html_expansion_and_table_depth_fail_without_truncation() {
        let source = format!("| x |\n| --- |\n| {} |", "&".repeat(14_000));
        let error =
            transform_html_tables(&source, crate::markdown::MAX_MARKDOWN_BYTES).unwrap_err();
        assert_eq!(error.code, ErrorCode::OutputTooLarge);
        let nested = format!("{}x{}", "<table>".repeat(65), "</table>".repeat(65));
        let error =
            transform_html_tables(&nested, crate::markdown::MAX_MARKDOWN_BYTES).unwrap_err();
        assert_eq!(error.code, ErrorCode::OutputTooLarge);
    }

    #[test]
    fn pipe_heavy_row_stops_at_output_limit_without_collecting_all_cells() {
        let row = format!("|{}|", "|".repeat(2_000_000));
        let mut out = BoundedText {
            bytes: Vec::new(),
            limit: 128,
        };
        let error = append_pipe_row(&row, "td", &mut out).unwrap_err();
        assert_eq!(error.code, ErrorCode::OutputTooLarge);
        assert!(out.bytes.len() <= out.limit);
    }

    #[test]
    fn plain_script_formula_and_backslash_escaping_match_oracle_rules() {
        let mut text = r"10<sup>4</sup> m<sup>2</sup> H<sub>2</sub>O <sup>n+1</sup> <sup>+</sup> \**bold** [web](https://example.org)".to_owned();
        plain_markdown(&text, crate::markdown::MAX_MARKDOWN_BYTES)
            .map(|value| text = value)
            .unwrap();
        assert_eq!(text, r"10^4 m^2 H_2O ^(n+1) ^(+) \**bold** web");
    }

    #[test]
    fn malformed_repeated_markers_are_scanned_linearly_and_preserved() {
        let scripts = "<sup>".repeat(10_000);
        assert_eq!(plain_scripts(&scripts, 64 * 1024).unwrap(), scripts);
        let images = "![unfinished ".repeat(10_000);
        assert_eq!(remove_image_markdown(&images, 256 * 1024).unwrap(), images);
        let bold = "**unfinished ".repeat(10_000);
        assert_eq!(unwrap_bold(&bold, 256 * 1024).unwrap(), bold);
    }

    #[test]
    fn html_pipe_cells_escape_html_but_keep_allowed_inline_tags_and_transform_pages() {
        let mut result = scripted_result();
        result.markdown =
            "| A | B |\n| --- | --- |\n| a & <b> | <u>밑줄</u><br><img src=\"x\"> |".into();
        result.pages.as_mut().unwrap()[0].markdown = "| 쪽 | 값 |\n| --- | --- |\n| 1 | 2 |".into();
        apply(
            &mut result,
            &ParseOptions {
                html_tables: Some(true),
                ..ParseOptions::default()
            },
        )
        .unwrap();
        assert!(result.markdown.contains("a &amp; &lt;b&gt;"));
        assert!(result.markdown.contains("<u>밑줄</u><br><img src=\"x\">"));
        assert!(
            result.pages.as_ref().unwrap()[0]
                .markdown
                .contains("<th>\n   쪽\n  </th>")
        );
    }

    #[test]
    fn plain_preserves_oracle_escape_and_unicode_superscript_cases() {
        let escaped = r#"\**keep** \[link](https://example.test) ![x](p) <IMG src="p"> <a>x</a>"#;
        assert_eq!(
            plain_markdown(escaped, crate::markdown::MAX_MARKDOWN_BYTES).unwrap(),
            r"\**keep** \link   <a>x</a>"
        );
        let formula = "H<sub>2</sub>O m<sup>2</sup> x<sup>n+1</sup> 한글<sup>四</sup> **굵게** <u>밑줄</u> [링크](https://example.test) ![image](p.png)";
        assert_eq!(
            plain_markdown(formula, crate::markdown::MAX_MARKDOWN_BYTES).unwrap(),
            "H_2O m^2 x^(n+1) 한글^四 굵게 밑줄 링크"
        );
    }

    #[test]
    fn html_pipe_conversion_preserves_blank_lines_around_the_table() {
        let source =
            "Before\n\n| A | B |\n| --- | --- |\n| 한글 \\| 칸 | <sup>2</sup> & < |\n\nAfter";
        let result = transform_html_tables(source, crate::markdown::MAX_MARKDOWN_BYTES).unwrap();
        assert_eq!(
            result,
            "Before\n\n<table>\n <tr>\n  <th>\n   A\n  </th>\n  <th>\n   B\n  </th>\n </tr>\n <tr>\n  <td>\n   한글 | 칸\n  </td>\n  <td>\n   <sup>2</sup> &amp; &lt;\n  </td>\n </tr>\n</table>\n\nAfter"
        );
    }

    #[test]
    fn matches_the_captured_authored_option_projection_cases() {
        let fixture: serde_json::Value = serde_json::from_str(include_str!(
            "../tests/fixtures/option-projections/captured.json"
        ))
        .unwrap();
        for case in fixture["cases"].as_array().unwrap() {
            let id = case["id"].as_str().unwrap();
            let input = case["input"].as_str().unwrap();
            let plain = plain_markdown(input, crate::markdown::MAX_MARKDOWN_BYTES).unwrap();
            assert_eq!(plain, case["plain"].as_str().unwrap(), "plain case {id}");
            let html = transform_html_tables(input, crate::markdown::MAX_MARKDOWN_BYTES).unwrap();
            assert_eq!(html, case["html"].as_str().unwrap(), "html case {id}");
            let combined_html =
                transform_html_tables(&plain, crate::markdown::MAX_MARKDOWN_BYTES).unwrap();
            assert_eq!(
                combined_html,
                case["plain_then_html"].as_str().unwrap(),
                "ordered case {id}"
            );

            let mut result = scripted_result();
            result.markdown = input.to_owned();
            result.pages = None;
            apply(
                &mut result,
                &ParseOptions {
                    script_tags: Some(false),
                    plain: Some(true),
                    ..ParseOptions::default()
                },
            )
            .unwrap();
            assert_eq!(
                result.markdown,
                case["scripts_off_then_plain"].as_str().unwrap(),
                "scriptTags-off case {id}"
            );
        }
    }
}
