//! Line-break opportunities ([UAX #14]) and grapheme clusters ([UAX #29]) from ICU4X's
//! `icu_segmenter` with compiled data (research 03 §7.4).
//!
//! The segmenter follows the Unicode 17 rules (ICU4X's `new_17_for_non_complex_scripts`; the
//! full `LineBreakTest-17.0.0.txt` passes but for 10 lines of dictionary scripts) and is built for scripts
//! that need no dictionary: Thai, Lao, Khmer and Burmese (class `SA`, no spaces between words)
//! get only the opportunities their punctuation and spaces give, which keeps their dictionary
//! and LSTM models out of the binary.
//!
//! [UAX #14]: https://www.unicode.org/reports/tr14/
//! [UAX #29]: https://www.unicode.org/reports/tr29/

use icu_properties::CodePointMapData;
use icu_properties::props::LineBreak;
use icu_segmenter::options::LineBreakOptions;
use icu_segmenter::{GraphemeClusterSegmenter, LineSegmenter};

/// A position in a text where a line may (or must) end.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub struct BreakOpportunity {
    /// The byte offset: the line ends just before it.
    pub offset: usize,
    /// A mandatory break: after a hard line break (`BK`, `CR`, `LF`, `NL`: rules LB4–LB5) or at
    /// the end of the text (LB3).
    pub mandatory: bool,
}

/// The break opportunities of `text`, in order: every position where a line may end, the end
/// of the text included (never the start). Rules LB1–LB31 of Unicode 17 with the default
/// (strict, normal word-break) options; no dictionary breaks inside Thai, Lao, Khmer or Burmese.
///
/// ```
/// use laminapdf_text::line_breaks;
///
/// let offsets: Vec<usize> = line_breaks("Hello, world!").iter().map(|b| b.offset).collect();
/// assert_eq!(offsets, [7, 13]); // after "Hello, " and at the end
/// ```
#[must_use]
pub fn line_breaks(text: &str) -> Vec<BreakOpportunity> {
    LineSegmenter::new_17_for_non_complex_scripts(LineBreakOptions::default())
        .segment_str(text)
        .filter(|&offset| offset > 0)
        .map(|offset| {
            let before = text.get(..offset).and_then(|t| t.chars().next_back());
            let hard = before.is_some_and(is_hard_break);
            BreakOpportunity {
                offset,
                mandatory: hard || offset == text.len(),
            }
        })
        .collect()
}

/// The grapheme-cluster boundaries of `text` (extended grapheme clusters, UAX #29): byte
/// offsets from 0 to `text.len()` inclusive (only 0 for the empty text). User-perceived
/// characters: a base and its combining marks, an emoji ZWJ sequence, a Hangul syllable.
#[must_use]
pub fn grapheme_boundaries(text: &str) -> Vec<usize> {
    GraphemeClusterSegmenter::new().segment_str(text).collect()
}

/// Whether `ch` is a hard line break (classes `BK`, `CR`, `LF`, `NL`; rules LB4–LB5).
///
/// ```
/// assert!(laminapdf_text::is_hard_break('\n') && !laminapdf_text::is_hard_break(' '));
/// ```
#[must_use]
pub fn is_hard_break(ch: char) -> bool {
    matches!(
        CodePointMapData::<LineBreak>::new().get(ch),
        LineBreak::MandatoryBreak | LineBreak::CarriageReturn | LineBreak::LineFeed | LineBreak::NextLine
    )
}

/// Whether `ch` hangs at the end of a line instead of taking space: spaces and the like
/// (classes `SP`, `ZW`, and the hard breaks), which UAX #14 lets a line end with and which
/// alignment ignores. Shared with `laminapdf-build`'s unshaped (standard 14) layout.
///
/// ```
/// assert!(laminapdf_text::is_trailing_space(' ') && !laminapdf_text::is_trailing_space('a'));
/// ```
#[must_use]
pub fn is_trailing_space(ch: char) -> bool {
    matches!(
        CodePointMapData::<LineBreak>::new().get(ch),
        LineBreak::Space | LineBreak::ZWSpace
    ) || is_hard_break(ch)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hard_breaks_are_mandatory() {
        let breaks = line_breaks("a b\nc");
        assert_eq!(
            breaks,
            [
                BreakOpportunity {
                    offset: 2,
                    mandatory: false
                },
                BreakOpportunity {
                    offset: 4,
                    mandatory: true
                },
                BreakOpportunity {
                    offset: 5,
                    mandatory: true
                },
            ]
        );
    }

    #[test]
    fn no_break_before_closing_punctuation_or_inside_numbers() {
        let offsets: Vec<usize> = line_breaks("(1,000.00) !").iter().map(|b| b.offset).collect();
        assert_eq!(offsets, [12]);
    }

    #[test]
    fn han_ideographs_break_between_each_other() {
        let offsets: Vec<usize> = line_breaks("你好世界").iter().map(|b| b.offset).collect();
        assert_eq!(offsets, [3, 6, 9, 12]);
    }

    #[test]
    fn graphemes_keep_marks_with_their_base() {
        assert_eq!(grapheme_boundaries("e\u{301}x"), [0, 3, 4]);
        assert_eq!(grapheme_boundaries("कि"), [0, 6]);
    }

    #[test]
    fn empty_text_has_no_breaks() {
        assert_eq!(line_breaks(""), []);
    }
}
