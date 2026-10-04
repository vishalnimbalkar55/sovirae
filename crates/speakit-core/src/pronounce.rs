//! User pronunciation rules: "say this word as that" (spec §9, user
//! feature 2026-10-04). Rules change only the text sent to the voice; the
//! document shown on screen keeps the original spelling.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct Rule {
    /// The word or phrase as written. Matched as whole words, ignoring case.
    pub from: String,
    /// What the voice says instead; empty skips the word.
    pub to: String,
}

impl Rule {
    pub fn new(from: impl Into<String>, to: impl Into<String>) -> Self {
        Self { from: from.into(), to: to.into() }
    }
}

/// Drops blank rules and duplicates (first wins), trimming both sides.
pub fn sanitize(rules: Vec<Rule>) -> Vec<Rule> {
    let mut out: Vec<Rule> = Vec::new();
    for r in rules {
        let from = r.from.split_whitespace().collect::<Vec<_>>().join(" ");
        let to = r.to.split_whitespace().collect::<Vec<_>>().join(" ");
        if from.is_empty() || out.iter().any(|o| fold(&o.from) == fold(&from)) {
            continue;
        }
        out.push(Rule { from, to });
    }
    out
}

fn fold(s: &str) -> String {
    s.to_lowercase()
}

fn is_word_char(c: char) -> bool {
    c.is_alphanumeric() || c == '\'' || c == '\u{2019}'
}

/// Replaces every whole-word, case-insensitive match of a rule's `from` with
/// its `to`. Longer phrases win over shorter ones at the same position.
pub fn apply(text: &str, rules: &[Rule]) -> String {
    if rules.is_empty() {
        return text.to_string();
    }
    let chars: Vec<char> = text.chars().collect();
    let folded: Vec<Vec<char>> = chars.iter().map(|c| c.to_lowercase().collect()).collect();
    let mut needles: Vec<(Vec<char>, &str)> = rules
        .iter()
        .map(|r| (r.from.to_lowercase().chars().collect::<Vec<_>>(), r.to.as_str()))
        .filter(|(n, _)| !n.is_empty())
        .collect();
    needles.sort_by_key(|(n, _)| std::cmp::Reverse(n.len()));

    let mut out = String::with_capacity(text.len());
    let mut i = 0;
    while i < chars.len() {
        let at_word_start = i == 0 || !is_word_char(chars[i - 1]);
        let hit = at_word_start
            .then(|| needles.iter().find(|(n, _)| matches_at(&folded, i, n)))
            .flatten();
        match hit {
            Some((n, to)) => {
                let end = i + consumed(&folded, i, n);
                out.push_str(to);
                i = end;
            }
            None => {
                out.push(chars[i]);
                i += 1;
            }
        }
    }
    out
}

/// Whether `needle` (already lower-cased) starts at `at` and ends on a
/// word boundary.
fn matches_at(folded: &[Vec<char>], at: usize, needle: &[char]) -> bool {
    let mut k = 0;
    let mut i = at;
    while k < needle.len() {
        let Some(f) = folded.get(i) else { return false };
        if !f.iter().eq(needle[k..].iter().take(f.len())) {
            return false;
        }
        k += f.len();
        i += 1;
    }
    k == needle.len() && (i >= folded.len() || !is_word_char_folded(&folded[i]))
}

fn is_word_char_folded(f: &[char]) -> bool {
    f.first().is_some_and(|c| is_word_char(*c))
}

/// Source chars covered by a match of `needle` at `at`.
fn consumed(folded: &[Vec<char>], at: usize, needle: &[char]) -> usize {
    let mut k = 0;
    let mut n = 0;
    while k < needle.len() {
        k += folded[at + n].len();
        n += 1;
    }
    n
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rules() -> Vec<Rule> {
        vec![Rule::new("Sovirae", "so-vee-ray"), Rule::new("GIF", "jif"), Rule::new("New York", "Noo Yawk"), Rule::new("(sic)", "")]
    }

    #[test]
    fn replaces_whole_words_ignoring_case() {
        assert_eq!(apply("SOVIRAE reads. A gif, not a gift.", &rules()), "so-vee-ray reads. A jif, not a gift.");
        assert_eq!(apply("Gifs are fine.", &rules()), "Gifs are fine.", "no partial match");
        assert_eq!(apply("sovirae's player", &rules()), "sovirae's player", "apostrophe keeps the word whole");
    }

    #[test]
    fn phrases_and_deletions() {
        assert_eq!(apply("I love New York (sic) a lot.", &rules()), "I love Noo Yawk  a lot.");
        assert_eq!(apply("Newer York", &rules()), "Newer York");
    }

    #[test]
    fn longest_match_wins_and_unicode_case_folds() {
        let r = vec![Rule::new("Straße", "Strasse"), Rule::new("Straßenbahn", "tram"), Rule::new("İstanbul", "Istanbul")];
        assert_eq!(apply("STRASSE? Straßenbahn!", &r), "STRASSE? tram!");
        assert_eq!(apply("die STRAßE", &r), "die Strasse");
    }

    #[test]
    fn empty_rules_pass_text_through() {
        assert_eq!(apply("Hello", &[]), "Hello");
        assert_eq!(apply("", &rules()), "");
    }

    #[test]
    fn sanitizes_rules() {
        let out = sanitize(vec![
            Rule::new("  Sovirae  ", " so  vee "),
            Rule::new("", "x"),
            Rule::new("SOVIRAE", "other"),
            Rule::new("ok", ""),
        ]);
        assert_eq!(out, vec![Rule::new("Sovirae", "so vee"), Rule::new("ok", "")]);
    }
}
