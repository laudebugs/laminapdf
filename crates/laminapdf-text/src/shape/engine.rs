//! [`Shaper`] over HarfRust, HarfBuzz's shaping algorithm ported to Rust (MIT).
//!
//! HarfRust reads the same `read-fonts` tables `skrifa` does (one copy in the build), so a
//! [`Font`] is shaped without parsing its bytes again. Positions are in font units (no scale
//! is set: HarfBuzz's default is the font's units per em), which is what `hb-shape` prints
//! without `--font-size`, so the goldens in `fixtures/shaping/` compare number for number.

use std::ops::Range;
use std::str::FromStr;

use harfrust::{ShapePlan, ShapePlanKey, ShaperInstance, UnicodeBuffer, Variation};

use super::{Direction, Script, ShapeOptions, ShapedGlyph, ShapedRun, Shaper, sub};
use crate::error::{Error, Result};
use crate::font::{Font, GlyphId};

/// The HarfRust shaper. Stateless: every call reads the font's layout tables afresh.
///
/// ```
/// # fn font_bytes() -> Vec<u8> { include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../fixtures/fonts/NotoSans-Regular.ttf")).to_vec() }
/// use laminapdf_text::{Font, HarfRust, ShapeOptions, Shaper};
///
/// let font = Font::from_bytes(font_bytes())?;
/// let run = HarfRust.shape(&font, "fi", &ShapeOptions::new())?;
/// assert_eq!(run.glyphs.len(), 1); // the `fi` ligature
///
/// let run = HarfRust.shape(&font, "fi", &ShapeOptions::new().with_feature("liga=0".parse()?))?;
/// assert_eq!(run.glyphs.len(), 2);
/// # Ok::<(), laminapdf_text::Error>(())
/// ```
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct HarfRust;

impl Shaper for HarfRust {
    fn shape_range(&self, font: &Font, text: &str, range: Range<usize>, options: &ShapeOptions) -> Result<ShapedRun> {
        let run_text = sub(text, range.clone())?;
        let before = sub(text, 0..range.start)?;
        let after = sub(text, range.end..text.len())?;
        if let Some(standard) = font.standard14() {
            return standard_run(font, standard, run_text, range, options);
        }
        let font_ref = font.font_ref()?;

        let script = options
            .script
            .or_else(|| Script::dominant(run_text))
            .unwrap_or(Script::COMMON);
        let direction = options.direction.unwrap_or_else(|| script.direction());

        let hb_script = harfrust::Script::from_iso15924_tag(harfrust::Tag::new(&script.to_bytes()));
        let hb_direction = match direction {
            Direction::LeftToRight => harfrust::Direction::LeftToRight,
            Direction::RightToLeft => harfrust::Direction::RightToLeft,
        };
        // Any non-empty tag is accepted (unknown ones select the default language system).
        let language = options
            .language
            .as_deref()
            .and_then(|language| harfrust::Language::from_str(language).ok());

        let mut buffer = UnicodeBuffer::new();
        buffer.set_pre_context(before);
        for (at, ch) in run_text.char_indices() {
            let cluster = u32::try_from(at).map_err(|_| Error::InvalidTextRange {
                start: range.start,
                end: range.end,
                len: text.len(),
            })?;
            buffer.add(stand_in(ch), cluster);
        }
        buffer.set_post_context(after);
        if let Some(s) = hb_script {
            buffer.set_script(s);
        }
        buffer.set_direction(hb_direction);
        if let Some(language) = &language {
            buffer.set_language(language.clone());
        }
        let features: Vec<harfrust::Feature> = options
            .features
            .iter()
            .map(|f| harfrust::Feature::new(harfrust::Tag::new(&f.tag().to_bytes()), f.value(), ..))
            .collect();

        let data = font.shaper_data()?;
        let instance = (!font.coordinates().is_empty()).then(|| {
            ShaperInstance::from_variations(
                &font_ref,
                font.coordinates().iter().map(|&(tag, value)| Variation {
                    tag: tag.to_skrifa(),
                    value,
                }),
            )
        });
        let shaper = data.shaper(&font_ref).instance(instance.as_ref()).build();
        // The plan HarfRust would build for this buffer (its direction, script and language
        // as set above), cached on the font's face and found by HarfRust's own key.
        let key = ShapePlanKey::new(hb_script, hb_direction)
            .language(language.as_ref())
            .instance(instance.as_ref())
            .features(&features);
        let plan = font.shape_plan(
            |plan| key.matches(plan),
            || ShapePlan::new(&shaper, hb_direction, hb_script, language.as_ref(), &features),
        );
        let output = shaper.shape(
            buffer,
            harfrust::ShapeOptions::new().plan(Some(&plan)).features(&features),
        );

        let glyphs = output
            .glyph_infos()
            .iter()
            .zip(output.glyph_positions())
            .map(|(info, pos)| {
                let gid = u16::try_from(info.glyph_id).map_err(|_| Error::InvalidFont { index: font.index() })?;
                let cluster = usize::try_from(info.cluster).map_err(|_| Error::InvalidFont { index: font.index() })?;
                Ok(ShapedGlyph {
                    gid: GlyphId::new(gid),
                    cluster: range.start + cluster,
                    x_advance: pos.x_advance,
                    y_advance: pos.y_advance,
                    x_offset: pos.x_offset,
                    y_offset: pos.y_offset,
                })
            })
            .collect::<Result<Vec<_>>>()?;
        Ok(ShapedRun {
            glyphs,
            text_range: range,
            direction,
            script,
            units_per_em: font.info().units_per_em,
            font: font.info().postscript_name.clone(),
        })
    }
}

/// `text` (at `range` of the whole text) laid out in a standard 14 font, which has no font
/// program to shape with: one glyph per character, left to right, its AFM width as its advance
/// (in the AFM's 1000 units per em). Control characters and default-ignorables advance by 0.
///
/// # Errors
/// [`Error::MissingGlyph`] for a character the font lacks.
fn standard_run(
    font: &Font,
    standard: crate::Standard14,
    text: &str,
    range: Range<usize>,
    options: &ShapeOptions,
) -> Result<ShapedRun> {
    let mut glyphs = Vec::with_capacity(text.len());
    for (at, ch) in text.char_indices() {
        let (gid, advance) =
            if ch.is_control() || matches!(ch, '\u{200b}'..='\u{200f}' | '\u{2060}'..='\u{206f}' | '\u{feff}') {
                (standard.glyph_for_char(' ').unwrap_or(GlyphId::NOTDEF), 0)
            } else {
                let gid = standard.glyph_for_char(ch).ok_or_else(|| Error::MissingGlyph {
                    ch,
                    font: font.info().postscript_name.clone(),
                })?;
                (gid, i32::from(standard.glyph_width(gid).unwrap_or(0)))
            };
        glyphs.push(ShapedGlyph {
            gid,
            cluster: range.start + at,
            x_advance: advance,
            y_advance: 0,
            x_offset: 0,
            y_offset: 0,
        });
    }
    Ok(ShapedRun {
        glyphs,
        text_range: range,
        direction: Direction::LeftToRight,
        script: options
            .script
            .or_else(|| Script::dominant(text))
            .unwrap_or(Script::COMMON),
        units_per_em: font.info().units_per_em,
        font: font.info().postscript_name.clone(),
    })
}

/// The character HarfRust shapes for `ch`. Fonts have no glyphs for control characters
/// (`Cc`), so a tab is shaped as a space (no tab stops yet) and every other control (hard
/// line breaks included, when a caller shapes them) as U+200B ZERO WIDTH SPACE, which the
/// shaper hides as a default-ignorable. The cluster keeps pointing at the original character.
fn stand_in(ch: char) -> char {
    match ch {
        '\t' => ' ',
        c if c.is_control() => '\u{200B}',
        c => c,
    }
}
