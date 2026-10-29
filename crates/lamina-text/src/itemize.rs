//! Itemisation (todo B4b; research 03 §7 steps 3–4): split a paragraph into runs that can each
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
/// Characters no font needs a glyph for (default-ignorable code points such as ZWJ and the
/// bidi controls, and hard line breaks) never decide the font.
///
/// # Errors
/// [`Error::MissingGlyph`] for the first character of a cluster no font covers (naming the
/// primary font, or no font when the list is empty); [`Error::InvalidFont`] when a font's
/// tables can't be read.
pub fn itemize(para: &BidiParagraph<'_>, fonts: &[Font]) -> Result<Vec<Item>> {
    let text = para.text();
    let charmaps = fonts
        .iter()
        .map(|f| f.font_ref().map(|r| r.charmap()))
        .collect::<Result<Vec<_>>>()?;
    let ignorable = CodePointSetData::new::<DefaultIgnorableCodePoint>();
    let needs_glyph = |ch: char| !ignorable.contains(ch) && !is_hard_break(ch);
    let covers = |font: usize, cluster: &str| {
        charmaps.get(font).is_some_and(|map| {
            cluster
                .chars()
                .filter(|&ch| needs_glyph(ch))
                .all(|ch| map.map(ch).is_some_and(|g| g.to_u32() != 0))
        })
    };

    let mut items: Vec<Item> = Vec::new();
    // Whether the last item's script was taken from a real character (else it is still open
    // to the first real script that follows).
    let mut script_known = false;
    let bounds = grapheme_boundaries(text);
    for pair in bounds.windows(2) {
        let &[start, end] = pair else { continue };
        let cluster = text.get(start..end).unwrap_or_default();
        let script = Script::dominant(cluster);
        let level = para.level_at(start).unwrap_or(0);
        let current = items.last().filter(|item| item.level == level);
        let font = match current {
            Some(item) if covers(item.font, cluster) => item.font,
            _ => (0..fonts.len()).find(|&f| covers(f, cluster)).ok_or_else(|| {
                let ch = cluster
                    .chars()
                    .find(|&ch| needs_glyph(ch) && !covers(0, &ch.to_string()))
                    .or_else(|| cluster.chars().next())
                    .unwrap_or_default();
                Error::MissingGlyph {
                    ch,
                    font: fonts
                        .first()
                        .map(|f| f.info().postscript_name.clone())
                        .unwrap_or_default(),
                }
            })?,
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shape::Direction;

    fn font(name: &str) -> Font {
        let path = format!("{}/../../fixtures/fonts/{name}", env!("CARGO_MANIFEST_DIR"));
        Font::from_bytes(std::fs::read(path).unwrap()).unwrap()
    }

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
