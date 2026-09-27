//! Input validation and deterministic normalization (spec §9.1).

use serde::Serialize;
use thiserror::Error;

/// Input limit in UTF-16 code units, matching JavaScript `String.length`.
pub const MAX_TEXT_UTF16: usize = 200_000;

#[derive(Debug, Clone, PartialEq, Eq, Error, Serialize)]
#[serde(tag = "code", rename_all = "SCREAMING_SNAKE_CASE")]
pub enum TextError {
    #[error("There is no text to read.")]
    NoText,
    #[error("The text is longer than {limit} characters.")]
    TextTooLong { length: usize, limit: usize },
}

pub fn utf16_len(s: &str) -> usize {
    s.chars().map(char::len_utf16).sum()
}

/// Rejects empty or over-limit input. Never truncates silently.
pub fn validate(text: &str) -> Result<(), TextError> {
    if text.trim().is_empty() {
        return Err(TextError::NoText);
    }
    let length = utf16_len(text);
    if length > MAX_TEXT_UTF16 {
        return Err(TextError::TextTooLong { length, limit: MAX_TEXT_UTF16 });
    }
    Ok(())
}

/// Returns the longest prefix within `limit` UTF-16 units, preferring to end
/// at a sentence boundary. Used only after the user consents to reading the
/// first portion.
pub fn truncate_to_limit(text: &str, limit: usize) -> &str {
    let mut units = 0;
    let mut hard_end = text.len();
    for (i, c) in text.char_indices() {
        if units + c.len_utf16() > limit {
            hard_end = i;
            break;
        }
        units += c.len_utf16();
    }
    let head = &text[..hard_end];
    if hard_end == text.len() {
        return head;
    }
    // Prefer the last sentence terminator followed by whitespace.
    let bytes = head.as_bytes();
    for i in (0..bytes.len().saturating_sub(1)).rev() {
        if matches!(bytes[i], b'.' | b'!' | b'?') && bytes[i + 1].is_ascii_whitespace() {
            if i + 1 >= hard_end / 2 {
                return &head[..i + 1];
            }
            break;
        }
    }
    head
}

pub fn word_count(text: &str) -> usize {
    text.split_whitespace().filter(|w| w.chars().any(char::is_alphanumeric)).count()
}

/// Approximate listening time: words / (180 × rate) minutes (spec §10.4).
pub fn estimate_minutes(words: usize, rate: f32) -> f32 {
    let rate = if rate.is_finite() && rate > 0.0 { rate } else { 1.0 };
    words as f32 / (180.0 * rate)
}

/// Normalized text plus a mapping from each normalized byte offset to the
/// original byte offset it came from.
#[derive(Debug, Clone)]
pub struct Normalized {
    pub text: String,
    /// `origin[i]` is the original byte offset of the normalized char that
    /// starts at normalized byte `i`; the final entry maps `text.len()`.
    origin: Vec<(usize, usize)>,
    original_len: usize,
}

impl Normalized {
    /// Maps the exclusive end of a normalized range to the end of the last
    /// original char it covers, excluding trailing layout characters.
    pub fn to_original_end(&self, norm_end: usize) -> usize {
        match self.text[..norm_end.min(self.text.len())].chars().next_back() {
            Some(c) => self.to_original(norm_end - c.len_utf8()) + c.len_utf8(),
            None => self.to_original(0),
        }
    }

    /// Maps a normalized byte offset back to the original text.
    pub fn to_original(&self, norm: usize) -> usize {
        if norm >= self.text.len() {
            return self.original_len;
        }
        match self.origin.binary_search_by_key(&norm, |&(n, _)| n) {
            Ok(i) => self.origin[i].1,
            Err(i) => self.origin[i.saturating_sub(1)].1,
        }
    }
}

/// Markdown syntax found by [`markdown_markup`]: chars that must not be
/// spoken, and line breaks that end a block (heading, list item) and so
/// read as a paragraph break.
struct Markup {
    hidden: Vec<bool>,
    block_end: Vec<bool>,
}

/// Flags Markdown syntax characters (`## `, `**`, `` ` ``, `- `, `> `,
/// `[text](url)` …) so they are skipped instead of read aloud. Indices are
/// positions in `chars`.
fn markdown_markup(chars: &[(usize, char)]) -> Markup {
    let n = chars.len();
    let mut hidden = vec![false; n];
    let mut block_end = vec![false; n];
    let c = |i: usize| chars.get(i).map(|&(_, c)| c);
    let is_space = |ch: Option<char>| ch.is_none_or(|ch| ch == ' ' || ch == '\t');

    // Line-level syntax.
    let mut start = 0;
    while start < n {
        let end = (start..n).find(|&i| c(i) == Some('\n')).unwrap_or(n);
        let mut p = start;
        while is_space(c(p)) && p < end {
            p += 1;
        }
        let line: String = chars[p..end].iter().map(|&(_, ch)| ch).collect();
        let trimmed = line.trim_end();
        let rule = trimmed.chars().filter(|ch| !ch.is_whitespace()).collect::<String>();
        let is_rule = rule.len() >= 3
            && ['-', '*', '_'].iter().any(|&m| rule.chars().all(|ch| ch == m));
        let is_fence = trimmed.starts_with("```") || trimmed.starts_with("~~~");
        let is_table_rule = trimmed.starts_with('|')
            && trimmed.contains('-')
            && trimmed.chars().all(|ch| matches!(ch, '|' | '-' | ':' | ' ' | '\t'));
        if is_rule || is_fence || is_table_rule {
            hidden[p..end].fill(true);
        } else {
            // Blockquote markers, possibly nested.
            while c(p) == Some('>') {
                hidden[p] = true;
                p += 1;
                while is_space(c(p)) && p < end {
                    p += 1;
                }
            }
            let hashes = (p..end).take_while(|&i| c(i) == Some('#')).count();
            let bullet = matches!(c(p), Some('-' | '*' | '+')) && is_space(c(p + 1));
            let digits = (p..end).take_while(|&i| c(i).is_some_and(|x| x.is_ascii_digit())).count();
            let numbered = digits > 0 && matches!(c(p + digits), Some('.' | ')')) && is_space(c(p + digits + 1));
            let heading = (1..=6).contains(&hashes) && is_space(c(p + hashes));
            if heading || bullet || numbered {
                // A block stands alone: break before and after it.
                if start > 0 {
                    block_end[start - 1] = true;
                }
                if end < n {
                    block_end[end] = true;
                }
            }
            if heading {
                hidden[p..p + hashes].fill(true);
                // Optional closing hashes: "## Title ##".
                let mut q = end;
                while q > p + hashes && is_space(c(q - 1)) {
                    q -= 1;
                }
                while q > p + hashes && c(q - 1) == Some('#') {
                    q -= 1;
                    hidden[q] = true;
                }
            } else if bullet {
                hidden[p] = true;
            }
            if trimmed.starts_with('|') {
                for i in p..end {
                    if c(i) == Some('|') {
                        hidden[i] = true;
                    }
                }
            }
        }
        start = end + 1;
    }

    // Inline syntax.
    let alnum = |ch: Option<char>| ch.is_some_and(char::is_alphanumeric);
    let mut i = 0;
    while i < n {
        if hidden[i] {
            i += 1;
            continue;
        }
        match chars[i].1 {
            '`' => hidden[i] = true,
            '*' => {
                // Keep arithmetic such as "2*3".
                let digits = c(i.wrapping_sub(1)).is_some_and(|p| p.is_ascii_digit())
                    && c(i + 1).is_some_and(|x| x.is_ascii_digit());
                hidden[i] = !digits;
            }
            '_' | '~' => {
                let run = (i..n).take_while(|&j| c(j) == Some(chars[i].1)).count();
                // Keep snake_case and a lone "~5 minutes".
                let inside_word = run == 1 && alnum(c(i.wrapping_sub(1))) && alnum(c(i + 1));
                let keep = inside_word || (chars[i].1 == '~' && run == 1);
                if !keep {
                    hidden[i..i + run].fill(true);
                }
                i += run;
                continue;
            }
            '[' => {
                // [text](url) and ![alt](url): speak only the text.
                let close = (i + 1..n).take_while(|&j| c(j) != Some('\n')).find(|&j| c(j) == Some(']'));
                if let Some(close) = close.filter(|&j| c(j + 1) == Some('(')) {
                    let paren = (close + 2..n)
                        .take_while(|&j| !matches!(c(j), Some('\n' | ' ')))
                        .find(|&j| c(j) == Some(')'));
                    if let Some(paren) = paren {
                        hidden[i] = true;
                        if i > 0 && c(i - 1) == Some('!') {
                            hidden[i - 1] = true;
                        }
                        hidden[close..=paren].fill(true);
                        i += 1;
                        continue;
                    }
                }
            }
            _ => {}
        }
        i += 1;
    }
    Markup { hidden, block_end }
}

/// Normalizes line endings, control characters, and repeated layout
/// whitespace while retaining paragraph boundaries (blank lines). Markdown
/// syntax is dropped so it is not read aloud.
pub fn normalize(original: &str) -> Normalized {
    let mut text = String::with_capacity(original.len());
    let mut origin = Vec::with_capacity(original.len() / 2);
    let mut pending_space: Option<usize> = None;
    let mut pending_newlines = 0usize;
    let mut pending_at = 0usize;

    let all: Vec<(usize, char)> = original.char_indices().collect();
    let markup = markdown_markup(&all);
    let mut chars = all.iter().copied().enumerate().peekable();
    while let Some((k, (i, c))) = chars.next() {
        if markup.hidden[k] {
            continue;
        }
        let c = if c == '\r' {
            if matches!(chars.peek(), Some((_, (_, '\n')))) {
                continue;
            }
            '\n'
        } else {
            c
        };
        match c {
            '\n' | '\u{2028}' | '\u{2029}' => {
                if pending_newlines == 0 {
                    pending_at = i;
                }
                pending_newlines += if c == '\u{2029}' || markup.block_end[k] { 2 } else { 1 };
                pending_space = None;
            }
            c if c.is_whitespace() => {
                if pending_newlines == 0 && pending_space.is_none() {
                    pending_space = Some(i);
                }
            }
            c if c.is_control() || matches!(c, '\u{200B}' | '\u{FEFF}' | '\u{00AD}') => {}
            c => {
                if !text.is_empty() {
                    if pending_newlines >= 2 {
                        origin.push((text.len(), pending_at));
                        text.push_str("\n\n");
                    } else if pending_newlines == 1 || pending_space.is_some() {
                        origin.push((text.len(), pending_space.unwrap_or(pending_at)));
                        // A single line break inside a paragraph is layout, not content.
                        text.push(' ');
                    }
                }
                pending_newlines = 0;
                pending_space = None;
                origin.push((text.len(), i));
                text.push(c);
            }
        }
    }
    Normalized { text, origin, original_len: original.len() }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_empty_and_whitespace() {
        assert_eq!(validate(""), Err(TextError::NoText));
        assert_eq!(validate(" \n\t "), Err(TextError::NoText));
        assert!(validate("Hi").is_ok());
    }

    #[test]
    fn counts_utf16_not_bytes() {
        // U+1F600 is one scalar, four UTF-8 bytes, two UTF-16 units.
        assert_eq!(utf16_len("😀"), 2);
        let at_limit = "a".repeat(MAX_TEXT_UTF16);
        assert!(validate(&at_limit).is_ok());
        let over = format!("{at_limit}😀");
        assert!(matches!(validate(&over), Err(TextError::TextTooLong { .. })));
    }

    #[test]
    fn truncation_stays_on_char_boundary() {
        let s = "😀😀😀";
        assert_eq!(truncate_to_limit(s, 3), "😀");
        let s = "One sentence here. Another one that goes on.";
        assert_eq!(truncate_to_limit(s, 30), "One sentence here.");
    }

    #[test]
    fn normalizes_whitespace_and_keeps_paragraphs() {
        let n = normalize("Hello   world.\r\nSame para.\r\n\r\n\r\nNew  para.\u{0007}");
        assert_eq!(n.text, "Hello world. Same para.\n\nNew para.");
    }

    #[test]
    fn strips_markdown_syntax() {
        let n = normalize("## Setup\r\nUse **bold** and *em*, `code`.\n- one\n- two\n> quoted\n\n---\n\nSee [docs](https://x.y/z).");
        assert_eq!(n.text, "Setup\n\nUse bold and em, code.\n\none\n\ntwo\n\nquoted\n\nSee docs.");
        let n = normalize("Steps:\n1. First\n2. Second");
        assert_eq!(n.text, "Steps:\n\n1. First\n\n2. Second");
        let n = normalize("snake_case, 2*3, C# and #1, ~5 min, __under__ ~~gone~~");
        assert_eq!(n.text, "snake_case, 2*3, C# and #1, ~5 min, under gone");
    }

    #[test]
    fn markdown_maps_back_to_original() {
        let src = "# Title\n\n**Bold** text.";
        let n = normalize(src);
        assert_eq!(n.text, "Title\n\nBold text.");
        let b = n.text.find("Bold").unwrap();
        assert_eq!(&src[n.to_original(b)..], "Bold** text.");
        assert_eq!(&src[n.to_original(0)..], "Title\n\n**Bold** text.");
    }

    #[test]
    fn maps_back_to_original() {
        let src = "  Alpha\t\tbeta";
        let n = normalize(src);
        assert_eq!(n.text, "Alpha beta");
        let b = n.text.find("beta").unwrap();
        assert_eq!(&src[n.to_original(b)..], "beta");
        assert_eq!(n.to_original(0), 2);
        assert_eq!(n.to_original(n.text.len()), src.len());
    }

    #[test]
    fn estimate_divides_by_rate() {
        assert_eq!(estimate_minutes(360, 1.0), 2.0);
        assert_eq!(estimate_minutes(360, 2.0), 1.0);
    }
}
