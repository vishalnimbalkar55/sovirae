//! SpeechDocument: normalized text, source mapping, and ordered segments
//! produced by deterministic sentence rules (spec §9.1–9.2).

use std::ops::Range;

use serde::Serialize;
use unicode_segmentation::UnicodeSegmentation;

use crate::text::{normalize, Normalized};

/// Words that end with a period but rarely end a sentence.
const ABBREVIATIONS: &[&str] = &[
    "mr", "mrs", "ms", "dr", "prof", "sr", "jr", "st", "mt", "vs", "etc", "e.g", "i.e", "fig",
    "no", "vol", "approx", "inc", "ltd", "co", "corp", "dept", "est", "u.s", "u.k", "a.m", "p.m",
    "jan", "feb", "mar", "apr", "jun", "jul", "aug", "sep", "sept", "oct", "nov", "dec",
];

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Segment {
    pub id: usize,
    /// Index of the sentence this segment belongs to; long sentences are
    /// split into several segments sharing one sentence ID.
    pub sentence_id: usize,
    /// Byte range in the normalized text.
    pub normalized: Range<usize>,
    /// Byte range in the original text.
    pub original: Range<usize>,
    /// Starts a new paragraph.
    pub paragraph_start: bool,
}

#[derive(Debug, Clone)]
pub struct SpeechDocument {
    pub original: String,
    normalized: Normalized,
    pub segments: Vec<Segment>,
}

impl SpeechDocument {
    /// Builds the index. `max_segment_chars` is the engine's input limit.
    pub fn new(original: String, max_segment_chars: usize) -> Self {
        let normalized = normalize(&original);
        let mut segments = Vec::new();
        let mut sentence_id = 0;
        let text = normalized.text.as_str();

        let mut para_start = 0;
        for para_end in paragraph_ends(text) {
            let para = &text[para_start..para_end];
            let mut first_in_para = true;
            for sentence in sentences(para) {
                let sentence = shift(sentence, para_start);
                for piece in split_long(text, sentence, max_segment_chars.max(40)) {
                    let piece = trim_range(text, piece);
                    if piece.is_empty() {
                        continue;
                    }
                    segments.push(Segment {
                        id: segments.len(),
                        sentence_id,
                        original: normalized.to_original(piece.start)
                            ..normalized.to_original_end(piece.end),
                        normalized: piece,
                        paragraph_start: first_in_para,
                    });
                    first_in_para = false;
                }
                sentence_id += 1;
            }
            para_start = (para_end + 2).min(text.len());
        }
        Self { original, normalized, segments }
    }

    pub fn normalized_text(&self) -> &str {
        &self.normalized.text
    }

    pub fn segment_text(&self, id: usize) -> &str {
        &self.normalized.text[self.segments[id].normalized.clone()]
    }
}

fn paragraph_ends(text: &str) -> Vec<usize> {
    let mut ends: Vec<usize> = text.match_indices("\n\n").map(|(i, _)| i).collect();
    ends.push(text.len());
    ends
}

fn shift(r: Range<usize>, by: usize) -> Range<usize> {
    r.start + by..r.end + by
}

fn trim_range(text: &str, r: Range<usize>) -> Range<usize> {
    let s = &text[r.clone()];
    let lead = s.len() - s.trim_start().len();
    let trail = s.len() - s.trim_end().len();
    if lead == s.len() {
        return r.start..r.start;
    }
    r.start + lead..r.end - trail
}

/// UAX #29 sentence bounds, merged back together after abbreviations and
/// initials so "Dr. Smith" or "J. R. R. Tolkien" stay in one sentence.
fn sentences(para: &str) -> Vec<Range<usize>> {
    let mut out: Vec<Range<usize>> = Vec::new();
    let mut merge_next = false;
    for (start, s) in para.split_sentence_bound_indices() {
        let range = start..start + s.len();
        if merge_next {
            if let Some(last) = out.last_mut() {
                last.end = range.end;
            }
        } else {
            out.push(range);
        }
        merge_next = ends_with_abbreviation(s.trim_end());
    }
    out
}

fn ends_with_abbreviation(s: &str) -> bool {
    let Some(body) = s.strip_suffix('.') else { return false };
    let last = body.rsplit(|c: char| c.is_whitespace() || c == '(' || c == '"').next().unwrap_or("");
    if last.chars().count() == 1 && last.chars().all(char::is_uppercase) {
        return true; // An initial such as "J."
    }
    let lower = last.to_lowercase();
    ABBREVIATIONS.contains(&lower.as_str())
}

/// Splits a sentence longer than `max` chars at the best phrase boundary,
/// falling back to whitespace, then to a hard char boundary.
fn split_long(text: &str, r: Range<usize>, max: usize) -> Vec<Range<usize>> {
    let mut out = Vec::new();
    let mut start = r.start;
    while text[start..r.end].chars().count() > max {
        let window_end = text[start..]
            .char_indices()
            .nth(max)
            .map(|(i, _)| start + i)
            .unwrap_or(r.end);
        let window = &text[start..window_end];
        let min = window.len() / 3;
        let cut = [&[';', ':', '—', '–'][..], &[','][..], &[' '][..]]
            .iter()
            .find_map(|set| {
                window
                    .char_indices()
                    .rev()
                    .find(|&(i, c)| i >= min && set.contains(&c))
                    .map(|(i, c)| start + i + c.len_utf8())
            })
            .unwrap_or(window_end);
        out.push(start..cut);
        start = cut;
    }
    out.push(start..r.end);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn texts(doc: &SpeechDocument) -> Vec<&str> {
        (0..doc.segments.len()).map(|i| doc.segment_text(i)).collect()
    }

    #[test]
    fn splits_sentences_and_keeps_abbreviations() {
        let doc = SpeechDocument::new(
            "Dr. Smith paid $3.50 on Jan. 5. Was it worth it? J. R. R. Tolkien said yes!".into(),
            400,
        );
        assert_eq!(
            texts(&doc),
            ["Dr. Smith paid $3.50 on Jan. 5.", "Was it worth it?", "J. R. R. Tolkien said yes!"]
        );
    }

    #[test]
    fn marks_paragraphs() {
        let doc = SpeechDocument::new("First one. Second.\n\n\nNew paragraph.".into(), 400);
        let starts: Vec<bool> = doc.segments.iter().map(|s| s.paragraph_start).collect();
        assert_eq!(starts, [true, false, true]);
    }

    #[test]
    fn splits_long_sentences_at_phrase_boundaries() {
        let long = format!("{}, {}; {}.", "a ".repeat(30).trim(), "b ".repeat(30).trim(), "c ".repeat(30).trim());
        let doc = SpeechDocument::new(long, 70);
        assert!(doc.segments.len() >= 2);
        assert!(doc.segments.iter().all(|s| s.sentence_id == 0));
        for i in 0..doc.segments.len() {
            assert!(doc.segment_text(i).chars().count() <= 70);
        }
        assert!(doc.segment_text(0).ends_with(';') || doc.segment_text(0).ends_with(','));
    }

    #[test]
    fn original_ranges_point_at_source_text() {
        let src = "  Hello   there.\r\n\r\nSecond  para.";
        let doc = SpeechDocument::new(src.into(), 400);
        assert_eq!(&src[doc.segments[0].original.clone()], "Hello   there.");
        assert_eq!(&src[doc.segments[1].original.clone()], "Second  para.");
    }

    #[test]
    fn handles_emoji_and_punctuation_only() {
        let doc = SpeechDocument::new("😀 Great. ...".into(), 400);
        assert!(!doc.segments.is_empty());
        let doc = SpeechDocument::new("   ".into(), 400);
        assert!(doc.segments.is_empty());
    }
}
