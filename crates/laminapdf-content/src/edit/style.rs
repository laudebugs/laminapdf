//! Styling the text a text object shows (ADR 0022 §6; E7 plan §2.1): new glyphs drawn in their
//! own font, size, colour or rise between the operators that set that style and the ones that put
//! back what the object had, and whole lines of an object moved along or across their baselines
//! to align them or space them.

use laminapdf_syntax::{Name, Object};

use super::edits::op;
use super::text::{ShownGlyph, Spacing, shows};
use super::{ContentObjectId, ContentOp, ContentOps, NotEditable};
use crate::state::{ColorSpace, CurrentColor, GraphicsState};

/// A fill colour in a device space.
///
/// Added in 0.7.0.
#[derive(Debug, Clone, Copy, PartialEq)]
#[non_exhaustive]
pub enum FillColour {
    /// `DeviceGray`.
    Gray(f64),
    /// `DeviceRGB`.
    Rgb(f64, f64, f64),
    /// `DeviceCMYK`.
    Cmyk(f64, f64, f64, f64),
}

impl FillColour {
    /// The operator that sets this fill colour (`g`, `rg` or `k`).
    fn operator(self) -> ContentOp {
        match self {
            Self::Gray(g) => op(b"g", vec![Object::number(g)]),
            Self::Rgb(r, g, b) => op(b"rg", [r, g, b].into_iter().map(Object::number).collect()),
            Self::Cmyk(c, m, y, k) => op(b"k", [c, m, y, k].into_iter().map(Object::number).collect()),
        }
    }
}

/// What a [`Segment`]'s glyphs are drawn with; what is `None` stays as the object has it.
///
/// Added in 0.7.0.
#[derive(Debug, Clone, Default, PartialEq)]
#[non_exhaustive]
pub struct SegmentStyle {
    /// The `/Font` resource name (`Tf`); the object's own font when `None`.
    pub font: Option<Name>,
    /// How many times the font size the object sets here the glyphs are drawn at (1 is the
    /// same size).
    pub scale: Option<f64>,
    /// The fill colour.
    pub fill: Option<FillColour>,
    /// The text rise (`Ts`) as a fraction of the glyphs' font size.
    pub rise: Option<f64>,
}

impl SegmentStyle {
    /// A style that changes nothing.
    ///
    /// Added in 0.7.0.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            font: None,
            scale: None,
            fill: None,
            rise: None,
        }
    }

    /// The same style with the font resource `name`.
    ///
    /// Added in 0.7.0.
    #[must_use]
    pub fn with_font(mut self, name: Name) -> Self {
        self.font = Some(name);
        self
    }

    /// The same style at `scale` times the object's font size.
    ///
    /// Added in 0.7.0.
    #[must_use]
    pub const fn with_scale(mut self, scale: f64) -> Self {
        self.scale = Some(scale);
        self
    }

    /// The same style with the fill colour `fill`.
    ///
    /// Added in 0.7.0.
    #[must_use]
    pub const fn with_fill(mut self, fill: FillColour) -> Self {
        self.fill = Some(fill);
        self
    }

    /// The same style with the text rise `rise`, a fraction of the font size.
    ///
    /// Added in 0.7.0.
    #[must_use]
    pub const fn with_rise(mut self, rise: f64) -> Self {
        self.rise = Some(rise);
        self
    }
}

/// New glyphs for [`ContentOps::rewrite_segments`] and the style they are drawn in.
///
/// Added in 0.7.0.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct Segment {
    /// The glyphs, with the codes and widths of the font they are drawn in.
    pub glyphs: Vec<ShownGlyph>,
    /// How they are drawn.
    pub style: SegmentStyle,
}

impl Segment {
    /// Glyphs drawn as the object draws the text around them.
    ///
    /// Added in 0.7.0.
    #[must_use]
    pub const fn new(glyphs: Vec<ShownGlyph>) -> Self {
        Self {
            glyphs,
            style: SegmentStyle::new(),
        }
    }

    /// The same glyphs drawn in `style`.
    ///
    /// Added in 0.7.0.
    #[must_use]
    pub fn styled(mut self, style: SegmentStyle) -> Self {
        self.style = style;
        self
    }
}

/// A segment ready to write: its operators before and after it and the size it is drawn at.
pub(super) struct Placed<'a> {
    pub(super) glyphs: &'a [ShownGlyph],
    pub(super) before: Vec<ContentOp>,
    pub(super) after: Vec<ContentOp>,
    pub(super) size: Option<f64>,
    /// Its first glyph among all the segments' glyphs.
    pub(super) offset: usize,
}

/// The operators that set `current` again.
fn color_ops(current: &CurrentColor) -> Vec<ContentOp> {
    let numbers = || {
        current
            .components
            .iter()
            .copied()
            .map(Object::number)
            .collect::<Vec<_>>()
    };
    match &current.space {
        ColorSpace::DeviceGray => vec![op(b"g", numbers())],
        ColorSpace::DeviceRgb => vec![op(b"rg", numbers())],
        ColorSpace::DeviceCmyk => vec![op(b"k", numbers())],
        ColorSpace::Pattern => {
            let mut operands = numbers();
            operands.extend(current.pattern.clone().map(Object::Name));
            vec![
                op(b"cs", vec![Object::Name(Name::new("Pattern"))]),
                op(b"scn", operands),
            ]
        }
        ColorSpace::Named(name) => vec![op(b"cs", vec![Object::Name(name.clone())]), op(b"scn", numbers())],
    }
}

/// What a text object's own operators have set so far, to put it back after a styled segment.
pub(super) struct Tracker {
    /// The operands of the last `Tf`, or the font the object starts with.
    font: Option<Vec<Object>>,
    /// The operators that make the fill colour what it is now.
    fill: Vec<ContentOp>,
    rise: f64,
}

impl Tracker {
    pub(super) fn new(state: &GraphicsState) -> Self {
        let font = state
            .text
            .font
            .as_ref()
            .and_then(|f| f.name.clone())
            .map(|name| vec![Object::Name(name), Object::number(state.text.font_size)]);
        Self {
            font,
            fill: color_ops(&state.fill),
            rise: state.text.rise,
        }
    }

    /// The font size the object sets here.
    fn size(&self) -> Option<f64> {
        self.font.as_ref()?.get(1)?.as_f64()
    }

    /// Follows an operator of the object.
    pub(super) fn follow(&mut self, original: &ContentOp) {
        match &original.keyword[..] {
            b"Tf" => self.font = Some(original.operands.clone()),
            b"Ts" => self.rise = original.operands.first().and_then(Object::as_f64).unwrap_or(0.0),
            b"g" | b"rg" | b"k" | b"cs" => self.fill = vec![original.clone()],
            b"sc" | b"scn" => {
                self.fill.retain(|o| &o.keyword[..] == b"cs");
                self.fill.push(original.clone());
            }
            _ => {}
        }
    }
}

/// The operators around each segment, from what `tracker` says the object has set here.
///
/// # Errors
/// [`NotEditable::ContentUnreadable`] for a font to put back that the object never named.
pub(super) fn place<'a>(segments: &'a [Segment], tracker: &Tracker) -> Result<Vec<Placed<'a>>, NotEditable> {
    let mut offset = 0;
    let mut out = Vec::with_capacity(segments.len());
    for segment in segments {
        let (mut before, mut after) = (Vec::new(), Vec::new());
        let mut size = None;
        if segment.style.font.is_some() || segment.style.scale.is_some() {
            let back = tracker.font.clone().ok_or(NotEditable::ContentUnreadable)?;
            let own = back.first().cloned().ok_or(NotEditable::ContentUnreadable)?;
            let current = back
                .get(1)
                .and_then(Object::as_f64)
                .ok_or(NotEditable::ContentUnreadable)?;
            let at = current * segment.style.scale.unwrap_or(1.0);
            let name = segment.style.font.clone().map_or(own, Object::Name);
            before.push(op(b"Tf", vec![name, Object::number(at)]));
            after.push(op(b"Tf", back));
            size = Some(at);
        }
        if let Some(fill) = segment.style.fill {
            before.push(fill.operator());
            after.extend(tracker.fill.clone());
        }
        if let Some(rise) = segment.style.rise {
            let em = size.or_else(|| tracker.size()).ok_or(NotEditable::ContentUnreadable)?;
            before.push(op(b"Ts", vec![Object::number(rise * em)]));
            after.push(op(b"Ts", vec![Object::number(tracker.rise)]));
        }
        out.push(Placed {
            glyphs: &segment.glyphs,
            before,
            after,
            size,
            offset,
        });
        offset += segment.glyphs.len();
    }
    Ok(out)
}

/// A line of a text object to move, in user-space units along and across its baseline.
///
/// Added in 0.7.0.
#[derive(Debug, Clone, Copy, PartialEq)]
#[non_exhaustive]
pub struct LineShift {
    /// The first show operator on the line, counted among the object's.
    pub first_show: usize,
    /// The last show operator on the line.
    pub last_show: usize,
    /// How far to move it along its baseline, in the writing direction (user space).
    pub along: f64,
    /// How far to move it up, across its baseline (user space; negative moves it down).
    pub up: f64,
    /// The size its glyphs are drawn at in user space, which gives text space's scale.
    pub size: f64,
}

impl LineShift {
    /// Moving the line made by show operators `first_show` to `last_show`, whose glyphs are drawn
    /// at `size`, `along` and `up`.
    ///
    /// Added in 0.7.0.
    #[must_use]
    pub const fn new(first_show: usize, last_show: usize, size: f64, along: f64, up: f64) -> Self {
        Self {
            first_show,
            last_show,
            along,
            up,
            size,
        }
    }
}

impl ContentOps {
    /// Moves lines of the text object: before a line's first show operator a `Td` moves it, and
    /// after its last a `Td` moves back, so what follows the line starts where it did (lines
    /// below stay where they were). The moves are in text space, scaled from user space by the
    /// glyphs' size over the font size the object sets, so a text matrix that scales or a font
    /// size of 1 is allowed. `up` is along text space's y axis: a text matrix that mirrors text
    /// turns it the other way, which the caller allows for (or refuses such text).
    ///
    /// # Errors
    /// [`NotEditable::ClippingPath`] for text that clips; [`NotEditable::ContentUnreadable`]
    /// when the object isn't recorded, a line's show operators aren't in it, its font size is
    /// zero, or a line starts, or the text after a line draws, without a line move of its own
    /// (a `Td` restarts from the line's start, not from where the pen is).
    ///
    /// Added in 0.7.0.
    pub fn shift_lines(&mut self, id: ContentObjectId, lines: &[LineShift]) -> Result<(), NotEditable> {
        let ops = self.own_ops(id)?;
        let text = self.state_of(id)?.text;
        let mut spacing = Spacing {
            size: text.font_size,
            character: text.char_spacing,
            word: text.word_spacing,
            scaling: text.horizontal_scaling,
        };
        let mut moves: Vec<Option<(f64, f64)>> = vec![None; lines.len()];
        let mut shown = 0;
        // Whether a show has drawn since the last line move, so the pen isn't at the line's start.
        let mut dirty = false;
        // Whether a move back put the line start where the pen wasn't.
        let mut broken = false;
        let mut out: Vec<ContentOp> = Vec::with_capacity(ops.len() + 2 * lines.len());
        for original in ops {
            spacing.update(&original.keyword, &original.operands);
            if matches!(&original.keyword[..], b"Td" | b"TD" | b"T*" | b"Tm" | b"'" | b"\"") {
                dirty = false;
                broken = false;
            }
            if !shows(&original.keyword) {
                out.push(original);
                continue;
            }
            if broken || (dirty && lines.iter().any(|l| l.first_show == shown)) {
                return Err(NotEditable::ContentUnreadable);
            }
            dirty = true;
            for (line, slot) in lines.iter().zip(moves.iter_mut()) {
                if line.first_show == shown {
                    if spacing.size.abs() <= f64::EPSILON || line.size.abs() <= f64::EPSILON {
                        return Err(NotEditable::ContentUnreadable);
                    }
                    let scale = line.size / spacing.size;
                    let by = (line.along / scale, line.up / scale);
                    *slot = Some(by);
                    out.push(op(b"Td", vec![Object::number(by.0), Object::number(by.1)]));
                }
            }
            out.push(original);
            for (line, slot) in lines.iter().zip(moves.iter()) {
                if line.last_show == shown
                    && let Some((x, y)) = slot
                {
                    out.push(op(b"Td", vec![Object::number(-x), Object::number(-y)]));
                    broken = true;
                }
            }
            shown += 1;
        }
        if moves.iter().any(Option::is_none) {
            return Err(NotEditable::ContentUnreadable);
        }
        self.replace(id, out);
        Ok(())
    }

    /// Puts `extra` after the object's operators: for something drawn with the text, such as an
    /// underline, which can't be inside the text object itself.
    ///
    /// # Errors
    /// As [`ContentOps::shift_lines`].
    ///
    /// Added in 0.7.0.
    pub fn append_to_object(&mut self, id: ContentObjectId, extra: Vec<ContentOp>) -> Result<(), NotEditable> {
        let mut ops = self.own_ops(id)?;
        ops.extend(extra);
        self.replace(id, ops);
        Ok(())
    }
}
