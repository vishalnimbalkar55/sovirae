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

/// Normalizes line endings, control characters, and repeated layout
/// whitespace while retaining paragraph boundaries (blank lines).
pub fn normalize(original: &str) -> Normalized {
    let mut text = String::with_capacity(original.len());
    let mut origin = Vec::with_capacity(original.len() / 2);
    let mut pending_space: Option<usize> = None;
    let mut pending_newlines = 0usize;
    let mut pending_at = 0usize;

    let mut chars = original.char_indices().peekable();
    while let Some((i, c)) = chars.next() {
        let c = if c == '\r' {
            if matches!(chars.peek(), Some((_, '\n'))) {
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
                pending_newlines += if c == '\u{2029}' { 2 } else { 1 };
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
