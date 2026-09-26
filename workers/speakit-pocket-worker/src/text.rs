//! Text preparation, ported from `pocket_tts/models/text_chunking.py`. The
//! model is trained on single sentences, so text is normalized, split on
//! sentence ends, and regrouped into chunks of at most `max_tokens`.

use std::collections::HashMap;

use serde::Deserialize;
use tokenizers::Tokenizer;

/// Per-language text rules from the model's config (`pocket_tts/config`).
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct TextOptions {
    pub pad_with_spaces_for_short_inputs: bool,
    pub remove_semicolons: bool,
    pub append_terminal_punctuation: bool,
    pub capitalize_first_letter: bool,
    /// Characters absent from the training text, rewritten ("" deletes).
    pub replace_characters: HashMap<String, String>,
}

impl Default for TextOptions {
    fn default() -> Self {
        Self {
            pad_with_spaces_for_short_inputs: false,
            remove_semicolons: false,
            append_terminal_punctuation: true,
            capitalize_first_letter: true,
            replace_characters: HashMap::new(),
        }
    }
}

const TERMINAL: &[char] = &['.', '!', '?', '\u{2026}'];
const WEAK: &[char] = &[',', ';', ':', '-', '\u{2013}', '\u{2014}'];
const CLOSERS: &[char] = &['"', '\'', '\u{201d}', '\u{2019}', ')', ']', '\u{00bb}'];

/// Normalized prompt and the reference's guess of frames to keep after EOS
/// (before its `+ 2`). `None` for text with nothing to say.
pub fn prepare(text: &str, o: &TextOptions) -> Option<(String, usize)> {
    let mut text = text.trim().to_string();
    if !o.replace_characters.is_empty() {
        let replaced: String = text
            .chars()
            .map(|c| o.replace_characters.get(c.encode_utf8(&mut [0; 4]) as &str).cloned().unwrap_or_else(|| c.to_string()))
            .collect();
        text = drop_marks_after_sentence_end(&replaced.split_whitespace().collect::<Vec<_>>().join(" "));
    }
    if text.is_empty() {
        return None;
    }
    text = text.replace('\n', " ").replace('\r', " ").replace("  ", " ");
    if o.remove_semicolons {
        text = text.replace(';', ",");
    }
    let frames_after_eos = if text.split_whitespace().count() <= 4 { 3 } else { 1 };

    if o.capitalize_first_letter {
        let mut chars = text.chars();
        if let Some(first) = chars.next() {
            if !first.is_uppercase() {
                text = first.to_uppercase().chain(chars).collect();
            }
        }
    }
    if o.append_terminal_punctuation {
        text = ensure_terminal_punctuation(&text);
    }
    if o.pad_with_spaces_for_short_inputs && text.split_whitespace().count() < 5 {
        text = format!("{}{text}", " ".repeat(8));
    }
    Some((text, frames_after_eos))
}

/// `re.sub(r"([.!?…])\s*[,;:]", r"\1", text)`: deleted quotes leave
/// '"Hi?", she said' as 'Hi?, she said'; keep the sentence mark only.
fn drop_marks_after_sentence_end(text: &str) -> String {
    let chars: Vec<char> = text.chars().collect();
    let mut out = String::with_capacity(text.len());
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        out.push(c);
        i += 1;
        if TERMINAL.contains(&c) {
            let mut j = i;
            while j < chars.len() && chars[j].is_whitespace() {
                j += 1;
            }
            if j < chars.len() && [',', ';', ':'].contains(&chars[j]) {
                i = j + 1;
            }
        }
    }
    out
}

fn ensure_terminal_punctuation(text: &str) -> String {
    let core = text.trim_end_matches(|c: char| CLOSERS.contains(&c) || c == ' ');
    let closers = text[core.len()..].trim();
    match core.chars().last() {
        None => text.to_string(),
        Some(c) if TERMINAL.contains(&c) => text.to_string(),
        Some(c) if WEAK.contains(&c) => {
            format!("{}.{closers}", core.trim_end_matches(|c: char| WEAK.contains(&c) || c == ' '))
        }
        Some(_) => format!("{text}."),
    }
}

pub struct Text {
    tokenizer: Tokenizer,
    sentence_ends: Vec<u32>,
    clause_ends: Vec<u32>,
}

impl Text {
    pub fn new(tokenizer: Tokenizer) -> Result<Self, String> {
        let mut t = Self { tokenizer, sentence_ends: Vec::new(), clause_ends: Vec::new() };
        // As in the reference: the marks' own tokens, minus the leading one.
        t.sentence_ends = t.encode(".!...?")?.into_iter().skip(1).collect();
        t.clause_ends = t.encode(",;:")?.into_iter().skip(1).collect();
        Ok(t)
    }

    pub fn encode(&self, text: &str) -> Result<Vec<u32>, String> {
        Ok(self.tokenizer.encode(text, true).map_err(|e| e.to_string())?.get_ids().to_vec())
    }

    fn decode(&self, ids: &[u32]) -> String {
        self.tokenizer.decode(ids, true).unwrap_or_default()
    }

    fn boundaries(&self, tokens: &[u32], marks: &[u32], skip_decimal_periods: bool) -> Vec<usize> {
        let mut out = vec![0];
        let mut previous_was_boundary = false;
        for (i, t) in tokens.iter().enumerate() {
            if marks.contains(t) {
                previous_was_boundary = true;
            } else {
                if previous_was_boundary && !(skip_decimal_periods && self.is_decimal_period(tokens, i)) {
                    out.push(i);
                }
                previous_was_boundary = false;
            }
        }
        out.push(tokens.len());
        out
    }

    /// True when `i` begins right after the period of a number like 3.14.
    fn is_decimal_period(&self, tokens: &[u32], i: usize) -> bool {
        let prefix: Vec<char> = self.decode(&tokens[..i]).chars().collect();
        let suffix = self.decode(&tokens[i..]);
        prefix.len() >= 2
            && prefix[prefix.len() - 1] == '.'
            && prefix[prefix.len() - 2].is_numeric()
            && suffix.chars().next().is_some_and(char::is_numeric)
    }

    fn segments(&self, tokens: &[u32], bounds: &[usize]) -> Vec<(usize, String)> {
        bounds.windows(2).map(|w| (w[1] - w[0], self.decode(&tokens[w[0]..w[1]]))).collect()
    }

    /// Splits text into chunks of whole sentences of at most `max_tokens`.
    pub fn chunks(&self, text: &str, o: &TextOptions, max_tokens: usize) -> Result<Vec<String>, String> {
        let Some((text, _)) = prepare(text, o) else { return Ok(Vec::new()) };
        let tokens = self.encode(text.trim())?;
        let sentences = self.segments(&tokens, &self.boundaries(&tokens, &self.sentence_ends, true));

        // Oversized sentences are split on commas, semicolons, and colons.
        let mut refined = Vec::new();
        for (n, sentence) in sentences {
            if n <= max_tokens {
                refined.push((n, sentence));
                continue;
            }
            let sub = self.encode(sentence.trim())?;
            let parts = self.segments(&sub, &self.boundaries(&sub, &self.clause_ends, false));
            if parts.len() > 1 {
                refined.extend(parts);
            } else {
                refined.push((n, sentence));
            }
        }

        let mut chunks = Vec::new();
        let mut current = String::new();
        let mut count = 0;
        for (n, sentence) in refined {
            if current.is_empty() {
                current = sentence;
                count = n;
            } else if count + n > max_tokens {
                chunks.push(current.trim().to_string());
                current = sentence;
                count = n;
            } else {
                current.push(' ');
                current.push_str(&sentence);
                count += n;
            }
        }
        if !current.is_empty() {
            chunks.push(current.trim().to_string());
        }
        Ok(chunks)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn french() -> TextOptions {
        let map = [("\u{201c}", ""), ("\u{201d}", ""), ("\"", ""), ("\u{2019}", "'"), (":", ","), ("(", ""), (")", "")];
        TextOptions {
            remove_semicolons: true,
            replace_characters: map.iter().map(|(a, b)| (a.to_string(), b.to_string())).collect(),
            ..TextOptions::default()
        }
    }

    #[test]
    fn capitalizes_and_ends_sentences() {
        let o = TextOptions::default();
        assert_eq!(prepare("hello world", &o), Some(("Hello world.".into(), 3)));
        assert_eq!(prepare("  it works, doesn't it?  ", &o).unwrap().0, "It works, doesn't it?");
        assert_eq!(prepare("a list, of things, and more words here,", &o), Some(("A list, of things, and more words here.".into(), 1)));
        assert_eq!(prepare("She said \"go\"", &o).unwrap().0, "She said \"go\".");
        assert_eq!(prepare("   ", &o), None);
    }

    #[test]
    fn applies_language_rules() {
        let o = french();
        assert_eq!(prepare("\u{201c}Salut ?\u{201d}, dit-il; puis: rien", &o).unwrap().0, "Salut ? dit-il, puis, rien.");
        assert_eq!(prepare("l\u{2019}homme (grand)", &o).unwrap().0, "L'homme grand.");
    }

    #[test]
    fn pads_short_inputs_when_asked() {
        let o = TextOptions { pad_with_spaces_for_short_inputs: true, ..TextOptions::default() };
        assert_eq!(prepare("Yes", &o).unwrap().0, "        Yes.");
    }
}
