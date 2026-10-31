//! Text on a surface (ISO 32000-2 §9.2–9.4, §14.9.4; research 03 §2, §5.3, §11.2): fonts
//! registered with a builder, text laid out at call time (shaped, bidi-reordered and broken
//! into lines by `lamina-text`, or measured with the AFM widths for a standard 14 font) and
//! written as `BT … Tf … Tm … TJ … ET` when the page is finished.

use std::ops::Range;
use std::slice;

use lamina_model::{PointPt, Pt, RectPt};
use lamina_text::{
    Alignment, Direction, Feature, FontInfo, FontKey, HarfRust, LayoutOptions, ShapedRun, break_lines,
    grapheme_boundaries, line_breaks,
};

use super::Surface;
use super::color::Color;
use super::path::Matrix;
use crate::annot::link::LinkSpec;
use crate::error::Result;
use crate::nav::LinkTarget;
use crate::tag::Tag;

/// A font registered with one builder ([`Builder::font`](crate::Builder::font),
/// [`Builder::standard_font`](crate::Builder::standard_font) or the [`Appender`](crate::Appender)'s):
/// what [`Surface::text`] and [`Surface::text_box`] draw with. Cheap to clone. A font from
/// another builder fails with `unknown-font` when its page is finished.
#[derive(Debug, Clone)]
pub struct Font {
    pub(crate) key: FontKey,
    pub(crate) font: lamina_text::Font,
}

impl Font {
    /// The font's names and metrics.
    #[must_use]
    pub fn info(&self) -> &FontInfo {
        self.font.info()
    }

    /// The `lamina-text` font it was registered from.
    #[must_use]
    pub const fn font(&self) -> &lamina_text::Font {
        &self.font
    }

    /// Whether this is one of the standard 14 fonts: drawn unshaped (one glyph per character,
    /// left to right, AFM widths without kerning) and not embedded.
    #[must_use]
    pub fn is_standard(&self) -> bool {
        self.font.standard14().is_some()
    }

    /// A length in font units at `size`, in points.
    fn scaled(&self, units: f32, size: f64) -> f64 {
        f64::from(units) * size / f64::from(self.info().units_per_em.max(1))
    }

    /// How far the font rises above the baseline at `size` (points, positive).
    pub(crate) fn ascent(&self, size: f64) -> f64 {
        self.scaled(self.info().ascent, size)
    }

    /// How far it reaches below the baseline at `size` (points, positive).
    pub(crate) fn descent(&self, size: f64) -> f64 {
        -self.scaled(self.info().descent, size)
    }
}

/// How glyphs are painted (`Tr`, §9.3.6). The clipping modes are left out: they would leave
/// a clip behind the text object that the surface can't track.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[non_exhaustive]
pub enum TextRenderMode {
    /// Filled with the fill colour (0), the default.
    #[default]
    Fill,
    /// Outlined with the stroke colour and the current line width (1).
    Stroke,
    /// Filled, then outlined (2).
    FillStroke,
    /// Neither (3): invisible but selectable and extractable, as OCR text layers are.
    Invisible,
}

/// What to lay out.
#[derive(Debug, Clone)]
pub(crate) struct Spec {
    pub(crate) text: String,
    pub(crate) font: Font,
    /// Font size, points.
    pub(crate) size: f64,
    pub(crate) align: Alignment,
    pub(crate) direction: Option<Direction>,
    pub(crate) language: Option<String>,
    pub(crate) features: Vec<Feature>,
    /// The box width; `None`: lines end only at hard breaks.
    pub(crate) width: Option<f64>,
}

/// One laid-out line.
#[derive(Debug, Clone)]
pub(crate) struct LaidLine {
    /// The line's left edge, points from the box's left edge.
    pub(crate) x: f64,
    /// The width of what is drawn, points.
    pub(crate) width: f64,
    /// The line's bytes in the text, trailing spaces and the hard break included.
    pub(crate) range: Range<usize>,
    /// The bytes drawn.
    pub(crate) content: Range<usize>,
    /// The shaped runs in drawing (logical) order with their x from the line's left edge; empty for a
    /// standard 14 font, whose content is encoded when the page is written.
    pub(crate) runs: Vec<(ShapedRun, f64)>,
}

#[expect(
    clippy::cast_possible_truncation,
    reason = "layout widths and sizes are f32 in lamina-text; points far below f32::MAX"
)]
fn f32_of(v: f64) -> f32 {
    v as f32
}

/// Lays `spec` out: through `lamina-text`'s shaping, bidi and greedy line breaking, or, for a
/// standard 14 font, with the AFM widths.
pub(crate) fn lay_out(spec: &Spec) -> Result<Vec<LaidLine>> {
    if spec.font.is_standard() {
        return simple_lines(spec);
    }
    let mut options = LayoutOptions::new(spec.width.map_or(f32::MAX, f32_of), f32_of(spec.size)).with_align(spec.align);
    if let Some(direction) = spec.direction {
        options = options.with_direction(direction);
    }
    if let Some(language) = &spec.language {
        options = options.with_language(language.clone());
    }
    for feature in &spec.features {
        options = options.with_feature(*feature);
    }
    let lines = break_lines(&HarfRust, slice::from_ref(&spec.font.font), &spec.text, &options)?;
    Ok(lines
        .into_iter()
        .map(|line| {
            // Drawn in logical order (each run at its own x), so extractors that read in
            // content order get multi-run right-to-left lines in reading order (B4d).
            // `Line::drawing_order`, moving the runs: a stable sort by where each run's text
            // starts, as that method's sort of their indices is.
            let mut runs = line.runs;
            runs.sort_by_key(|r| r.run.text_range.start);
            let runs = runs.into_iter().map(|r| (r.run, r.x)).collect();
            LaidLine {
                x: line.x,
                width: line.width,
                range: line.range,
                content: line.content,
                runs,
            }
        })
        .collect())
}

/// Whether `ch` ends a paragraph (bidi class `B`, UAX #9 P1), as `lamina-text`'s layout splits
/// them: line feed, carriage return, U+001C–001E, U+0085, U+2029.
fn is_paragraph_separator(ch: char) -> bool {
    matches!(ch, '\n' | '\r' | '\u{1C}'..='\u{1E}' | '\u{85}' | '\u{2029}')
}

/// Whether `ch` hangs at a line's end, neither drawn nor measured: `lamina-text`'s rule
/// (spaces and hard breaks, UAX #14) plus the paragraph separators.
fn is_trailing(ch: char) -> bool {
    lamina_text::is_trailing_space(ch) || is_paragraph_separator(ch)
}

/// `range` of `text` without its trailing spaces and hard breaks.
fn content_of(text: &str, range: Range<usize>) -> Range<usize> {
    let trimmed = text
        .get(range.clone())
        .unwrap_or_default()
        .trim_end_matches(is_trailing);
    range.start..range.start + trimmed.len()
}

/// The paragraphs of `text`, each ending after its separator (CR LF together), as
/// `lamina-text`'s `break_lines` resolves them.
fn paragraphs(text: &str) -> Vec<Range<usize>> {
    let mut out = Vec::new();
    let mut start = 0;
    let mut chars = text.char_indices().peekable();
    while let Some((at, ch)) = chars.next() {
        if !is_paragraph_separator(ch) {
            continue;
        }
        let mut end = at + ch.len_utf8();
        if ch == '\r' && chars.peek().is_some_and(|&(_, next)| next == '\n') {
            chars.next();
            end += 1;
        }
        out.push(start..end);
        start = end;
    }
    if start < text.len() || out.is_empty() {
        out.push(start..text.len());
    }
    out
}

/// Greedy lines for a standard 14 font: left to right, one glyph per character (the font
/// has no program to shape with), each paragraph on its own, at UAX #14 opportunities, an
/// overlong word broken between grapheme clusters: `lamina-text`'s `break_lines` rules,
/// measured with the AFM widths instead of a shaper.
fn simple_lines(spec: &Spec) -> Result<Vec<LaidLine>> {
    let mut lines = Vec::new();
    for paragraph in paragraphs(&spec.text) {
        simple_paragraph(spec, paragraph, &mut lines)?;
    }
    Ok(lines)
}

/// The lines of the paragraph `para` of `spec.text`.
fn simple_paragraph(spec: &Spec, para: Range<usize>, lines: &mut Vec<LaidLine>) -> Result<()> {
    let font = &spec.font;
    let text = spec.text.as_str();
    let measure = |range: Range<usize>| -> Result<f64> {
        let mut width = 0.0;
        for ch in text.get(range).unwrap_or_default().chars() {
            let gid = font
                .font
                .glyph_for(ch)
                .ok_or_else(|| lamina_text::Error::MissingGlyph {
                    ch,
                    font: font.info().postscript_name.clone(),
                })?;
            width += font.scaled(font.font.advance(gid).unwrap_or(0.0), spec.size);
        }
        Ok(width)
    };
    let limit = spec.width.unwrap_or(f64::INFINITY);
    let line = |range: Range<usize>, width: f64| {
        let room = limit - width;
        let x = match spec.align {
            _ if !room.is_finite() => 0.0,
            Alignment::Start | Alignment::Left => 0.0,
            Alignment::End | Alignment::Right => room,
            Alignment::Center => room / 2.0,
        };
        LaidLine {
            x,
            width,
            content: content_of(text, range.clone()),
            range,
            runs: Vec::new(),
        }
    };
    let ptext = text.get(para.clone()).unwrap_or_default();
    let breaks: Vec<(usize, bool)> = line_breaks(ptext)
        .iter()
        .map(|b| (para.start + b.offset, b.mandatory))
        .collect();
    let mut start = para.start;
    let mut fit: Option<(usize, f64)> = None;
    let mut i = 0;
    while let Some(&(offset, mandatory)) = breaks.get(i) {
        let width = measure(content_of(text, start..offset))?;
        if width <= limit {
            if mandatory {
                lines.push(line(start..offset, width));
                start = offset;
                fit = None;
            } else {
                fit = Some((offset, width));
            }
            i += 1;
            continue;
        }
        if let Some((end, width)) = fit.take() {
            lines.push(line(start..end, width));
            start = end;
            continue;
        }
        // One word wider than the box: as many grapheme clusters as fit (at least one).
        let word = content_of(text, start..offset);
        let mut end = None;
        for boundary in grapheme_boundaries(text.get(word.clone()).unwrap_or_default()) {
            let at = word.start + boundary;
            if at == start {
                continue;
            }
            let w = measure(start..at)?;
            if w > limit && end.is_some() {
                break;
            }
            end = Some((at, w));
        }
        let Some((end, width)) = end else {
            // Only spaces: nothing to draw before the opportunity.
            lines.push(line(start..offset, 0.0));
            start = offset;
            i += 1;
            continue;
        };
        if end >= word.end {
            // A single cluster wider than the box: it takes the line on its own.
            lines.push(line(start..offset, width));
            start = offset;
            i += 1;
        } else {
            lines.push(line(start..end, width));
            start = end;
        }
    }
    Ok(())
}

/// Text recorded on a surface: what was asked for and how it was laid out.
#[derive(Debug, Clone)]
pub(crate) struct TextCommand {
    pub(crate) spec: Spec,
    /// The box's left edge and the first line's baseline, page space.
    pub(crate) origin: PointPt,
    /// From one baseline to the next, points.
    pub(crate) leading: f64,
    pub(crate) lines: Vec<LaidLine>,
    pub(crate) fill: Option<Color>,
    pub(crate) mode: TextRenderMode,
    pub(crate) link: Option<LinkSpec>,
    /// The structure element the run is (its own, inside the surface's current one).
    pub(crate) tag: Option<Box<Tag>>,
    /// The surface's transform when the text was drawn (page space), for the link's
    /// rectangle.
    pub(crate) ctm: Matrix,
}

impl TextCommand {
    /// The baseline of line `i`, page space.
    pub(crate) fn baseline(&self, i: usize) -> f64 {
        #[expect(clippy::cast_precision_loss, reason = "line counts are small")]
        let i = i as f64;
        self.origin.y.get() + i * self.leading
    }

    /// The box the drawn lines cover, from the top of the ascent of the first to the bottom
    /// of the descent of the last, before the surface's transform; `None` without lines.
    pub(crate) fn bounds(&self) -> Option<RectPt> {
        let last = self.lines.len().checked_sub(1)?;
        let x0 = self.lines.iter().map(|l| l.x).fold(f64::INFINITY, f64::min);
        let x1 = self
            .lines
            .iter()
            .map(|l| l.x + l.width)
            .fold(f64::NEG_INFINITY, f64::max);
        let x = self.origin.x.get();
        let size = self.spec.size;
        Some(RectPt::new(
            Pt::new(x + x0),
            Pt::new(self.baseline(0) - self.spec.font.ascent(size)),
            Pt::new(x + x1),
            Pt::new(self.baseline(last) + self.spec.font.descent(size)),
        ))
    }
}

/// A run of text drawn by [`Surface::text`]: change how it is drawn (colour, render mode,
/// OpenType features, language, direction) or make it a link. Changes that affect shaping
/// lay the run out again. Dropping the handle keeps the text.
///
/// ```
/// use lamina_build::{BuildOptions, Builder, Color, LinkTarget, PageSize, Standard14, TextRenderMode};
/// use lamina_model::{PointPt, Pt};
///
/// let mut b = Builder::new(BuildOptions::default());
/// let helvetica = b.standard_font(Standard14::Helvetica)?;
/// let mut page = b.page(PageSize::A4);
/// let run = page
///     .surface()
///     .text("lamina", &helvetica, Pt::new(12.0), PointPt::from_points(72.0, 72.0))?
///     .fill(Color::rgb(0.1, 0.3, 0.7))
///     .link(LinkTarget::Uri("https://example.com".into()), None);
/// assert!(run.width().get() > 30.0);
/// page.finish()?;
/// # Ok::<(), lamina_build::Error>(())
/// ```
#[derive(Debug)]
#[must_use = "the text is drawn either way; the handle only changes how"]
pub struct TextRun<'s> {
    surface: &'s mut Surface,
    index: usize,
}

impl<'s> TextRun<'s> {
    pub(crate) fn new(surface: &'s mut Surface, index: usize) -> Self {
        Self { surface, index }
    }

    fn command(&self) -> Option<&TextCommand> {
        self.surface.text_command(self.index)
    }

    fn edit(self, change: impl FnOnce(&mut TextCommand)) -> Self {
        if let Some(command) = self.surface.text_command_mut(self.index) {
            change(command);
        }
        self
    }

    fn relayout(self, change: impl FnOnce(&mut Spec)) -> Result<Self> {
        if let Some(command) = self.surface.text_command_mut(self.index) {
            let mut spec = command.spec.clone();
            change(&mut spec);
            command.lines = lay_out(&spec)?;
            command.spec = spec;
        }
        Ok(self)
    }

    /// Fills this run with `color` (the surface's fill colour is unchanged after it).
    pub fn fill(self, color: Color) -> Self {
        self.edit(|c| c.fill = Some(color))
    }

    /// How the glyphs are painted (`Tr`).
    pub fn render_mode(self, mode: TextRenderMode) -> Self {
        self.edit(|c| c.mode = mode)
    }

    /// Makes the run a link: a link annotation over its bounding box (after the surface's
    /// transform, as an axis-aligned box), with `contents` as its `/Contents`.
    pub fn link(self, target: LinkTarget, contents: Option<String>) -> Self {
        self.edit(|c| c.link = Some(LinkSpec { target, contents }))
    }

    /// Makes the run the structure element `tag` (§14.7.2), a child of the surface's
    /// innermost open element: the same as drawing it inside [`Surface::tag`]. A run that is
    /// also a link holds a `Link` element with the text and the annotation.
    pub fn tag(self, tag: impl Into<Tag>) -> Self {
        let tag = Box::new(tag.into());
        self.edit(|c| c.tag = Some(tag))
    }

    /// Shapes the run with OpenType `features` on top of the defaults (`liga=0`, `smcp`).
    /// Ignored for a standard 14 font, which isn't shaped.
    ///
    /// # Errors
    /// As [`Surface::text`].
    pub fn features(self, features: &[Feature]) -> Result<Self> {
        self.relayout(|s| s.features = features.to_vec())
    }

    /// The text's language (a BCP 47 tag), for language-specific forms.
    ///
    /// # Errors
    /// As [`Surface::text`].
    pub fn language(self, language: &str) -> Result<Self> {
        self.relayout(|s| s.language = Some(language.to_owned()))
    }

    /// The paragraph direction (default: from the first strong character, UAX #9).
    ///
    /// # Errors
    /// As [`Surface::text`].
    pub fn direction(self, direction: Direction) -> Result<Self> {
        self.relayout(|s| s.direction = Some(direction))
    }

    /// The width of the widest line, points.
    #[must_use]
    pub fn width(&self) -> Pt {
        Pt::new(
            self.command()
                .map_or(0.0, |c| c.lines.iter().map(|l| l.width).fold(0.0, f64::max)),
        )
    }

    /// The box the run covers on the page (before the surface's transform): from the top of
    /// the font's ascent above the first baseline to the bottom of its descent below the last.
    /// Empty text has an empty box at the origin.
    #[must_use]
    pub fn bounds(&self) -> RectPt {
        match self.command() {
            Some(c) => c
                .bounds()
                .unwrap_or_else(|| RectPt::new(c.origin.x, c.origin.y, c.origin.x, c.origin.y)),
            None => RectPt::new(Pt::ZERO, Pt::ZERO, Pt::ZERO, Pt::ZERO),
        }
    }

    /// The number of lines (more than one only when the text has hard line breaks).
    #[must_use]
    pub fn lines(&self) -> usize {
        self.command().map_or(0, |c| c.lines.len())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn content_drops_trailing_spaces_and_breaks() {
        assert_eq!(content_of("ab  \n", 0..5), 0..2);
    }
}
