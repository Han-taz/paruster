//! Form-cell label heuristics shared with form recognition.

const LABEL_KEYWORDS: &[&str] = &[
    "성명",
    "이름",
    "주소",
    "전화",
    "전화번호",
    "휴대폰",
    "핸드폰",
    "연락처",
    "생년월일",
    "주민등록번호",
    "소속",
    "직위",
    "직급",
    "부서",
    "이메일",
    "팩스",
    "학교",
    "학년",
    "반",
    "번호",
    "신청인",
    "대표자",
    "담당자",
    "작성자",
    "확인자",
    "승인자",
    "일시",
    "날짜",
    "기간",
    "장소",
    "목적",
    "사유",
    "비고",
    "금액",
    "수량",
    "단가",
    "합계",
    "계",
    "소계",
    "등록기준지",
    "본적",
    "위임인",
    "청구사유",
    "소명자료",
];

const ENGLISH_LABEL_WORDS: &[&str] = &[
    "name",
    "date",
    "address",
    "tel",
    "phone",
    "mobile",
    "fax",
    "email",
    "e-mail",
    "dept",
    "department",
    "division",
    "title",
    "position",
    "grade",
    "rank",
    "birth",
    "nationality",
    "sex",
    "gender",
    "signature",
    "sign",
    "seal",
    "remarks",
    "note",
    "period",
    "place",
    "purpose",
    "reason",
    "amount",
    "total",
    "sum",
    "qty",
    "quantity",
    "unit",
    "no",
    "id",
    "passport",
];

const ENGLISH_STOPWORDS: &[&str] = &["of", "the", "and", "or", "in"];

/// Returns whether a cell looks like a Korean or English form label.
///
/// Trailing superscript and footnote markers are ignored, while numeric unit
/// values, sentence-like text, and company names are rejected.
pub fn is_label_cell(text: &str) -> bool {
    let trimmed = text
        .trim()
        .trim_end_matches(|ch| "¹²³⁴⁵⁶⁷⁸⁹⁰*※".contains(ch))
        .trim();
    let length = trimmed.chars().count();
    if trimmed.is_empty() || length > 30 {
        return false;
    }
    if LABEL_KEYWORDS
        .iter()
        .any(|keyword| trimmed.contains(keyword))
    {
        return true;
    }

    let compact: String = trimmed.chars().filter(|ch| !ch.is_whitespace()).collect();
    let compact_len = compact.chars().count();
    let korean_count = compact
        .chars()
        .filter(|ch| ('가'..='힣').contains(ch))
        .count();
    let allowed_short = compact.chars().all(|ch| {
        ('가'..='힣').contains(&ch)
            || ch.is_ascii_digit()
            || matches!(ch, '(' | ')' | '（' | '）' | '·' | ':' | '：' | '-')
    });
    if allowed_short
        && (2..=12).contains(&compact_len)
        && korean_count >= 2
        && (compact_len <= 8 || trimmed.split_whitespace().count() <= 2)
        && !is_numeric_value(&compact)
        && !has_sentence_ending(trimmed)
        && !compact.starts_with("(주)")
        && !compact.starts_with("（주）")
        && !compact.starts_with("주식회사")
    {
        return true;
    }

    if trimmed.ends_with(':') || trimmed.ends_with('：') {
        let body = trimmed.trim_end_matches([':', '：']);
        if !body.is_empty()
            && body.chars().all(|ch| {
                ('가'..='힣').contains(&ch) || ch.is_ascii_alphabetic() || ch.is_whitespace()
            })
        {
            return true;
        }
    }

    if length <= 20
        && trimmed.chars().all(|ch| {
            ch.is_ascii_alphabetic() || ch.is_whitespace() || matches!(ch, '.' | '/' | '&' | '-')
        })
    {
        let words: Vec<String> = trimmed
            .to_ascii_lowercase()
            .split(|ch: char| ch.is_whitespace() || matches!(ch, '/' | '&'))
            .filter(|word| !word.is_empty() && !ENGLISH_STOPWORDS.contains(word))
            .map(|word| word.strip_suffix('.').unwrap_or(word).to_owned())
            .collect();
        if (1..=3).contains(&words.len())
            && words
                .iter()
                .all(|word| ENGLISH_LABEL_WORDS.contains(&word.as_str()))
        {
            return true;
        }
    }
    false
}

fn is_numeric_value(text: &str) -> bool {
    let mut value = text.strip_prefix('제').unwrap_or(text);
    for unit in [
        "퍼센트",
        "개월",
        "주년",
        "차례",
        "원",
        "명",
        "건",
        "개",
        "회",
        "부",
        "매",
        "장",
        "점",
        "호",
        "번",
        "년",
        "월",
        "일",
        "시",
        "분",
        "초",
    ] {
        if let Some(prefix) = value.strip_suffix(unit) {
            value = prefix;
            break;
        }
    }
    let magnitude_end = value
        .trim_end_matches(|ch| "십백천만억조".contains(ch))
        .len();
    value = &value[..magnitude_end];
    if value.is_empty()
        || value.starts_with('.')
        || value.starts_with(',')
        || value.ends_with('.')
        || value.ends_with(',')
    {
        return false;
    }
    let mut saw_digit = false;
    let mut decimal_points = 0;
    for ch in value.chars() {
        if ch.is_ascii_digit() {
            saw_digit = true;
        } else if ch == '.' {
            decimal_points += 1;
            if decimal_points > 1 {
                return false;
            }
        } else if ch != ',' {
            return false;
        }
    }
    saw_digit
}

fn has_sentence_ending(text: &str) -> bool {
    [
        "입니다",
        "합니다",
        "습니다",
        "하세요",
        "십시오",
        "시오",
        "바랍니다",
        "바람",
        "할 것",
        "할것",
        "하며",
        "하고",
        "한다",
        "된다",
        "됨",
        "음",
        "임",
    ]
    .iter()
    .any(|ending| text.ends_with(ending))
}

#[cfg(test)]
mod tests {
    use super::is_label_cell;

    #[test]
    fn label_cell_rejects_numeric_values_and_sentences() {
        for value in [
            "6개월",
            "1억원",
            "5백만원",
            "2026년",
            "제3호",
            "신청하시기 바랍니다",
            "이 자료는 참고용입니다",
        ] {
            assert!(!is_label_cell(value), "{value:?} should not be a label");
        }
        assert!(!is_label_cell("(주)한글"));
        assert!(!is_label_cell("주식회사한글"));
    }

    #[test]
    fn label_cell_recognizes_keywords_short_forms_and_english() {
        for label in [
            "성명",
            "등록기준지²",
            "업 체 명",
            "신청인:",
            "Name",
            "Date of Birth",
            "Tel.",
        ] {
            assert!(is_label_cell(label), "{label:?} should be a label");
        }
    }
}
