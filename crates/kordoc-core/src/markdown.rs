//! Deterministic Markdown projection from source-neutral document IR.
#![allow(dead_code)] // Exports and ParseSuccess assembly are coordinator integration work.

use kordoc_ir::{ErrorCode, IrBlock, IrBlockType, IrCell, IrSpan, IrTable, KordocError};

#[cfg(not(test))]
pub(crate) const MAX_MARKDOWN_BYTES: usize = 256 * 1024 * 1024;
#[cfg(test)]
pub(crate) const MAX_MARKDOWN_BYTES: usize = 64 * 1024;
const MAX_BLOCK_DEPTH: usize = 64;

fn output_too_large() -> KordocError {
    KordocError::new(
        ErrorCode::OutputTooLarge,
        "Projected Markdown exceeds the output limit",
    )
}

fn append_limited(out: &mut String, text: &str) -> Result<(), KordocError> {
    if out
        .len()
        .checked_add(text.len())
        .is_none_or(|size| size > MAX_MARKDOWN_BYTES)
    {
        return Err(output_too_large());
    }
    out.push_str(text);
    Ok(())
}

fn escape_gfm(text: &str) -> Result<String, KordocError> {
    if text.len() > MAX_MARKDOWN_BYTES {
        return Err(output_too_large());
    }
    let normalized = sanitize_text(text);
    let text = normalized.trim();
    let mut out = String::with_capacity(text.len());
    let mut line_start = true;
    let mut chars = text.char_indices().peekable();
    while let Some((index, ch)) = chars.next() {
        let next = chars.peek().map(|(_, value)| *value);
        if ch == '\\' {
            if next.is_some_and(|next| matches!(next, '|' | '$')) {
                append_limited(&mut out, "\\")?;
                let (_, escaped) = chars.next().expect("peeked character is present");
                append_limited(&mut out, &escaped.to_string())?;
            } else {
                append_limited(&mut out, "\\\\")?;
            }
        } else if ch == '$' {
            if let Some(end) = math_span_end(text, index) {
                append_limited(&mut out, &text[index..end])?;
                while chars
                    .peek()
                    .is_some_and(|(next_index, _)| *next_index < end)
                {
                    chars.next();
                }
                line_start = text[index..end].ends_with('\n');
            } else {
                append_limited(&mut out, "$")?;
            }
        } else if matches!(ch, '~' | '*' | '_' | '`' | '|') {
            append_limited(&mut out, "\\")?;
            append_limited(&mut out, &ch.to_string())?;
        } else if ch == '#' && line_start {
            let remaining = &text[index..];
            let hashes = remaining.chars().take_while(|value| *value == '#').count();
            let has_space =
                remaining[hashes..].starts_with(' ') || remaining[hashes..].starts_with('\t');
            if (1..=6).contains(&hashes) && has_space {
                append_limited(&mut out, "\\")?;
            }
            append_limited(&mut out, "#")?;
        } else if ch == '<'
            && !["<u>", "</u>", "<sup>", "</sup>", "<sub>", "</sub>"]
                .iter()
                .any(|tag| text[index..].starts_with(tag))
            && next
                .is_some_and(|next| next.is_ascii_alphabetic() || matches!(next, '/' | '!' | '?'))
        {
            append_limited(&mut out, "\\<")?;
        } else {
            append_limited(&mut out, &ch.to_string())?;
        }
        if ch != '$' {
            line_start = if line_start && matches!(ch, ' ' | '\t') {
                true
            } else {
                ch == '\n'
            };
        }
    }
    Ok(out)
}

const WINGDINGS: &[&str] = &[
    "🖉", "✂", "✁", "👓", "🕭", "🕮", "🕯", "🕿", "✆", "🖂", "🖃", "📪", "📫", "📬", "📭", "📁", "📂",
    "📄", "🗏", "🗐", "🗄", "⌛", "🖮", "🖰", "🖲", "🖳", "🖴", "🖫", "🖬", "✇", "✍", "🖎", "✌", "👌", "👍",
    "👎", "☜", "☞", "☝", "☟", "🖐", "☺", "😐", "☹", "💣", "☠", "🏳", "🏱", "✈", "☼", "💧", "❄", "🕆",
    "✞", "🕈", "✠", "✡", "☪", "☯", "ॐ", "☸", "♈", "♉", "♊", "♋", "♌", "♍", "♎", "♏", "♐",
    "♑", "♒", "♓", "🙰", "🙵", "●", "○", "■", "□", "□", "❑", "❒", "⬧", "⧫", "◆", "❖", "⬥", "⌧",
    "⮹", "⌘", "🏵", "🏶", "🙶", "🙷", "", "⓪", "①", "②", "③", "④", "⑤", "⑥", "⑦", "⑧", "⑨", "⑩", "⓿",
    "❶", "❷", "❸", "❹", "❺", "❻", "❼", "❽", "❾", "❿", "🙢", "🙠", "🙡", "🙣", "🙞", "🙜", "🙝", "🙟", "·",
    "•", "▪", "⚪", "○", "◯", "◉", "◎", "🔿", "▪", "◻", "🟂", "✦", "★", "✶", "✴", "✹", "✵", "⯐", "⌖",
    "⟡", "⌑", "⯑", "✪", "✰", "🕐", "🕑", "🕒", "🕓", "🕔", "🕕", "🕖", "🕗", "🕘", "🕙", "🕚",
    "🕛", "⮰", "⮱", "⮲", "⮳", "⮴", "⮵", "⮶", "⮷", "🙪", "🙫", "🙕", "🙔", "🙗", "🙖", "🙐", "🙑", "🙒", "🙓",
    "⌫", "⌦", "⮘", "⮚", "⮙", "⮛", "⮈", "⮊", "⮉", "⮋", "←", "→", "↑", "↓", "↖", "↗", "↙", "↘", "⬅",
    "➔", "⬆", "⬇", "⬉", "⬈", "⬋", "⬊", "⇦", "⇨", "⇧", "⇩", "⬄", "⇳", "⬀", "⬁", "⬃", "⬂", "🢬", "🢭",
    "✗", "✔", "☒", "☑", "",
];

fn map_pua(code: u32, original: char) -> Option<String> {
    if (0xF020..=0xF0FF).contains(&code) {
        let mapped = match code {
            0xF022 => Some("✂"),
            0xF036 => Some("⌛"),
            0xF045 => Some("☜"),
            0xF046 => Some("☞"),
            0xF047 => Some("☝"),
            0xF048 => Some("☟"),
            0xF04A => Some("☺"),
            0xF04E => Some("☠"),
            0xF052 => Some("☼"),
            0xF054 => Some("❄"),
            0xF058 => Some("✠"),
            0xF059 => Some("✡"),
            0xF06C | 0xF06D => Some("●"),
            0xF06E => Some("■"),
            0xF06F..=0xF072 => Some("□"),
            0xF073 => Some("⬧"),
            0xF074 => Some("⧫"),
            0xF075 => Some("◆"),
            0xF076 => Some("❖"),
            0xF077 => Some("⬥"),
            0xF09E | 0xF0A0 => Some("·"),
            0xF09F => Some("•"),
            0xF0A1 => Some("⚪"),
            0xF0A2 | 0xF0A3 => Some("○"),
            0xF0A4 => Some("◉"),
            0xF0A5 => Some("◎"),
            0xF0A7 => Some("▪"),
            0xF0A8 => Some("◻"),
            0xF0AA => Some("✦"),
            0xF0AB => Some("★"),
            0xF0AC => Some("✶"),
            0xF0AD => Some("✴"),
            0xF0AE => Some("✹"),
            0xF0E8 => Some("➔"),
            0xF0EF => Some("⇦"),
            0xF0F0 => Some("⇨"),
            0xF0F1 => Some("⇧"),
            0xF0F2 => Some("⇩"),
            0xF0FB => Some("✗"),
            0xF0FC => Some("✔"),
            0xF0FD => Some("☒"),
            0xF0FE => Some("☑"),
            _ if code >= 0xF021 => WINGDINGS
                .get((code - 0xF021) as usize)
                .copied()
                .filter(|value| !value.is_empty()),
            _ => None,
        };
        return mapped
            .map(str::to_owned)
            .or_else(|| Some(original.to_string()));
    }
    if (0xF02B1..=0xF02C4).contains(&code) {
        return char::from_u32(0x2460 + (code - 0xF02B1)).map(|ch| ch.to_string());
    }
    let supplementary = match code {
        0xF003B => Some("↓"),
        0xF02EF => Some("·"),
        0xF0854 => Some("《"),
        0xF0855 => Some("》"),
        0xF00DA | 0xF02FB => Some("▸"),
        0xF080F | 0xF0848 => Some("━"),
        0xF0827 | 0xF031C => Some("■"),
        0xF03C5 | 0xF03DA | 0xF03FF => Some("□"),
        0xF012B | 0xF00E1 => Some("(인)"),
        0xF02FC => Some("►"),
        0xF03A0 => Some("↵"),
        0xF03EF => Some("한"),
        0xF03F0 => Some("글"),
        0xF03F1 => Some("과"),
        0xF03F2 => Some("컴"),
        0xF03F3 => Some("퓨"),
        0xF03F4 => Some("터"),
        0xF0090 => Some("✺"),
        0xF0288 => Some("⓪"),
        0xF0289 => Some("①"),
        0xF028A => Some("②"),
        0xF028C => Some("④"),
        0xF028D => Some("⑤"),
        0xF028E => Some("⑥"),
        0xF028F => Some("⑦"),
        0xF0290 => Some("⑧"),
        0xF0291 => Some("⑨"),
        0xF02EC => Some("◇"),
        0xF03A7 => Some("⊟"),
        0xF03A8 => Some("⊞"),
        0xF0806 => Some("┌"),
        0xF0807 => Some("┬"),
        0xF0808 => Some("┐"),
        0xF080C => Some("└"),
        0xF080E => Some("┘"),
        0xF0810 => Some("│"),
        0xF081C => Some("┈"),
        0xF0832 => Some("═"),
        _ => None,
    };
    if supplementary.is_some() {
        return supplementary.map(str::to_owned);
    }
    if (0xF0000..=0xFFFFD).contains(&code) {
        return None;
    }
    Some(original.to_string())
}

fn normalize_pua(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for ch in text.chars() {
        if let Some(mapped) = map_pua(ch as u32, ch) {
            out.push_str(&mapped);
        }
    }
    out
}

fn sanitize_text(text: &str) -> String {
    let mut cleaned = normalize_pua(text)
        .lines()
        .filter(|line| !is_hwp_shape_alt(line))
        .collect::<Vec<_>>()
        .join("\n");
    let mut collapsed = String::with_capacity(cleaned.len());
    let mut previous_space = false;
    for ch in cleaned.chars() {
        if ch == ' ' && previous_space {
            continue;
        }
        collapsed.push(ch);
        previous_space = ch == ' ';
    }
    cleaned = collapsed.trim().to_owned();
    if cleaned.encode_utf16().count() <= 30 && cleaned.contains(' ') {
        let tokens = cleaned.split(' ').collect::<Vec<_>>();
        let is_single_hangul = |token: &str| {
            let mut chars = token.chars();
            chars
                .next()
                .is_some_and(|ch| matches!(ch as u32, 0xAC00..=0xD7AF | 0x3131..=0x318E))
                && chars.next().is_none()
        };
        let count = tokens
            .iter()
            .filter(|token| is_single_hangul(token))
            .count();
        let all_date_units = tokens.iter().all(|token| {
            !is_single_hangul(token) || token.chars().any(|ch| "년월일시분초".contains(ch))
        });
        if tokens.len() >= 3 && count * 10 >= tokens.len() * 7 && !all_date_units {
            cleaned = tokens.concat();
        }
    }
    cleaned
}

fn is_hwp_shape_alt(line: &str) -> bool {
    let mut text = line.trim();
    for prefix in ["모서리가 둥근 ", "둥근 "] {
        if let Some(stripped) = text.strip_prefix(prefix) {
            text = stripped;
            break;
        }
    }
    let text = text.strip_suffix('.').unwrap_or(text).trim();
    let Some(shape) = text.strip_suffix("입니다") else {
        return false;
    };
    let shape = shape.trim_end();
    matches!(
        shape,
        "사각형"
            | "직사각형"
            | "정사각형"
            | "원"
            | "타원"
            | "삼각형"
            | "이등변 삼각형"
            | "직각 삼각형"
            | "선"
            | "직선"
            | "곡선"
            | "화살표"
            | "굵은 화살표"
            | "이중 화살표"
            | "오각형"
            | "육각형"
            | "팔각형"
            | "별"
            | "4점별"
            | "5점별"
            | "6점별"
            | "7점별"
            | "8점별"
            | "십자"
            | "십자형"
            | "구름"
            | "구름형"
            | "마름모"
            | "도넛"
            | "평행사변형"
            | "사다리꼴"
            | "부채꼴"
            | "호"
            | "반원"
            | "물결"
            | "번개"
            | "하트"
            | "빗금"
            | "블록 화살표"
            | "수식"
            | "표"
            | "그림"
            | "개체"
            | "그리기 개체"
            | "그리기개체"
            | "묶음 개체"
            | "묶음개체"
            | "글상자"
            | "수식 개체"
            | "수식개체"
            | "OLE 개체"
            | "OLE개체"
    )
}

fn is_ordered_marker(text: &str) -> bool {
    let digit_count = text.bytes().take_while(u8::is_ascii_digit).count();
    digit_count > 0
        && text[digit_count..].starts_with('.')
        && text[digit_count + 1..]
            .chars()
            .next()
            .is_some_and(char::is_whitespace)
}

fn is_hangul_marker(text: &str) -> bool {
    let mut chars = text.chars();
    chars
        .next()
        .is_some_and(|ch| matches!(ch as u32, 0xAC00..=0xD7AF))
        && chars.next() == Some('.')
        && chars.next().is_some_and(char::is_whitespace)
}

fn math_span_end(text: &str, start: usize) -> Option<usize> {
    let delimiter = if text[start..].starts_with("$$") {
        "$$"
    } else {
        "$"
    };
    let mut escaped = false;
    for (offset, ch) in text[start + delimiter.len()..].char_indices() {
        if escaped {
            escaped = false;
            continue;
        }
        if ch == '\\' {
            escaped = true;
            continue;
        }
        let index = start + delimiter.len() + offset;
        if delimiter == "$" && ch == '\n' {
            return None;
        }
        if text[index..].starts_with(delimiter) {
            return Some(index + delimiter.len());
        }
    }
    None
}

fn sanitize_href(href: &str) -> Option<String> {
    let href = href.trim();
    if href.len() > MAX_MARKDOWN_BYTES {
        return None;
    }
    let allowed = ["https:", "http:", "mailto:", "tel:", "#"]
        .iter()
        .any(|scheme| {
            href.get(..scheme.len())
                .is_some_and(|prefix| prefix.eq_ignore_ascii_case(scheme))
        });
    if !allowed {
        return None;
    }
    let mut safe = String::with_capacity(href.len());
    for ch in href.chars() {
        match ch {
            '(' => safe.push_str("%28"),
            ')' => safe.push_str("%29"),
            '<' => safe.push_str("%3C"),
            '>' => safe.push_str("%3E"),
            '"' => safe.push_str("%22"),
            '\'' => safe.push_str("%27"),
            '`' => safe.push_str("%60"),
            c if c.is_ascii_whitespace() => {
                for byte in c.to_string().bytes() {
                    safe.push_str(&format!("%{byte:02X}"));
                }
            }
            _ => safe.push(ch),
        }
    }
    Some(safe)
}

fn escape_html(text: &str, attribute: bool) -> Result<String, KordocError> {
    let mut out = String::new();
    for ch in text.chars() {
        match ch {
            '&' => append_limited(&mut out, "&amp;")?,
            '<' => append_limited(&mut out, "&lt;")?,
            '>' => append_limited(&mut out, "&gt;")?,
            '"' if attribute => append_limited(&mut out, "&quot;")?,
            _ => append_limited(&mut out, &ch.to_string())?,
        }
    }
    Ok(out)
}

fn append_escaped(out: &mut String, text: &str) -> Result<(), KordocError> {
    let escaped = escape_gfm(text)?;
    append_limited(out, &escaped)
}

fn render_spans(spans: &[IrSpan]) -> Result<String, KordocError> {
    let mut out = String::new();
    for span in spans {
        if span.placeholder == Some(true) {
            continue;
        }
        let text = normalize_pua(&span.text);
        let leading_len = text.len() - text.trim_start().len();
        let trailing_start = text.trim_end().len();
        let leading = &text[..leading_len];
        let core = &text[leading_len..trailing_start.max(leading_len)];
        let trailing = &text[trailing_start.max(leading_len)..];
        if core.is_empty() {
            append_limited(&mut out, &text)?;
            continue;
        }
        let code = span.code == Some(true);
        let mut marker = if code {
            "`"
        } else if span.bold == Some(true) && span.italic == Some(true) {
            "***"
        } else if span.bold == Some(true) {
            "**"
        } else if span.italic == Some(true) {
            "*"
        } else {
            ""
        };
        let strike = span.strike == Some(true) && !code;
        if strike {
            marker = if marker.is_empty() {
                "~~"
            } else {
                match marker {
                    "***" => "~~***",
                    "**" => "~~**",
                    "*" => "~~*",
                    _ => "~~",
                }
            };
        }
        append_limited(&mut out, leading)?;
        if span.underline == Some(true) && !code {
            append_limited(&mut out, "<u>")?;
        }
        if !marker.is_empty() {
            append_limited(&mut out, marker)?;
        }
        if code {
            append_limited(&mut out, core)?;
        } else {
            append_escaped(&mut out, core)?;
        }
        if !marker.is_empty() {
            if strike {
                let close = if marker.ends_with("***") {
                    "***~~"
                } else if marker.ends_with("**") {
                    "**~~"
                } else if marker.ends_with('*') {
                    "*~~"
                } else {
                    "~~"
                };
                append_limited(&mut out, close)?;
            } else {
                append_limited(&mut out, marker)?;
            }
        }
        if span.underline == Some(true) && !code {
            append_limited(&mut out, "</u>")?;
        }
        append_limited(&mut out, trailing)?;
    }
    Ok(out)
}

fn escape_html_cell_text(text: &str) -> Result<String, KordocError> {
    let escaped = escape_html(text, false)?;
    // These are the only inline tags accepted from IR text as kordoc formatting markers.
    Ok(escaped
        .replace("&lt;u&gt;", "<u>")
        .replace("&lt;/u&gt;", "</u>")
        .replace("&lt;sup&gt;", "<sup>")
        .replace("&lt;/sup&gt;", "</sup>")
        .replace("&lt;sub&gt;", "<sub>")
        .replace("&lt;/sub&gt;", "</sub>"))
}

fn checked_table_shape(table: &IrTable) -> Result<(), KordocError> {
    let rows = table.rows as usize;
    let cols = table.cols as usize;
    if rows > 2_000_000
        || cols > 200
        || rows.checked_mul(cols).is_none_or(|count| count > 2_000_000)
    {
        return Err(output_too_large());
    }
    if table.cells.len() < rows || table.cells.iter().take(rows).any(|row| row.len() < cols) {
        return Err(KordocError::new(
            ErrorCode::ParseError,
            "IR table grid dimensions do not match its cells",
        ));
    }
    Ok(())
}

fn table_caption_html(table: &IrTable, depth: usize) -> Result<String, KordocError> {
    if let Some(blocks) = table
        .caption_blocks
        .as_deref()
        .filter(|blocks| blocks.iter().any(|block| block.kind == IrBlockType::Table))
    {
        let mut pieces = Vec::with_capacity(blocks.len());
        for block in blocks {
            if block.kind == IrBlockType::Table {
                if let Some(nested) = block.table.as_ref() {
                    let caption = table_caption_html(nested, depth + 1)?;
                    let rendered = table_html(nested, depth + 1)?;
                    pieces.push(if caption.is_empty() {
                        rendered
                    } else {
                        format!("{caption}<br>{rendered}")
                    });
                }
            } else if let Some(text) = block.text.as_deref() {
                let mut value = sanitize_text(text);
                if let Some(note) = block.footnote_text.as_deref()
                    && block.text.is_some()
                {
                    value.push_str(&format!(" (주: {})", sanitize_text(note)));
                }
                pieces.push(escape_html_cell_text(&value)?);
            }
        }
        let size = pieces
            .iter()
            .try_fold(0usize, |sum, item| {
                sum.checked_add(item.len())?.checked_add(4)
            })
            .ok_or_else(output_too_large)?;
        if size > MAX_MARKDOWN_BYTES {
            return Err(output_too_large());
        }
        return Ok(pieces.join("<br>"));
    }
    table
        .caption
        .as_deref()
        .filter(|text| !text.trim().is_empty())
        .map(sanitize_text)
        .map(|text| escape_html_cell_text(&text))
        .transpose()
        .map(|text| text.unwrap_or_default())
}

fn cell_html(cell: &IrCell, depth: usize) -> Result<String, KordocError> {
    if let Some(blocks) = cell.blocks.as_deref().filter(|blocks| !blocks.is_empty()) {
        let mut pieces = Vec::with_capacity(blocks.len());
        for block in blocks {
            if block.kind == IrBlockType::Table {
                if let Some(table) = block.table.as_ref() {
                    let caption = table_caption_html(table, depth + 1)?;
                    let rendered = table_html(table, depth + 1)?;
                    pieces.push(if caption.is_empty() {
                        rendered
                    } else {
                        format!("{caption}<br>{rendered}")
                    });
                }
            } else if block.kind == IrBlockType::Image {
                if let Some(src) = block.text.as_deref() {
                    pieces.push(format!(
                        "<img src=\"{}\" alt=\"image\">",
                        escape_html(src, true)?
                    ));
                }
            } else if block.kind == IrBlockType::Separator {
                pieces.push("<hr>".to_owned());
            } else if let Some(text) = block.text.as_deref() {
                let mut value = sanitize_text(text);
                if let Some(note) = block.footnote_text.as_deref()
                    && block.text.is_some()
                {
                    value.push_str(&format!(" (주: {})", sanitize_text(note)));
                }
                pieces.push(escape_html_cell_text(&value)?.replace('\n', "<br>"));
            }
        }
        let total_len = pieces
            .iter()
            .try_fold(0usize, |total, piece| {
                total.checked_add(piece.len())?.checked_add(4)
            })
            .ok_or_else(output_too_large)?;
        if total_len > MAX_MARKDOWN_BYTES {
            return Err(output_too_large());
        }
        let out = pieces.join("<br>");
        Ok(out)
    } else {
        let out = escape_html_cell_text(&sanitize_text(&cell.text))?.replace('\n', "<br>");
        if out.len() > MAX_MARKDOWN_BYTES {
            return Err(output_too_large());
        }
        Ok(out)
    }
}

fn table_html(table: &IrTable, depth: usize) -> Result<String, KordocError> {
    if depth > MAX_BLOCK_DEPTH {
        return Err(output_too_large());
    }
    checked_table_shape(table)?;
    let rows = table.rows as usize;
    let cols = table.cols as usize;
    let mut out = String::new();
    append_limited(&mut out, "<table>\n")?;
    let mut covered = vec![vec![false; cols]; rows];
    for row in 0..rows {
        append_limited(&mut out, "<tr>")?;
        for col in 0..cols {
            if covered[row][col] {
                continue;
            }
            let cell = &table.cells[row][col];
            let col_span = (cell.col_span as usize).max(1).min(cols - col);
            let row_span = (cell.row_span as usize).max(1).min(rows - row);
            for dr in 0..row_span {
                for dc in 0..col_span {
                    if dr > 0 || dc > 0 {
                        covered[row + dr][col + dc] = true;
                    }
                }
            }
            let tag = if row == 0 || cell.is_header == Some(true) {
                "th"
            } else {
                "td"
            };
            append_limited(&mut out, "<")?;
            append_limited(&mut out, tag)?;
            if col_span > 1 {
                append_limited(&mut out, &format!(" colspan=\"{col_span}\""))?;
            }
            if row_span > 1 {
                append_limited(&mut out, &format!(" rowspan=\"{row_span}\""))?;
            }
            append_limited(&mut out, ">")?;
            append_limited(&mut out, &cell_html(cell, depth)?)?;
            append_limited(&mut out, "</")?;
            append_limited(&mut out, tag)?;
            append_limited(&mut out, ">")?;
        }
        append_limited(&mut out, "</tr>\n")?;
    }
    append_limited(&mut out, "</table>")?;
    Ok(out)
}

fn table_markdown(table: &IrTable, depth: usize) -> Result<String, KordocError> {
    if depth > MAX_BLOCK_DEPTH {
        return Err(output_too_large());
    }
    checked_table_shape(table)?;
    let rows = table.rows as usize;
    let cols = table.cols as usize;
    if rows == 0 || cols == 0 {
        return Ok(String::new());
    }
    let merged = table
        .cells
        .iter()
        .take(rows)
        .flat_map(|row| row.iter().take(cols))
        .any(|cell| cell.col_span > 1 || cell.row_span > 1);
    let structured = crate::table::has_structured_cell_content(table);
    if merged || structured || table.render_as_table == Some(true) {
        return table_html(table, depth);
    }
    let text = |cell: &IrCell| -> Result<String, KordocError> {
        let use_blocks = cell.blocks.as_deref().is_some_and(|blocks| {
            blocks.iter().any(|block| {
                block.spans.is_some() || (block.kind == IrBlockType::Image && block.text.is_some())
            })
        });
        let mut content = if use_blocks {
            let blocks = cell.blocks.as_deref().unwrap_or_default();
            let mut pieces = Vec::new();
            for block in blocks {
                if block.kind == IrBlockType::Image {
                    pieces.push(
                        block
                            .text
                            .as_deref()
                            .map(|src| format!("![image]({src})"))
                            .unwrap_or_default(),
                    );
                } else if let Some(spans) = block.spans.as_deref() {
                    let mut rendered = render_spans(spans)?;
                    if let Some(note) = block
                        .footnote_text
                        .as_deref()
                        .filter(|_| block.text.is_some())
                    {
                        rendered.push_str(&format!(" (주: {})", escape_gfm(note)?));
                    }
                    pieces.push(rendered);
                } else if let Some(text) = block.text.as_deref() {
                    let mut text = escape_gfm(&sanitize_text(text))?;
                    if let Some(note) = block.footnote_text.as_deref() {
                        text.push_str(&format!(" (주: {})", escape_gfm(note)?));
                    }
                    pieces.push(text);
                }
            }
            pieces.join("<br>")
        } else {
            escape_gfm(&sanitize_text(&cell.text))?
        };
        content = content.replace('\n', "<br>");
        Ok(content)
    };
    if rows == 1 && cols == 1 {
        let content = sanitize_text(&table.cells[0][0].text);
        let mut lines = Vec::new();
        for line in content
            .lines()
            .map(str::trim)
            .filter(|line| !line.is_empty())
        {
            if is_ordered_marker(line) {
                lines.push(format!("**{}**", escape_gfm(line)?));
            } else if is_hangul_marker(line) {
                lines.push(format!("  {}", escape_gfm(line)?));
            } else {
                lines.push(escape_gfm(line)?);
            }
        }
        return Ok(lines.join("\n"));
    }
    if cols == 1 {
        let mut lines = Vec::new();
        for row in table.cells.iter().take(rows) {
            for line in text(&row[0])?
                .lines()
                .map(str::trim)
                .filter(|line| !line.is_empty())
            {
                lines.push(line.to_owned());
            }
        }
        return Ok(lines.join("\n"));
    }
    let mut out = String::new();
    for row in 0..rows {
        append_limited(&mut out, "| ")?;
        for col in 0..cols {
            if col > 0 {
                append_limited(&mut out, " | ")?;
            }
            append_limited(&mut out, &text(&table.cells[row][col])?)?;
        }
        append_limited(&mut out, " |\n")?;
        if row == 0 {
            append_limited(&mut out, "| ")?;
            for col in 0..cols {
                if col > 0 {
                    append_limited(&mut out, " | ")?;
                }
                append_limited(&mut out, "---")?;
            }
            append_limited(&mut out, " |\n")?;
        }
    }
    Ok(out.trim_end().to_owned())
}

fn table_markdown_with_caption(table: &IrTable, depth: usize) -> Result<String, KordocError> {
    if depth > MAX_BLOCK_DEPTH {
        return Err(output_too_large());
    }
    let mut out = String::new();
    if let Some(blocks) = table
        .caption_blocks
        .as_deref()
        .filter(|blocks| blocks.iter().any(|block| block.kind == IrBlockType::Table))
    {
        for block in blocks {
            if block.kind == IrBlockType::Table {
                if let Some(nested) = block.table.as_ref() {
                    let rendered = table_markdown_with_caption(nested, depth + 1)?;
                    if !rendered.is_empty() {
                        append_limited(&mut out, &rendered)?;
                        append_limited(&mut out, "\n\n")?;
                    }
                }
            } else if let Some(text) = block.text.as_deref() {
                append_limited(&mut out, "**")?;
                append_escaped(&mut out, text)?;
                if let Some(note) = block.footnote_text.as_deref() {
                    append_limited(&mut out, &format!(" (주: {})", escape_gfm(note)?))?;
                }
                append_limited(&mut out, "**\n\n")?;
            }
        }
    } else if let Some(caption) = table
        .caption
        .as_deref()
        .filter(|text| !text.trim().is_empty())
    {
        append_limited(&mut out, "**")?;
        append_escaped(&mut out, caption)?;
        append_limited(&mut out, "**\n\n")?;
    }
    let rendered = table_markdown(table, depth)?;
    if !rendered.is_empty() {
        append_limited(&mut out, &rendered)?;
    }
    Ok(out)
}

/// Render blocks in their input order with a fixed output and nesting budget.
pub fn blocks_to_markdown(blocks: &[IrBlock]) -> Result<String, KordocError> {
    let mut out = String::new();
    render_blocks(blocks, 0, &mut out)?;
    Ok(out.trim().to_owned())
}

fn render_blocks(blocks: &[IrBlock], depth: usize, out: &mut String) -> Result<(), KordocError> {
    if depth > MAX_BLOCK_DEPTH {
        return Err(output_too_large());
    }
    for (index, block) in blocks.iter().enumerate() {
        if index > 0 && !out.ends_with('\n') {
            append_limited(out, "\n")?;
        }
        match block.kind {
            IrBlockType::Heading => {
                let level = block
                    .level
                    .filter(|level| *level != 0)
                    .unwrap_or(2)
                    .clamp(1, 6) as usize;
                let text = block.text.as_deref().unwrap_or("");
                if text.is_empty() {
                    continue;
                }
                append_limited(out, &"#".repeat(level))?;
                append_limited(out, " ")?;
                append_escaped(out, text)?;
                if let Some(note) = block.footnote_text.as_deref() {
                    append_limited(out, " (주: ")?;
                    append_escaped(out, note)?;
                    append_limited(out, ")")?;
                }
                append_limited(out, "\n\n")?;
            }
            IrBlockType::Image => {
                if let Some(src) = block.text.as_deref() {
                    if !out.is_empty() {
                        append_limited(out, "\n")?;
                    }
                    append_limited(out, "![image](")?;
                    append_limited(out, src)?;
                    append_limited(out, ")\n\n")?;
                }
            }
            IrBlockType::Separator => {
                if !out.is_empty() {
                    append_limited(out, "\n")?;
                }
                append_limited(out, "---\n\n")?;
            }
            IrBlockType::List => {
                if let Some(text) = block.text.as_deref() {
                    let text = sanitize_text(text);
                    let numbered = block.list_type == Some(kordoc_ir::ListType::Ordered)
                        && is_ordered_marker(&text);
                    let bulleted = block.list_type != Some(kordoc_ir::ListType::Ordered)
                        && text.trim_start().starts_with("- ");
                    if !numbered && !bulleted {
                        append_limited(
                            out,
                            if block.list_type == Some(kordoc_ir::ListType::Ordered) {
                                "1. "
                            } else {
                                "- "
                            },
                        )?;
                    }
                    append_escaped(out, &text)?;
                    if let Some(note) = block.footnote_text.as_deref() {
                        append_limited(out, " (주: ")?;
                        append_escaped(out, note)?;
                        append_limited(out, ")")?;
                    }
                    append_limited(out, "\n")?;
                    if let Some(children) = block.children.as_deref() {
                        render_blocks(children, depth + 1, out)?;
                    }
                }
            }
            IrBlockType::Table => {
                if let Some(table) = block.table.as_ref() {
                    let rendered = table_markdown_with_caption(table, depth + 1)?;
                    if !rendered.is_empty() {
                        append_limited(out, &rendered)?;
                        append_limited(out, "\n\n")?;
                    }
                }
            }
            IrBlockType::Paragraph => {
                if let Some(text) = block.text.as_deref() {
                    let mut rendered = if let Some(spans) =
                        block.spans.as_deref().filter(|spans| !spans.is_empty())
                    {
                        render_spans(spans)?
                    } else {
                        escape_gfm(text)?
                    };
                    if let Some(href) = block.href.as_deref().and_then(sanitize_href) {
                        rendered = format!("[{rendered}]({href})");
                    }
                    if let Some(note) = block.footnote_text.as_deref() {
                        rendered.push_str(" (주: ");
                        rendered.push_str(&escape_gfm(note)?);
                        rendered.push(')');
                    }
                    if block.quote == Some(true) {
                        append_limited(out, "> ")?;
                    }
                    append_limited(out, &rendered)?;
                    append_limited(out, "\n\n")?;
                }
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::blocks_to_markdown;
    use kordoc_ir::{ErrorCode, IrBlock, IrSpan};

    fn block(json: &str) -> IrBlock {
        serde_json::from_str(json).expect("valid IR block")
    }

    #[test]
    fn markdown_preserves_block_order_and_escaping() {
        let blocks = [
            block(r#"{"type":"heading","level":1,"text":"Title"}"#),
            block(r#"{"type":"paragraph","text":"A | B"}"#),
            block(r#"{"type":"list","listType":"unordered","text":"- item"}"#),
            block(r#"{"type":"separator"}"#),
        ];
        assert_eq!(
            blocks_to_markdown(&blocks).unwrap(),
            "# Title\n\nA \\| B\n\n- item\n\n---"
        );
    }

    #[test]
    fn explicit_zero_heading_level_uses_oracle_default_level() {
        let block = block(r#"{"type":"heading","level":0,"text":"Fallback title"}"#);
        assert_eq!(blocks_to_markdown(&[block]).unwrap(), "## Fallback title");
    }

    #[test]
    fn markdown_renders_nested_table_as_html() {
        let block = block(
            r#"{"type":"table","table":{"rows":1,"cols":1,"hasHeader":false,"cells":[[{"text":"outer","colSpan":1,"rowSpan":1,"blocks":[{"type":"table","table":{"rows":1,"cols":1,"hasHeader":false,"cells":[[{"text":"inner","colSpan":1,"rowSpan":1}]]}}]}]]}}"#,
        );
        let output = blocks_to_markdown(&[block]).unwrap();
        assert_eq!(output.matches("<table>").count(), 2, "{output}");
        assert!(output.contains("<th>inner</th>"), "{output}");
    }

    #[test]
    fn markdown_rejects_unsafe_link_scheme() {
        let block = block(r#"{"type":"paragraph","text":"open","href":"javascript:alert(1)"}"#);
        assert_eq!(blocks_to_markdown(&[block]).unwrap(), "open");
        let too_large = IrBlock::paragraph("x".repeat(super::MAX_MARKDOWN_BYTES + 1));
        assert_eq!(
            blocks_to_markdown(&[too_large]).unwrap_err().code,
            ErrorCode::OutputTooLarge
        );
    }

    #[test]
    fn markdown_preserves_preescaped_cells_and_math_spans() {
        assert_eq!(super::escape_gfm("a \\| b").unwrap(), "a \\| b");
        assert_eq!(super::escape_gfm("$a|b<c$ | d").unwrap(), "$a|b<c$ \\| d");
        assert_eq!(
            super::escape_gfm("<u>underline</u>").unwrap(),
            "<u>underline</u>"
        );
        assert_eq!(
            super::escape_gfm("symbol \u{f080f} and unknown \u{f1234}").unwrap(),
            "symbol ━ and unknown"
        );
    }

    #[test]
    fn spans_keep_edge_whitespace_and_apply_strike_after_style() {
        let block = block(
            r#"{"type":"paragraph","text":" before middle after ","spans":[{"text":" before ","bold":true},{"text":"middle","bold":true,"strike":true},{"text":" after ","italic":true}]}"#,
        );
        assert_eq!(
            blocks_to_markdown(&[block]).unwrap(),
            "**before** ~~**middle**~~ *after*"
        );
    }

    #[test]
    fn underlined_span_keeps_edge_whitespace_outside_markup() {
        let spans = [IrSpan {
            text: "  underlined  ".into(),
            underline: Some(true),
            ..Default::default()
        }];
        assert_eq!(
            super::render_spans(&spans).unwrap(),
            "  <u>underlined</u>  "
        );
    }

    #[test]
    fn separator_after_list_keeps_block_spacing() {
        let blocks = [
            block(r#"{"type":"list","listType":"unordered","text":"item"}"#),
            block(r#"{"type":"separator"}"#),
        ];
        assert_eq!(blocks_to_markdown(&blocks).unwrap(), "- item\n\n---");
    }

    #[test]
    fn image_and_separator_spacing_matches_oracle_block_lines() {
        let blocks = [
            block(r#"{"type":"list","listType":"unordered","text":"item"}"#),
            block(r#"{"type":"image","text":"asset(image).png"}"#),
            block(r#"{"type":"paragraph","text":"content"}"#),
            block(r#"{"type":"separator"}"#),
        ];
        assert_eq!(
            blocks_to_markdown(&blocks).unwrap(),
            "- item\n\n![image](asset(image).png)\n\ncontent\n\n\n---"
        );
    }

    #[test]
    fn list_items_are_single_spaced_and_only_exact_numbers_are_preserved() {
        let blocks = [
            block(r#"{"type":"list","listType":"ordered","text":"2024 annual"}"#),
            block(r#"{"type":"list","listType":"ordered","text":"2. second"}"#),
            block(r#"{"type":"list","listType":"unordered","text":"third"}"#),
        ];
        assert_eq!(
            blocks_to_markdown(&blocks).unwrap(),
            "1. 2024 annual\n2. second\n- third"
        );
    }

    #[test]
    fn one_cell_table_styles_numbered_and_hangul_items_and_uses_cell_text() {
        let block = block(
            r#"{"type":"table","table":{"rows":1,"cols":1,"hasHeader":false,"cells":[[{"text":"1. numbered\n가. Korean\nplain","colSpan":1,"rowSpan":1,"blocks":[{"type":"paragraph","text":"stale block text"}]}]]}}"#,
        );
        assert_eq!(
            blocks_to_markdown(&[block]).unwrap(),
            "**1. numbered**\n  가. Korean\nplain"
        );
    }

    #[test]
    fn markdown_normalizes_verified_pua_shape_alt_and_short_korean_spacing() {
        let cases = [
            ("\u{f020}", "\u{f020}"),
            ("\u{f021}", "🖉"),
            ("\u{f08c}", "❶"),
            ("\u{f028b}", ""),
            ("수식 입니다.\n표 입니다.\n실제 본문", "실제 본문"),
            ("현 장 대 응 단 장", "현장대응단장"),
            ("년 월 일", "년 월 일"),
            (
                "본문\n4점별 입니다.\n붙임 문서는 표 입니다.",
                "본문\n붙임 문서는 표 입니다.",
            ),
        ];
        for (input, expected) in cases {
            assert_eq!(
                super::escape_gfm(input).unwrap(),
                expected,
                "input={input:?}"
            );
        }
    }

    #[test]
    fn generated_bmp_wingdings_parity_matches_oracle_indices() {
        let overrides = [
            (0x22u32, "✂"),
            (0x36, "⌛"),
            (0x45, "☜"),
            (0x46, "☞"),
            (0x47, "☝"),
            (0x48, "☟"),
            (0x4a, "☺"),
            (0x4e, "☠"),
            (0x52, "☼"),
            (0x54, "❄"),
            (0x58, "✠"),
            (0x59, "✡"),
            (0x6c, "●"),
            (0x6d, "●"),
            (0x6e, "■"),
            (0x6f, "□"),
            (0x70, "□"),
            (0x71, "□"),
            (0x72, "□"),
            (0x73, "⬧"),
            (0x74, "⧫"),
            (0x75, "◆"),
            (0x76, "❖"),
            (0x77, "⬥"),
            (0x9e, "·"),
            (0x9f, "•"),
            (0xa0, "·"),
            (0xa1, "⚪"),
            (0xa2, "○"),
            (0xa3, "○"),
            (0xa4, "◉"),
            (0xa5, "◎"),
            (0xa7, "▪"),
            (0xa8, "◻"),
            (0xaa, "✦"),
            (0xab, "★"),
            (0xac, "✶"),
            (0xad, "✴"),
            (0xae, "✹"),
            (0xe8, "➔"),
            (0xef, "⇦"),
            (0xf0, "⇨"),
            (0xf1, "⇧"),
            (0xf2, "⇩"),
            (0xfb, "✗"),
            (0xfc, "✔"),
            (0xfd, "☒"),
            (0xfe, "☑"),
        ];
        for low in 0x20u32..=0xff {
            let code = 0xf000 + low;
            let ch = char::from_u32(code).unwrap();
            let expected = overrides
                .iter()
                .find_map(|(key, value)| (*key == low).then(|| (*value).to_owned()))
                .or_else(|| {
                    (low >= 0x21)
                        .then(|| super::WINGDINGS[(low - 0x21) as usize].to_owned())
                        .filter(|value| !value.is_empty())
                })
                .unwrap_or_else(|| ch.to_string());
            assert_eq!(
                super::normalize_pua(&ch.to_string()),
                expected,
                "U+{code:04X}"
            );
        }
    }

    #[test]
    fn generated_supplementary_pua_parity_matches_verified_mappings() {
        let mappings = [
            (0xF003B, "↓"),
            (0xF02EF, "·"),
            (0xF0854, "《"),
            (0xF0855, "》"),
            (0xF00DA, "▸"),
            (0xF080F, "━"),
            (0xF0827, "■"),
            (0xF03C5, "□"),
            (0xF012B, "(인)"),
            (0xF00E1, "(인)"),
            (0xF02FC, "►"),
            (0xF031C, "■"),
            (0xF03A0, "↵"),
            (0xF03EF, "한"),
            (0xF03F0, "글"),
            (0xF03F1, "과"),
            (0xF03F2, "컴"),
            (0xF03F3, "퓨"),
            (0xF03F4, "터"),
            (0xF0090, "✺"),
            (0xF0288, "⓪"),
            (0xF0289, "①"),
            (0xF028A, "②"),
            (0xF028C, "④"),
            (0xF028D, "⑤"),
            (0xF028E, "⑥"),
            (0xF028F, "⑦"),
            (0xF0290, "⑧"),
            (0xF0291, "⑨"),
            (0xF02EC, "◇"),
            (0xF02FB, "▸"),
            (0xF03A7, "⊟"),
            (0xF03A8, "⊞"),
            (0xF03DA, "□"),
            (0xF03FF, "□"),
            (0xF0806, "┌"),
            (0xF0807, "┬"),
            (0xF0808, "┐"),
            (0xF080C, "└"),
            (0xF080E, "┘"),
            (0xF0810, "│"),
            (0xF081C, "┈"),
            (0xF0832, "═"),
            (0xF0848, "━"),
        ];
        for (code, expected) in mappings {
            let input = char::from_u32(code).unwrap().to_string();
            assert_eq!(super::normalize_pua(&input), expected, "U+{code:05X}");
        }
        for code in [0xF0000, 0xF028B, 0xF09FF, 0xF0A00, 0xFFFFD] {
            let input = char::from_u32(code).unwrap().to_string();
            assert_eq!(super::normalize_pua(&input), "", "unmapped U+{code:05X}");
        }
    }

    #[test]
    fn generated_short_korean_spacing_parity_matches_oracle_rules() {
        let cases = [
            ("현 장 대 응 단 장", "현장대응단장"),
            ("가 나 다", "가나다"),
            ("가 나  다", "가나다"),
            ("년 월 일", "년 월 일"),
            ("시 분", "시 분"),
            ("가 나 123", "가 나 123"),
            ("N = 잠수펌프의 수", "N = 잠수펌프의 수"),
        ];
        for (input, expected) in cases {
            assert_eq!(super::sanitize_text(input), expected, "input={input:?}");
        }
    }

    #[test]
    fn nested_html_keeps_caption_footnote_and_empty_blocks_fallback() {
        let block = block(
            r#"{"type":"table","table":{"rows":1,"cols":2,"hasHeader":false,"cells":[[{"text":"fallback","colSpan":1,"rowSpan":1,"blocks":[]},{"text":"holder","colSpan":1,"rowSpan":1,"blocks":[{"type":"paragraph","text":"note content","footnoteText":"42"},{"type":"table","table":{"rows":1,"cols":1,"hasHeader":false,"caption":"inner caption","cells":[[{"text":"inner","colSpan":1,"rowSpan":1}]]}}]}]],"captionBlocks":[{"type":"paragraph","text":"caption"},{"type":"table","table":{"rows":1,"cols":1,"hasHeader":false,"caption":"inner caption","cells":[[{"text":"inner","colSpan":1,"rowSpan":1}]]}}]}}"#,
        );
        let rendered = blocks_to_markdown(&[block]).unwrap();
        assert!(rendered.contains("**caption**"), "{rendered}");
        assert!(rendered.contains("**inner caption**"), "{rendered}");
        assert!(rendered.contains("fallback"), "{rendered}");
        assert!(rendered.contains("note content (주: 42)"), "{rendered}");
    }

    #[test]
    fn renderer_accepts_more_than_default_row_limit_within_cell_and_output_budgets() {
        let rows = (0..10_001)
            .map(|_| {
                vec![super::super::table::builder::CellContext {
                    text: "x".into(),
                    col_span: 1,
                    row_span: 1,
                    col_addr: None,
                    row_addr: None,
                }]
            })
            .collect::<Vec<_>>();
        let table = super::super::table::builder::build_table(
            &rows,
            super::super::table::builder::BuildTableOptions {
                max_rows: Some(10_001),
                ..Default::default()
            },
        )
        .unwrap();
        let block = IrBlock {
            kind: kordoc_ir::IrBlockType::Table,
            table: Some(table),
            ..Default::default()
        };
        assert_eq!(
            blocks_to_markdown(&[block]).unwrap().lines().count(),
            10_001
        );
    }
}
