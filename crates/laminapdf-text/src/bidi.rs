//! Bidirectional text (research 03 §7.3): the Unicode Bidirectional Algorithm,
//! [UAX #9], resolves an embedding level per character (even: left to right, odd: right to
//! left), and after line breaking each line's runs are put in visual order (rule L2). PDF has
//! no bidi concept: content is painted in visual order with absolute positions.
//!
//! The algorithm is `unicode-bidi`'s, because ICU4X ships the data but not the algorithm. The
//! `Bidi_Class` data is ICU4X's (`icu_properties`), so it has the same Unicode version as the
//! line breaker's.
//!
//! [UAX #9]: https://www.unicode.org/reports/tr9/

use std::ops::Range;

use icu_properties::CodePointMapData;
use icu_properties::props::BidiClass;
use unicode_bidi::{BidiClass as Class, Level, ParagraphBidiInfo};

use crate::error::{Error, Result};
use crate::shape::Direction;

/// A run of text at one embedding level.
///
/// Added in 0.2.0.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub struct BidiRun {
    /// The run's bytes in the paragraph's text.
    pub range: Range<usize>,
    /// The embedding level (0–125): even is left to right, odd right to left.
    pub level: u8,
}

impl BidiRun {
    /// The run's direction.
    ///
    /// Added in 0.2.0.
    #[must_use]
    pub const fn direction(&self) -> Direction {
        Direction::from_level(self.level)
    }
}

/// One paragraph's bidi resolution: the levels of its characters, and its lines' runs in
/// visual order.
///
/// The text is treated as one paragraph: a paragraph separator inside it (U+2029, or a line
/// feed) does not start a new paragraph direction (split the text first for that). Explicit
/// embeddings, overrides and isolates (U+202A–202E, U+2066–2069) are honoured.
///
/// ```
/// use laminapdf_text::{BidiParagraph, Direction};
///
/// let text = "abc אבג def";
/// let para = BidiParagraph::new(text, None);
/// assert_eq!(para.base_direction(), Direction::LeftToRight);
/// let runs = para.visual_runs(0..text.len())?;
/// let levels: Vec<u8> = runs.iter().map(|r| r.level).collect();
/// assert_eq!(levels, [0, 1, 0]);
/// # Ok::<(), laminapdf_text::Error>(())
/// ```
///
/// Added in 0.2.0.
#[derive(Debug)]
pub struct BidiParagraph<'t> {
    info: ParagraphBidiInfo<'t>,
}

impl<'t> BidiParagraph<'t> {
    /// Resolves `text`'s levels. `base` sets the paragraph direction; `None` takes it from the
    /// first strong character (rules P2–P3), left to right when there is none.
    ///
    /// Added in 0.2.0.
    #[must_use]
    pub fn new(text: &'t str, base: Option<Direction>) -> Self {
        let level = base.map(|d| match d {
            Direction::LeftToRight => Level::ltr(),
            Direction::RightToLeft => Level::rtl(),
        });
        let data = CodePointMapData::<BidiClass>::new();
        Self {
            info: ParagraphBidiInfo::new_with_data_source(&data, text, level),
        }
    }

    /// The paragraph's text.
    ///
    /// Added in 0.2.0.
    #[must_use]
    pub fn text(&self) -> &'t str {
        self.info.text
    }

    /// The paragraph direction (level 0 or 1).
    ///
    /// Added in 0.2.0.
    #[must_use]
    pub fn base_direction(&self) -> Direction {
        Direction::from_level(self.info.paragraph_level.number())
    }

    /// Whether every character resolved to level 0: nothing to reorder.
    ///
    /// Added in 0.2.0.
    #[must_use]
    pub fn is_pure_ltr(&self) -> bool {
        self.info.is_pure_ltr && self.info.paragraph_level.is_ltr()
    }

    /// The resolved level of the character at byte `at` (before the per-line rule L1);
    /// `None` past the end.
    ///
    /// Added in 0.2.0.
    #[must_use]
    pub fn level_at(&self, at: usize) -> Option<u8> {
        self.info.levels.get(at).map(Level::number)
    }

    /// Maximal runs of one resolved level, in logical order (before rule L1): what text is
    /// itemised and shaped by.
    ///
    /// Added in 0.2.0.
    #[must_use]
    pub fn level_runs(&self) -> Vec<BidiRun> {
        runs_of(&self.info.levels, 0..self.info.levels.len())
    }

    /// The runs of the line `line` (a byte range of the text, usually from line breaking) in
    /// visual order, left to right, after rule L1 (trailing whitespace and separators take
    /// the paragraph level) and L2 (reversal).
    ///
    /// # Errors
    /// [`Error::InvalidTextRange`] when `line` is out of bounds or not on character
    /// boundaries.
    ///
    /// Added in 0.2.0.
    pub fn visual_runs(&self, line: Range<usize>) -> Result<Vec<BidiRun>> {
        let levels = self.l1(line.clone())?;
        let mut runs: Vec<BidiRun> = Vec::new();
        for (i, &level) in levels.iter().enumerate() {
            let at = line.start + i;
            match runs.last_mut() {
                Some(run) if run.level == level => run.range.end = at + 1,
                _ => runs.push(BidiRun {
                    range: at..at + 1,
                    level,
                }),
            }
        }
        reorder(&mut runs);
        Ok(runs)
    }

    /// The levels of `line`'s characters after rule L1, one per character in logical order.
    ///
    /// # Errors
    /// As [`BidiParagraph::visual_runs`].
    ///
    /// Added in 0.2.0.
    pub fn line_levels(&self, line: Range<usize>) -> Result<Vec<u8>> {
        let levels = self.l1(line.clone())?;
        let text = self.info.text.get(line).unwrap_or_default();
        Ok(text
            .char_indices()
            .filter_map(|(i, _)| levels.get(i).copied())
            .collect())
    }

    /// The levels of `line`'s bytes after rule L1: segment and paragraph separators, and any
    /// whitespace or isolate controls before them or at the end of the line, take the
    /// paragraph level (characters removed by X9 next to them too). Only the line's slice is
    /// copied.
    fn l1(&self, line: Range<usize>) -> Result<Vec<u8>> {
        let invalid = || Error::InvalidTextRange {
            start: line.start,
            end: line.end,
            len: self.info.text.len(),
        };
        let text = self.info.text.get(line.clone()).ok_or_else(invalid)?;
        let levels = self.info.levels.get(line.clone()).ok_or_else(invalid)?;
        let classes = self.info.original_classes.get(line.clone()).ok_or_else(invalid)?;
        let mut out: Vec<u8> = levels.iter().map(Level::number).collect();
        let paragraph = self.info.paragraph_level.number();
        let mut trailing = true;
        for (i, ch) in text.char_indices().rev() {
            let reset = match classes.get(i).copied() {
                Some(Class::B | Class::S) => {
                    trailing = true;
                    true
                }
                Some(
                    Class::WS
                    | Class::FSI
                    | Class::LRI
                    | Class::RLI
                    | Class::PDI
                    | Class::BN
                    | Class::LRE
                    | Class::RLE
                    | Class::LRO
                    | Class::RLO
                    | Class::PDF,
                ) => trailing,
                _ => {
                    trailing = false;
                    false
                }
            };
            if reset {
                for level in out.iter_mut().skip(i).take(ch.len_utf8()) {
                    *level = paragraph;
                }
            }
        }
        Ok(out)
    }
}

/// Rule L2: from the highest level down to the lowest odd one, reverse every maximal
/// sequence of runs at that level or higher.
fn reorder(runs: &mut [BidiRun]) {
    let Some(max) = runs.iter().map(|r| r.level).max() else {
        return;
    };
    let Some(min_odd) = runs.iter().map(|r| r.level).filter(|l| !l.is_multiple_of(2)).min() else {
        return;
    };
    for level in (min_odd..=max).rev() {
        let mut i = 0;
        while i < runs.len() {
            if runs.get(i).is_some_and(|r| r.level >= level) {
                let start = i;
                while runs.get(i).is_some_and(|r| r.level >= level) {
                    i += 1;
                }
                if let Some(seq) = runs.get_mut(start..i) {
                    seq.reverse();
                }
            } else {
                i += 1;
            }
        }
    }
}

/// Maximal runs of equal levels in `levels[range]`.
fn runs_of(levels: &[Level], range: Range<usize>) -> Vec<BidiRun> {
    let mut runs: Vec<BidiRun> = Vec::new();
    for (i, level) in levels.iter().enumerate().skip(range.start).take(range.len()) {
        let level = level.number();
        match runs.last_mut() {
            Some(run) if run.level == level => run.range.end = i + 1,
            _ => runs.push(BidiRun { range: i..i + 1, level }),
        }
    }
    runs
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rtl_paragraph_is_detected_from_the_first_strong_character() {
        let para = BidiParagraph::new("123 שלום abc", None);
        assert_eq!(para.base_direction(), Direction::RightToLeft);
    }

    #[test]
    fn base_direction_can_be_forced() {
        let para = BidiParagraph::new("שלום", Some(Direction::LeftToRight));
        assert_eq!(para.base_direction(), Direction::LeftToRight);
    }

    #[test]
    fn level_runs_follow_logical_order() {
        let text = "ab אב cd";
        let runs = BidiParagraph::new(text, None).level_runs();
        let got: Vec<(&str, u8)> = runs.iter().map(|r| (&text[r.range.clone()], r.level)).collect();
        assert_eq!(got, [("ab ", 0), ("אב", 1), (" cd", 0)]);
    }

    #[test]
    fn visual_runs_reverse_rtl_lines() {
        let text = "שלום abc עולם";
        let para = BidiParagraph::new(text, None);
        let runs = para.visual_runs(0..text.len()).unwrap();
        let got: Vec<&str> = runs.iter().map(|r| &text[r.range.clone()]).collect();
        assert_eq!(got, [" עולם", "abc", "שלום "]);
    }

    #[test]
    fn visual_runs_reject_ranges_off_character_boundaries() {
        let para = BidiParagraph::new("שלום", None);
        assert_eq!(para.visual_runs(0..1).map_err(|e| e.code()), Err("invalid-text-range"));
    }
}
