//! Itemisation (research 03 §7 steps 3–4): split a paragraph into runs that can each
//! be shaped in one go: one script (UAX #24), one bidi level (UAX #9) and one font.
//!
//! Fonts are chosen per grapheme cluster (so combining marks stay with their base's font) from
//! the caller's list: the current run's font when it covers the cluster, else the first in the
//! list that does. There is no system-font fallback (never in core); a cluster no font covers
//! is [`Error::MissingGlyph`].

use std::ops::Range;

use icu_properties::CodePointSetData;
use icu_properties::props::DefaultIgnorableCodePoint;
use skrifa::MetadataProvider;
use skrifa::charmap::Charmap;

use crate::bidi::BidiParagraph;
use crate::error::{Error, Result};
use crate::font::Font;
use crate::linebreak::{grapheme_boundaries, is_hard_break};
use crate::shape::Script;

/// A run of text to shape with one font, script and direction.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub struct Item {
    /// The run's bytes in the paragraph's text.
    pub range: Range<usize>,
    /// The script: the first character of a real script in the run; [`Script::COMMON`] when
    /// it has none (digits and punctuation only). Common and inherited characters (spaces,
    /// punctuation, combining marks) join the run before them, or the one after them at the
    /// start of a paragraph.
    pub script: Script,
    /// The bidi embedding level (odd: right to left).
    pub level: u8,
    /// The font: an index into the list given to [`itemize`].
    pub font: usize,
}

/// Splits `para`'s text into [`Item`]s in logical order, covering the whole text.
///
/// `fonts` is the caller's list in order of preference; the first is the primary font.
/// A cluster of a real script takes the first font in the list that covers it (a fallback
/// never sticks); spaces, digits and punctuation stay with the run they are in when its font
/// covers them. Characters no font needs a glyph for (default-ignorable code points such as
/// ZWJ and the bidi controls, control characters such as a tab, hard line breaks) never decide
/// the font, so text made only of them itemises even with no fonts (its items name font 0).
///
/// # Errors
/// [`Error::MissingGlyph`] for the first character of a cluster no font covers (naming the
/// primary font, or no font when the list is empty); [`Error::InvalidFont`] when a font's
/// tables can't be read.
pub fn itemize(para: &BidiParagraph<'_>, fonts: &[Font]) -> Result<Vec<Item>> {
    let charmaps = charmaps(fonts)?;
    let order: Vec<usize> = (0..fonts.len()).collect();
    itemize_range(para, fonts, &charmaps, &order, 0..para.text().len())
}

/// Which characters a font has: its character map, or a standard 14 font's own table.
pub(crate) enum Coverage<'a> {
    Map(Charmap<'a>),
    Standard(crate::Standard14),
}

impl Coverage<'_> {
    /// Whether the font has a glyph for `ch` other than `.notdef`.
    fn covers(&self, ch: char) -> bool {
        match self {
            Self::Map(map) => map.map(ch).is_some_and(|g| g.to_u32() != 0),
            Self::Standard(font) => font.glyph_for_char(ch).is_some(),
        }
    }
}

/// Each font's coverage, in the list's order.
///
/// # Errors
/// [`Error::InvalidFont`] when a font's tables can't be read.
pub(crate) fn charmaps(fonts: &[Font]) -> Result<Vec<Coverage<'_>>> {
    fonts
        .iter()
        .map(|f| match f.standard14() {
            Some(standard) => Ok(Coverage::Standard(standard)),
            None => f.font_ref().map(|r| Coverage::Map(r.charmap())),
        })
        .collect()
}

/// [`itemize`] over `range` of `para`'s text with the fonts `order` picks from `fonts`, in
/// that order of preference (a styled span's fonts): items name their font by its index in
/// `fonts`, and none crosses `range`'s ends. Text with nothing to draw names `order`'s first
/// font (0 when `order` is empty). `charmaps` are [`charmaps`]`(fonts)`, read once per layout.
pub(crate) fn itemize_range(
    para: &BidiParagraph<'_>,
    fonts: &[Font],
    charmaps: &[Coverage<'_>],
    order: &[usize],
    range: Range<usize>,
) -> Result<Vec<Item>> {
    let text = para.text();
    let segment = text.get(range.clone()).ok_or(Error::InvalidTextRange {
        start: range.start,
        end: range.end,
        len: text.len(),
    })?;
    let covers = |font: usize, cluster: &str| {
        charmaps
            .get(font)
            .is_some_and(|map| cluster.chars().filter(|&ch| needs_glyph(ch)).all(|ch| map.covers(ch)))
    };
    let primary = order.first().copied().unwrap_or(0);

    let mut items: Vec<Item> = Vec::new();
    // Whether the last item's script was taken from a real character (else it is still open
    // to the first real script that follows).
    let mut script_known = false;
    let bounds = grapheme_boundaries(segment);
    for pair in bounds.windows(2) {
        let &[start, end] = pair else { continue };
        let (start, end) = (range.start + start, range.start + end);
        let cluster = text.get(start..end).unwrap_or_default();
        let script = Script::dominant(cluster);
        let level = para.level_at(start).unwrap_or(0);
        let current = items.last().filter(|item| item.level == level).map(|item| item.font);
        let font = if !cluster.chars().any(needs_glyph) {
            // Nothing to draw (a hard break, a ZWJ): it decides nothing.
            current.unwrap_or(primary)
        } else if let Some(f) = current.filter(|&f| script.is_none() && covers(f, cluster)) {
            // Spaces, digits and punctuation stay in the run they are in.
            f
        } else {
            // Every other cluster takes the first font that covers it, so a fallback font
            // never sticks to text the primary font has.
            order
                .iter()
                .copied()
                .find(|&f| covers(f, cluster))
                .ok_or_else(|| missing_glyph(fonts.get(primary), cluster, &|c| covers(primary, c)))?
        };
        let joins = items.last().is_some_and(|item| {
            item.level == level
                && item.font == font
                && match script {
                    None => true,
                    Some(s) => !script_known || item.script == s,
                }
        });
        if let (true, Some(item)) = (joins, items.last_mut()) {
            item.range.end = end;
            if let (Some(s), false) = (script, script_known) {
                item.script = s;
                script_known = true;
            }
        } else {
            // A level or font change, or a new script, starts an item; common characters at
            // the start of one leave its script open to the first real script that follows.
            items.push(Item {
                range: start..end,
                script: script.unwrap_or(Script::COMMON),
                level,
                font,
            });
            script_known = script.is_some();
        }
    }
    Ok(items)
}

/// Whether `ch` needs a glyph of its own: not a default-ignorable code point (ZWJ, the bidi
/// controls, variation selectors), not a control character (a tab is drawn as a space, other
/// controls as nothing; see the shaper) and not a hard line break.
pub(crate) fn needs_glyph(ch: char) -> bool {
    // No ASCII character is default-ignorable: skip the set lookup for them.
    !ch.is_control()
        && !is_hard_break(ch)
        && (ch.is_ascii() || !CodePointSetData::new::<DefaultIgnorableCodePoint>().contains(ch))
}

/// [`Error::MissingGlyph`] for the first character of `cluster` the primary font lacks.
fn missing_glyph(primary: Option<&Font>, cluster: &str, covers: &impl Fn(&str) -> bool) -> Error {
    let mut buf = [0u8; 4];
    let ch = cluster
        .chars()
        .find(|&ch| needs_glyph(ch) && !covers(ch.encode_utf8(&mut buf)))
        .or_else(|| cluster.chars().next())
        .unwrap_or_default();
    Error::MissingGlyph {
        ch,
        font: primary.map(|f| f.info().postscript_name.clone()).unwrap_or_default(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shape::Direction;
    use crate::test_fonts::font;

    fn summary<'t>(text: &'t str, items: &[Item]) -> Vec<(&'t str, Script, u8, usize)> {
        items
            .iter()
            .map(|i| (&text[i.range.clone()], i.script, i.level, i.font))
            .collect()
    }

    #[test]
    fn splits_by_level_and_font_keeping_spaces_with_the_run_before() {
        let fonts = [font("NotoSans-Regular.ttf"), font("NotoSansHebrew-Regular.ttf")];
        let text = "Hi שלום!";
        let para = BidiParagraph::new(text, Some(Direction::LeftToRight));
        let items = itemize(&para, &fonts).unwrap();
        assert_eq!(
            summary(text, &items),
            [
                ("Hi ", Script::LATIN, 0, 0),
                ("שלום", Script::HEBREW, 1, 1),
                ("!", Script::COMMON, 0, 0),
            ]
        );
    }

    #[test]
    fn leading_common_characters_take_the_following_script() {
        let fonts = [font("NotoSansDevanagari-Regular.ttf")];
        let text = "1. नमस्ते";
        let para = BidiParagraph::new(text, None);
        let items = itemize(&para, &fonts).unwrap();
        assert_eq!(summary(text, &items), [("1. नमस्ते", Script::DEVANAGARI, 0, 0)]);
    }

    #[test]
    fn no_ascii_character_is_default_ignorable() {
        let set = CodePointSetData::new::<DefaultIgnorableCodePoint>();
        assert!((0..=0x7f_u8).map(char::from).all(|ch| !set.contains(ch)));
    }

    #[test]
    fn uncovered_clusters_are_missing_glyphs() {
        let fonts = [font("NotoSans-Regular.ttf")];
        let para = BidiParagraph::new("a中", None);
        let err = itemize(&para, &fonts).unwrap_err();
        assert_eq!(
            err,
            Error::MissingGlyph {
                ch: '中',
                font: "NotoSans-Regular".to_owned()
            }
        );
    }
}
