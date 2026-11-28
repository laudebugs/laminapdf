//! `SetTextStyle`: change how existing text looks (E7 plan §2.1): its font, size, colour, bold,
//! italic, underline, baseline, alignment and line spacing. A style that keeps the run's glyphs
//! (size, colour, rise) draws them between the operators that set it and the ones that put the
//! object's own back; a new font draws the run's text again in that font. Alignment and spacing
//! move whole lines of the run's text object.

use std::collections::{BTreeMap, BTreeSet};
use std::ops::Range;

use laminapdf_content::{
    ContentObjectId, ContentObjectKind, ContentOp, ContentOps, FillColour, LineShift, PageContentWriter, Segment,
    SegmentStyle, ShownGlyph,
};
use laminapdf_model::PageId;
use laminapdf_syntax::Object;
use laminapdf_text::{Font, Glyph, Standard14};
use serde::{Deserialize, Serialize};

use super::fonts::{Chain, fidelity, standard};
use super::text::{Reading, cos_page, joined, shown_glyph};
use super::{TextRunRef, find};
use crate::annot::{AnnotColor, AnnotFont, StandardFamily, TextAlign};
use crate::blob::BlobStore;
use crate::error::{Error, Result};
use crate::op::Effect;
use crate::text_edit::TextEditReport;

/// How much smaller superscript and subscript text is than the text around it.
const SCRIPT_SCALE: f64 = 0.65;
/// How far superscript text rises, as a fraction of its own size.
const SUPER_RISE: f64 = 0.5;
/// How far subscript text drops, as a fraction of its own size.
const SUB_RISE: f64 = -0.2;
/// How far below the baseline an underline sits, as a fraction of the glyphs' size.
const UNDERLINE_OFFSET: f64 = 0.12;
/// An underline's thickness, as a fraction of the glyphs' size.
const UNDERLINE_WIDTH: f64 = 0.06;
/// How far apart two baselines may be, as a fraction of the glyphs' size, and still be one line:
/// superscript and subscript text keep its line.
const SAME_LINE: f64 = 0.45;

/// Where text sits on its line.
///
/// Added in 0.7.0.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "tsify", derive(tsify::Tsify))]
#[non_exhaustive]
pub enum Baseline {
    /// On the baseline, at the size it is.
    Normal,
    /// Raised and smaller.
    Super,
    /// Lowered and smaller.
    Sub,
}

/// Changes how runs of text look ([E7 plan] §2.1); each field is optional and changes only what
/// it names.
///
/// * `font` draws the run's text again in a standard family (`helvetica`, `times`, `courier`) or
///   in a font whose bytes were added to the session, which is embedded as a new subset; every
///   character must have a glyph in it (`missing-glyph` otherwise). The run's own font object
///   stays as it was for the text that uses it.
/// * `bold` and `italic` pick the face: the family's, when `font` is a standard family, or the
///   bold or italic face of the run's own font when that is one of the standard 14 fonts. A font
///   given by its bytes is one face: name the bold face as `font` instead. Setting either to
///   `false` picks the regular face.
/// * `size` is the glyphs' new size in points; `colour` the fill colour.
/// * `underline` draws a line under the run, as a path after the text. Lamina PDF doesn't read an
///   underline back to take it off, so `false` is refused.
/// * `baseline` raises or lowers the run at 65 % of its size (`normal` puts it back on the
///   baseline at the size it has).
/// * `align` moves each line of the run's text object, so the line starts at the block's left
///   edge, ends at its right edge or sits in the middle of it; the block is the widest extent
///   of all the object's lines. `lineSpacing` sets the distance from the line of the first run to
///   the lines of the others in the object, as a multiple of the glyphs' size, which is what
///   a paragraph's line height is.
///
/// Rotated, mirrored and right-to-left runs, and runs a font or an object stops editing, fail
/// with `content-not-editable`, as for [`EditText`](crate::EditText). The runs may be in
/// several text objects; none may overlap another. Saving is an incremental update.
///
/// On the wire: `{ "op": "setTextStyle", "args": { "page": "p12.0", "runs": [{ "object": {
/// "ordinal": 3, "hash": "a3f1c2…" }, "start": 0, "end": 5 }], "bold": true, "color": [0.8, 0, 0] } }`.
///
/// [E7 plan]: https://github.com/laudebugs/laminapdf/blob/main/docs/plans/editor/e7-editing-existing-content.md
///
/// Added in 0.7.0.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "tsify", derive(tsify::Tsify))]
#[non_exhaustive]
pub struct SetTextStyle {
    /// The page.
    #[serde(with = "crate::wire::page_id")]
    #[cfg_attr(feature = "tsify", tsify(type = "string"))]
    pub page: PageId,
    /// The runs, as `text_runs` names them.
    pub runs: Vec<TextRunRef>,
    /// The font to draw the runs in.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "tsify", tsify(optional, type = "string"))]
    pub font: Option<AnnotFont>,
    /// The glyphs' size in points, from 1 to 500.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "tsify", tsify(optional))]
    pub size: Option<f64>,
    /// The fill colour.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "tsify", tsify(optional))]
    pub color: Option<AnnotColor>,
    /// Bold, or the regular weight.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "tsify", tsify(optional))]
    pub bold: Option<bool>,
    /// Italic, or upright.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "tsify", tsify(optional))]
    pub italic: Option<bool>,
    /// Draws a line under the runs (`true` only).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "tsify", tsify(optional))]
    pub underline: Option<bool>,
    /// Raised, lowered or normal.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "tsify", tsify(optional))]
    pub baseline: Option<Baseline>,
    /// Where the runs' lines start in their text object's block.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "tsify", tsify(optional))]
    pub align: Option<TextAlign>,
    /// The distance between the baselines of the runs' lines, as a multiple of the glyphs' size
    /// (0.5 to 10).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "tsify", tsify(optional))]
    pub line_spacing: Option<f64>,
}

// Ops compare by value (`Op: Eq`); NaN never reaches a style from JSON, and is refused as
// `invalid-op` when a caller builds one.
impl Eq for SetTextStyle {}

impl SetTextStyle {
    /// An op that names `runs` on `page` and changes nothing yet: set the fields it changes.
    ///
    /// Added in 0.7.0.
    #[must_use]
    pub const fn new(page: PageId, runs: Vec<TextRunRef>) -> Self {
        Self {
            page,
            runs,
            font: None,
            size: None,
            color: None,
            bold: None,
            italic: None,
            underline: None,
            baseline: None,
            align: None,
            line_spacing: None,
        }
    }

    /// Whether any field changes how a run's glyphs are drawn (the others move lines).
    const fn restyles(&self) -> bool {
        self.font.is_some()
            || self.size.is_some()
            || self.color.is_some()
            || self.bold.is_some()
            || self.italic.is_some()
            || self.underline.is_some()
            || self.baseline.is_some()
    }

    /// Refuses an op that names no run or changes nothing, or whose numbers are out of range.
    fn check(&self) -> Result<()> {
        if self.runs.is_empty() {
            return Err(Error::InvalidOp("setTextStyle needs at least one run"));
        }
        if !self.restyles() && self.align.is_none() && self.line_spacing.is_none() {
            return Err(Error::InvalidOp("setTextStyle changes nothing"));
        }
        if self.underline == Some(false) {
            return Err(Error::InvalidOp(
                "an underline Lamina PDF didn't draw can't be taken off: leave underline out",
            ));
        }
        let size_ok = self.size.is_none_or(|s| (1.0..=500.0).contains(&s));
        let spacing_ok = self.line_spacing.is_none_or(|s| (0.5..=10.0).contains(&s));
        let color_ok = self.color.is_none_or(|c| match c {
            AnnotColor::Gray(g) => (0.0..=1.0).contains(&g),
            AnnotColor::Rgb(r, g, b) => [r, g, b].iter().all(|v| (0.0..=1.0).contains(v)),
            AnnotColor::Cmyk(c, m, y, k) => [c, m, y, k].iter().all(|v| (0.0..=1.0).contains(v)),
        });
        if size_ok && spacing_ok && color_ok {
            Ok(())
        } else {
            Err(Error::InvalidOp(
                "setTextStyle's size, colour or line spacing is out of range",
            ))
        }
    }

    pub(crate) fn apply_to(&self, doc: &mut laminapdf_model::Document, blobs: &BlobStore) -> Result<Effect> {
        self.check()?;
        let first = Reading::of(doc, &self.page)?;
        // Each run's object, glyphs and line, found in the page as it is before the op.
        let mut targets: Vec<Target> = Vec::with_capacity(self.runs.len());
        for run in &self.runs {
            let id = find(&first.ir, &self.page, run.object)?;
            let line = first.line_of(id, run.start as usize);
            targets.push(Target {
                ordinal: run.object.ordinal,
                glyphs: run.start as usize..run.end as usize,
                line,
            });
        }
        // From the last run to the first, so a run's glyph numbers stay what the op names.
        targets.sort_by_key(|t| std::cmp::Reverse((t.ordinal, t.glyphs.start)));
        for pair in targets.windows(2) {
            if let [later, earlier] = pair
                && later.ordinal == earlier.ordinal
                && earlier.glyphs.end > later.glyphs.start
            {
                return Err(Error::InvalidOp("setTextStyle's runs overlap"));
            }
        }
        let mut reports = Vec::new();
        if self.restyles() {
            for target in &targets {
                reports.push(self.style_run(doc, blobs, target)?);
            }
        }
        if self.align.is_some() || self.line_spacing.is_some() {
            self.lay_out(doc, &targets)?;
        }
        Ok(Effect {
            pages: vec![self.page.clone()],
            text_edits: reports,
            ..Effect::default()
        })
    }

    /// Restyles one run, in a page read as the earlier runs left it.
    fn style_run(
        &self,
        doc: &mut laminapdf_model::Document,
        blobs: &BlobStore,
        target: &Target,
    ) -> Result<TextEditReport> {
        let mut reading = Reading::of(doc, &self.page)?;
        let id = reading.object_at(target.ordinal)?;
        let shown_ids = reading.objects.get(&id).cloned().unwrap_or_default();
        let run = shown_ids
            .get(target.glyphs.clone())
            .filter(|run| !run.is_empty())
            .ok_or(Error::InvalidOp("setTextStyle's glyphs aren't among the object's"))?;
        if !reading.one_show(run) {
            return Err(Error::InvalidOp(
                "setTextStyle's glyphs aren't drawn by one show operator",
            ));
        }
        let font = reading
            .font_of(run)
            .ok_or(Error::InvalidOp("setTextStyle's glyphs aren't in one font"))?;
        reading.load(doc, font)?;
        if let Some(reason) = reading.ir.edit_refusal(id).or_else(|| reading.refusal(run, font)) {
            return Err(Error::ContentNotEditable { reason });
        }
        if reading.ir.object(id).map(laminapdf_content::ContentObject::kind) != Some(ContentObjectKind::Text) {
            return Err(Error::InvalidOp("setTextStyle names an object that isn't text"));
        }
        let encoder = reading
            .encoders
            .get(&font)
            .ok_or(Error::InvalidOp("the font wasn't read"))?;
        let picked: Vec<&Glyph> = run.iter().filter_map(|&g| reading.text.glyphs.get(g)).collect();
        let first = picked
            .first()
            .ok_or(Error::InvalidOp("setTextStyle's run has no glyphs"))?;
        let (size, own_name) = (first.size, first.font_name.to_string());
        let ex = direction(&picked);
        let old_len: f64 = picked.iter().map(|g| g.advance.0.hypot(g.advance.1)).sum();
        let origin = first.origin;
        let color = first.color;
        let shown: Vec<ShownGlyph> = shown_ids
            .iter()
            .filter_map(|&g| reading.text.glyphs.get(g))
            .map(|g| shown_glyph(encoder.glyph(g.code)))
            .collect();
        let old_text = joined(&picked);

        let (mut style, scale) = self.glyph_style(size);

        let page = cos_page(doc, &self.page)?;
        let new_font = self.new_font(&own_name, blobs)?;
        let mut writer = PageContentWriter::new(doc.cos_mut().edit(), &page)?;
        let new_len;
        let mut chain = None;
        if let Some(new_font) = new_font {
            // The run's text drawn again in the new font, which must have a glyph for each character.
            let mut fonts = Chain::only(new_font)?;
            let glyphs = fonts.encode(0, &old_text)?;
            let names = fonts.names(&mut writer)?;
            let name = names
                .get(&0)
                .cloned()
                .ok_or(Error::InvalidOp("the font wasn't added to the page"))?;
            style = style.with_font(name);
            new_len = glyphs.iter().map(|g| g.width / 1000.0 * size * scale).sum::<f64>();
            let segment = Segment::new(glyphs).styled(style);
            reading
                .ir
                .rewrite_segments(id, &shown, target.glyphs.clone(), &[segment])
                .map_err(|reason| Error::ContentNotEditable { reason })?;
            chain = Some(fonts);
        } else {
            new_len = old_len * scale;
            reading
                .ir
                .restyle_text(id, &shown, target.glyphs.clone(), &style)
                .map_err(|reason| Error::ContentNotEditable { reason })?;
        }
        if self.underline == Some(true) {
            let stroke = self.color.map_or_else(
                || color.map_or(FillColour::Gray(0.0), |[r, g, b]| FillColour::Rgb(r, g, b)),
                fill,
            );
            let extra = underline(&reading.ir, id, origin, ex, new_len, size * scale, stroke)?;
            reading
                .ir
                .append_to_object(id, extra)
                .map_err(|reason| Error::ContentNotEditable { reason })?;
        }
        writer.write_parts(&reading.ir, &reading.ir.encode())?;
        if let Some(chain) = chain {
            chain.write(writer.cos())?;
        }
        drop(writer);
        let grew = new_len - old_len;
        Ok(TextEditReport {
            page: self.page.clone(),
            ordinal: target.ordinal,
            start: u32::try_from(target.glyphs.start).unwrap_or(u32::MAX),
            end: u32::try_from(target.glyphs.end).unwrap_or(u32::MAX),
            fidelity: fidelity(&[], grew),
            substitutions: Vec::new(),
            grew_by: grew.max(0.0),
        })
    }

    /// How the glyphs of a run drawn at `size` are styled, and by what factor their size changes.
    fn glyph_style(&self, size: f64) -> (SegmentStyle, f64) {
        let mut style = SegmentStyle::new();
        let mut scale = self.size.map_or(1.0, |s| s / size);
        match self.baseline {
            Some(Baseline::Super) => {
                scale *= SCRIPT_SCALE;
                style = style.with_rise(SUPER_RISE);
            }
            Some(Baseline::Sub) => {
                scale *= SCRIPT_SCALE;
                style = style.with_rise(SUB_RISE);
            }
            Some(Baseline::Normal) => style = style.with_rise(0.0),
            None => {}
        }
        if (scale - 1.0).abs() > f64::EPSILON {
            style = style.with_scale(scale);
        }
        if let Some(color) = self.color {
            style = style.with_fill(fill(color));
        }
        (style, scale)
    }

    /// The font the runs are drawn in, when it changes: `font`, or the face of the run's own
    /// standard font that `bold` and `italic` ask for.
    fn new_font(&self, own_name: &str, blobs: &BlobStore) -> Result<Option<Font>> {
        let own = Standard14::from_base_font(own_name);
        let (own_bold, own_italic) = own.map_or((false, false), |f| style_of(f.base_font()));
        match &self.font {
            Some(AnnotFont::Font(id)) => {
                if self.bold.is_some() || self.italic.is_some() {
                    return Err(Error::InvalidOp(
                        "a font given by its bytes is one face: name the bold or italic face as the font",
                    ));
                }
                let bytes = blobs.get(id).ok_or(Error::UnknownBlob(*id))?;
                Ok(Some(Font::from_bytes(bytes.clone()).map_err(Error::Text)?))
            }
            Some(AnnotFont::Standard(family)) => Ok(Some(Font::standard(standard(
                *family,
                self.bold.unwrap_or(own_bold),
                self.italic.unwrap_or(own_italic),
            )))),
            None if self.bold.is_some() || self.italic.is_some() => {
                let family = own.and_then(|f| family_of(f.base_font())).ok_or(Error::InvalidOp(
                    "bold and italic need a standard font to take the face from: set font",
                ))?;
                let face = standard(family, self.bold.unwrap_or(own_bold), self.italic.unwrap_or(own_italic));
                Ok((Some(face) != own).then(|| Font::standard(face)))
            }
            None => Ok(None),
        }
    }

    /// Moves the lines of the runs' text objects: aligns them in the object's block and spaces
    /// them, in the page as the restyling left it.
    fn lay_out(&self, doc: &mut laminapdf_model::Document, targets: &[Target]) -> Result<()> {
        let mut reading = Reading::of(doc, &self.page)?;
        let mut by_object: BTreeMap<u32, BTreeSet<usize>> = BTreeMap::new();
        for target in targets {
            if let Some(line) = target.line {
                by_object.entry(target.ordinal).or_default().insert(line);
            }
        }
        for (ordinal, wanted) in by_object {
            let id = reading.object_at(ordinal)?;
            let lines = reading.lines_of(id);
            if let Some(reason) = reading.ir.edit_refusal(id) {
                return Err(Error::ContentNotEditable { reason });
            }
            let block = (
                lines.iter().map(|l| l.start).fold(f64::INFINITY, f64::min),
                lines.iter().map(|l| l.end).fold(f64::NEG_INFINITY, f64::max),
            );
            let mut shifts = Vec::new();
            let mut anchor: Option<(f64, f64)> = None;
            let mut placed = 0.0;
            for line in lines
                .iter()
                .enumerate()
                .filter(|(i, _)| wanted.contains(i))
                .map(|(_, l)| l)
            {
                let along = match self.align {
                    Some(TextAlign::Left) => block.0 - line.start,
                    Some(TextAlign::Center) => f64::midpoint(block.0, block.1) - f64::midpoint(line.start, line.end),
                    Some(TextAlign::Right) => block.1 - line.end,
                    None => 0.0,
                };
                let mut up = 0.0;
                if let Some(spacing) = self.line_spacing {
                    match anchor {
                        None => anchor = Some((line.across, line.size)),
                        Some((top, _)) => {
                            placed += 1.0;
                            up = top - placed * spacing * line.size - line.across;
                        }
                    }
                }
                if along.abs() > f64::EPSILON || up.abs() > f64::EPSILON {
                    shifts.push(LineShift::new(line.show_first, line.show_last, line.size, along, up));
                }
            }
            if shifts.is_empty() {
                continue;
            }
            reading
                .ir
                .shift_lines(id, &shifts)
                .map_err(|reason| Error::ContentNotEditable { reason })?;
        }
        let page = cos_page(doc, &self.page)?;
        let mut writer = PageContentWriter::new(doc.cos_mut().edit(), &page)?;
        writer.write_parts(&reading.ir, &reading.ir.encode())?;
        Ok(())
    }
}

/// A run the op names, in the page as it was before the op.
struct Target {
    ordinal: u32,
    glyphs: Range<usize>,
    /// Its line among its object's, which restyling doesn't change.
    line: Option<usize>,
}

/// A line of a text object, in user space along the object's writing direction.
pub(super) struct TextLine {
    /// Where the line starts along the baseline.
    pub(super) start: f64,
    /// Where it ends.
    pub(super) end: f64,
    /// Where its baseline is across the writing direction, up positive.
    pub(super) across: f64,
    /// The size of its first glyph.
    pub(super) size: f64,
    /// Its first and last show operator among the object's.
    pub(super) show_first: usize,
    pub(super) show_last: usize,
}

impl Reading {
    /// The object with this ordinal among the page's.
    ///
    /// # Errors
    /// `content-object-stale` when the page has none.
    pub(super) fn object_at(&self, ordinal: u32) -> Result<ContentObjectId> {
        self.ir
            .objects()
            .iter()
            .map(laminapdf_content::ContentObject::id)
            .find(|&id| self.ir.reference(id).is_some_and(|r| r.ordinal == ordinal))
            .ok_or(Error::ContentObjectStale { ordinal })
    }

    /// The text object's lines, in the order it draws them.
    pub(super) fn lines_of(&self, id: ContentObjectId) -> Vec<TextLine> {
        let glyphs: Vec<&Glyph> = self
            .objects
            .get(&id)
            .into_iter()
            .flatten()
            .filter_map(|&g| self.text.glyphs.get(g))
            .collect();
        let Some(base) = glyphs.first().map(|g| g.origin) else {
            return Vec::new();
        };
        let ex = direction(&glyphs);
        let n = (-ex.1, ex.0);
        let at = |g: &Glyph| {
            let (dx, dy) = (g.origin.x - base.x, g.origin.y - base.y);
            (dx * ex.0 + dy * ex.1, dx * n.0 + dy * n.1)
        };
        let mut out: Vec<TextLine> = Vec::new();
        for glyph in glyphs {
            let (along, across) = at(glyph);
            let end = along + glyph.advance.0 * ex.0 + glyph.advance.1 * ex.1;
            let show = usize::try_from(glyph.show)
                .ok()
                .and_then(|s| self.local_shows.get(s).copied().flatten())
                .unwrap_or(0);
            match out.last_mut() {
                Some(line) if (line.across - across).abs() <= SAME_LINE * glyph.size => {
                    line.start = line.start.min(along);
                    line.end = line.end.max(end);
                    line.show_first = line.show_first.min(show);
                    line.show_last = line.show_last.max(show);
                }
                _ => out.push(TextLine {
                    start: along.min(end),
                    end: along.max(end),
                    across,
                    size: glyph.size,
                    show_first: show,
                    show_last: show,
                }),
            }
        }
        out
    }

    /// The line of object `id` that its glyph `glyph` is on.
    fn line_of(&self, id: ContentObjectId, glyph: usize) -> Option<usize> {
        let shown = self.objects.get(&id)?;
        let target = self.text.glyphs.get(*shown.get(glyph)?)?;
        let lines = self.lines_of(id);
        let base = self.text.glyphs.get(*shown.first()?)?.origin;
        let glyphs: Vec<&Glyph> = shown.iter().filter_map(|&g| self.text.glyphs.get(g)).collect();
        let ex = direction(&glyphs);
        let across = (target.origin.x - base.x) * -ex.1 + (target.origin.y - base.y) * ex.0;
        lines
            .iter()
            .position(|l| (l.across - across).abs() <= SAME_LINE * target.size)
    }
}

/// The unit vector the glyphs advance along; to the right when none advances.
fn direction(glyphs: &[&Glyph]) -> (f64, f64) {
    glyphs
        .iter()
        .map(|g| g.advance)
        .find(|a| a.0.hypot(a.1) > f64::EPSILON)
        .map_or((1.0, 0.0), |a| {
            let length = a.0.hypot(a.1);
            (a.0 / length, a.1 / length)
        })
}

/// A colour as the fill the content crate sets.
fn fill(color: AnnotColor) -> FillColour {
    match color {
        AnnotColor::Gray(g) => FillColour::Gray(g),
        AnnotColor::Rgb(r, g, b) => FillColour::Rgb(r, g, b),
        AnnotColor::Cmyk(c, m, y, k) => FillColour::Cmyk(c, m, y, k),
    }
}

/// Whether a standard 14 face is bold and whether it is italic.
fn style_of(base_font: &str) -> (bool, bool) {
    (
        base_font.contains("Bold"),
        base_font.contains("Italic") || base_font.contains("Oblique"),
    )
}

/// The family a standard 14 font belongs to; `None` for Symbol and Zapf Dingbats.
fn family_of(base_font: &str) -> Option<StandardFamily> {
    if base_font.starts_with("Helvetica") {
        Some(StandardFamily::Helvetica)
    } else if base_font.starts_with("Times") {
        Some(StandardFamily::Times)
    } else if base_font.starts_with("Courier") {
        Some(StandardFamily::Courier)
    } else {
        None
    }
}

/// The operators that draw a line under `length` points of text that starts at `origin` (user
/// space) and runs along `ex`, in the space the text object is drawn in: stroked in `stroke`,
/// below the baseline of glyphs of `size`.
fn underline(
    ir: &ContentOps,
    id: ContentObjectId,
    origin: laminapdf_model::UserSpacePoint,
    ex: (f64, f64),
    length: f64,
    size: f64,
    stroke: FillColour,
) -> Result<Vec<ContentOp>> {
    let ctm = ir
        .object(id)
        .and_then(laminapdf_content::ContentObject::state)
        .map(|s| s.ctm)
        .ok_or(Error::ContentNotEditable {
            reason: laminapdf_content::NotEditable::ContentUnreadable,
        })?;
    let inverse = ctm.invert().ok_or(Error::ContentNotEditable {
        reason: laminapdf_content::NotEditable::ContentUnreadable,
    })?;
    let down = (ex.1 * UNDERLINE_OFFSET * size, -ex.0 * UNDERLINE_OFFSET * size);
    let from = (origin.x + down.0, origin.y + down.1);
    let to = (from.0 + ex.0 * length, from.1 + ex.1 * length);
    let local = |p: (f64, f64)| {
        (
            inverse.a * p.0 + inverse.c * p.1 + inverse.e,
            inverse.b * p.0 + inverse.d * p.1 + inverse.f,
        )
    };
    let scale = (ctm.a * ctm.d - ctm.b * ctm.c).abs().sqrt();
    let width = if scale > f64::EPSILON {
        UNDERLINE_WIDTH * size / scale
    } else {
        UNDERLINE_WIDTH * size
    };
    let (start, end) = (local(from), local(to));
    let number = Object::number;
    let op = |keyword: &'static [u8], operands: Vec<Object>| {
        ContentOp::new(laminapdf_content::Operator::from_keyword(keyword), operands)
    };
    Ok(vec![
        op(b"q", Vec::new()),
        stroke_colour(stroke),
        op(b"w", vec![number(width)]),
        op(b"m", vec![number(start.0), number(start.1)]),
        op(b"l", vec![number(end.0), number(end.1)]),
        op(b"S", Vec::new()),
        op(b"Q", Vec::new()),
    ])
}

/// The operator that sets the stroking colour to `colour`.
fn stroke_colour(colour: FillColour) -> ContentOp {
    let numbers = |values: &[f64]| values.iter().copied().map(Object::number).collect::<Vec<_>>();
    let (keyword, operands): (&'static [u8], Vec<Object>) = match colour {
        FillColour::Rgb(red, green, blue) => (b"RG", numbers(&[red, green, blue])),
        FillColour::Cmyk(cyan, magenta, yellow, key) => (b"K", numbers(&[cyan, magenta, yellow, key])),
        FillColour::Gray(gray) => (b"G", numbers(&[gray])),
        _ => (b"G", numbers(&[0.0])),
    };
    ContentOp::new(laminapdf_content::Operator::from_keyword(keyword), operands)
}
