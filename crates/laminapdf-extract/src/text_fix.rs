//! Text fixes ([ADR 0030] §5): lines joined into a block's text, words hyphenated across a line
//! break joined, and Latin ligatures decomposed.
//!
//! [ADR 0030]: https://github.com/laudebugs/laminapdf/blob/main/docs/decisions/0030-extraction-output-and-model-plugins.md

/// The soft hyphen, U+00AD.
const SOFT_HYPHEN: char = '\u{AD}';

/// `lines` joined with a space; with `dehyphenate`, a line ending in a hyphen or soft hyphen
/// joins the next without it when that line starts with a lowercase letter (`extrac-` +
/// `tion`), and keeps it otherwise (`well-` + `Known`).
pub(crate) fn join_lines<'a>(lines: impl IntoIterator<Item = &'a str>, dehyphenate: bool) -> String {
    let mut text = String::new();
    for line in lines {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        if text.is_empty() {
            text.push_str(line);
            continue;
        }
        let lower_next = line.chars().next().is_some_and(char::is_lowercase);
        if text.ends_with(SOFT_HYPHEN) {
            text.pop();
            if !lower_next {
                text.push('-');
            }
        } else if dehyphenate && text.ends_with('-') && !text.ends_with(" -") {
            // A hyphen ending a word: dropped before a lowercase continuation, kept (with no
            // space) before anything else.
            if lower_next {
                text.pop();
            }
        } else {
            text.push(' ');
        }
        text.push_str(line);
    }
    text
}

/// `text` with the Latin ligatures U+FB00–FB06 spelled out (when no `/ActualText` gave the
/// text, which already spells them).
pub(crate) fn decompose_ligatures(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for c in text.chars() {
        match c {
            '\u{FB00}' => out.push_str("ff"),
            '\u{FB01}' => out.push_str("fi"),
            '\u{FB02}' => out.push_str("fl"),
            '\u{FB03}' => out.push_str("ffi"),
            '\u{FB04}' => out.push_str("ffl"),
            '\u{FB05}' | '\u{FB06}' => out.push_str("st"),
            other => out.push(other),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::{decompose_ligatures, join_lines};

    #[test]
    fn a_word_hyphenated_before_a_lowercase_line_is_joined() {
        assert_eq!(
            join_lines(["text extrac-", "tion works"], true),
            "text extraction works"
        );
    }

    #[test]
    fn a_hyphen_before_an_uppercase_line_stays() {
        assert_eq!(join_lines(["a well-", "Known name"], true), "a well-Known name");
    }

    #[test]
    fn a_soft_hyphen_is_always_removed() {
        assert_eq!(join_lines(["extrac\u{AD}", "tion"], false), "extraction");
        assert_eq!(join_lines(["Ber\u{AD}", "Lin"], true), "Ber-Lin");
    }

    #[test]
    fn without_dehyphenation_lines_join_with_a_space() {
        assert_eq!(join_lines(["text extrac-", "tion"], false), "text extrac- tion");
        assert_eq!(join_lines(["one", "  ", "two"], true), "one two");
    }

    #[test]
    fn a_dash_standing_alone_is_not_a_hyphen() {
        assert_eq!(join_lines(["fast -", "really"], true), "fast - really");
    }

    #[test]
    fn ligatures_are_spelled_out() {
        assert_eq!(
            decompose_ligatures("\u{FB01}nd the \u{FB03}x \u{FB00}"),
            "find the ffix ff"
        );
    }
}
