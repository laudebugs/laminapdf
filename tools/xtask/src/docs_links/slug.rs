//! GitHub's heading anchors, as computed by `github-slugger` (which github.com uses).
//!
//! A slug is the heading's text, lowercased, with every character that is not a letter, a
//! number, a mark (Unicode general categories L, N, M), a connector punctuation such as `_`
//! (Pc), `-` or a space removed, and each space replaced by `-`. Runs of spaces
//! are not collapsed: `Round A — Foundations` becomes `round-a--foundations`. Repeated headings
//! get `-1`, `-2`, … in document order.

use std::collections::BTreeMap;

use unicode_properties::{GeneralCategory, GeneralCategoryGroup, UnicodeGeneralCategory};

/// The slug of one heading's plain text (no duplicate suffix).
pub(crate) fn slug(text: &str) -> String {
    text.chars()
        .flat_map(char::to_lowercase)
        .filter_map(|c| match c {
            ' ' => Some('-'),
            '-' => Some(c),
            c if is_kept(c) => Some(c),
            _ => None,
        })
        .collect()
}

/// Letters, marks, numbers and connector punctuation: what github-slugger keeps.
fn is_kept(c: char) -> bool {
    matches!(
        c.general_category_group(),
        GeneralCategoryGroup::Letter | GeneralCategoryGroup::Mark | GeneralCategoryGroup::Number
    ) || c.general_category() == GeneralCategory::ConnectorPunctuation
}

/// Hands out unique slugs for the headings of one document, like `github-slugger`'s `Slugger`.
#[derive(Debug, Default)]
pub(crate) struct Slugger {
    /// Slug → how many times its base has been repeated so far.
    occurrences: BTreeMap<String, u32>,
}

impl Slugger {
    /// The anchor of the next heading with this text.
    pub(crate) fn next(&mut self, text: &str) -> String {
        let base = slug(text);
        let mut candidate = base.clone();
        while self.occurrences.contains_key(&candidate) {
            let count = self.occurrences.entry(base.clone()).or_default();
            *count += 1;
            candidate = format!("{base}-{count}");
        }
        self.occurrences.insert(candidate.clone(), 0);
        candidate
    }
}

#[cfg(test)]
mod tests {
    use super::{Slugger, slug};

    #[test]
    fn section_sign_and_dots_are_dropped() {
        assert_eq!(slug("7. Encryption (§7.6)"), "7-encryption-76");
    }

    #[test]
    fn em_dash_leaves_a_double_hyphen() {
        assert_eq!(slug("Round A — Foundations"), "round-a--foundations");
    }

    #[test]
    fn at_sign_and_slash_are_dropped() {
        assert_eq!(slug("B6. WASM and @lamina/pdf"), "b6-wasm-and-laminapdf");
    }

    #[test]
    fn emoji_are_dropped() {
        assert_eq!(slug("🚀 Launch plan"), "-launch-plan");
    }

    #[test]
    fn unicode_letters_are_kept_and_lowercased() {
        assert_eq!(slug("Ünïcode Straße"), "ünïcode-straße");
    }

    #[test]
    fn decomposed_combining_marks_are_kept() {
        assert_eq!(slug("Cafe\u{301} menu"), "cafe\u{301}-menu");
    }

    #[test]
    fn devanagari_vowel_signs_and_virama_are_kept() {
        assert_eq!(slug("हिन्दी पाठ"), "हिन्दी-पाठ");
    }

    #[test]
    fn underscores_and_hyphens_are_kept() {
        assert_eq!(slug("as_slice and wasm-bindgen"), "as_slice-and-wasm-bindgen");
    }

    #[test]
    fn other_punctuation_is_dropped() {
        assert_eq!(
            slug("What's `Limits`? A: \"soft\" vs. hard!"),
            "whats-limits-a-soft-vs-hard"
        );
    }

    #[test]
    fn repeated_headings_get_numbered_suffixes() {
        let mut slugger = Slugger::default();
        let slugs: Vec<_> = ["Tests", "Tests", "Tests"].iter().map(|t| slugger.next(t)).collect();
        assert_eq!(slugs, ["tests", "tests-1", "tests-2"]);
    }

    #[test]
    fn suffix_skips_a_slug_taken_by_a_literal_heading() {
        let mut slugger = Slugger::default();
        let slugs: Vec<_> = ["a", "a-1", "a"].iter().map(|t| slugger.next(t)).collect();
        assert_eq!(slugs, ["a", "a-1", "a-2"]);
    }
}
