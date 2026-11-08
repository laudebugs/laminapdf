//! Graphics and text state (ISO 32000-2 §8.4, §9.3) and the matrix arithmetic of §8.3.

use std::sync::Arc;

use laminapdf_model::{BlendMode, Dash, Matrix, UserSpaceRect};
use laminapdf_syntax::{Name, ObjRef, Object};

/// A font selected by `Tf` (by resource name) or by an `ExtGState`'s `/Font` entry. Glyph
/// decoding and metrics are `laminapdf-text`'s job; this is where to find the font.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct FontRef {
    /// The `/Font` resource name (`None` when set through an `ExtGState`).
    pub name: Option<Name>,
    /// The font dictionary's object, when it is indirect (a cache key across pages).
    pub object: Option<ObjRef>,
    /// The font dictionary, resolved; `None` when the resource is missing or not a dictionary
    /// (`content-resource-missing`).
    pub dict: Option<Arc<Object>>,
}

/// A colour space as selected by `CS`/`cs` or implied by `G`/`RG`/`K` (§8.6).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum ColorSpace {
    /// `DeviceGray` (the initial space).
    DeviceGray,
    /// `DeviceRGB`.
    DeviceRgb,
    /// `DeviceCMYK`.
    DeviceCmyk,
    /// `Pattern` (§8.7.3).
    Pattern,
    /// A space from the `/ColorSpace` resources, by name (not interpreted here).
    Named(Name),
}

/// A current colour: its space, components and pattern (§8.6.8).
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct CurrentColor {
    /// The colour space.
    pub space: ColorSpace,
    /// The components, as given (the initial colour of a device space is black; a named
    /// space starts with none until `SC`/`SCN` sets them).
    pub components: Vec<f64>,
    /// The pattern name set by `SCN`/`scn` in a `Pattern` space.
    pub pattern: Option<Name>,
}

impl Default for CurrentColor {
    fn default() -> Self {
        Self::initial(ColorSpace::DeviceGray)
    }
}

impl CurrentColor {
    /// The initial colour of `space` (§8.6.8 Table 73: black for device spaces).
    #[must_use]
    pub fn initial(space: ColorSpace) -> Self {
        let components = match space {
            ColorSpace::DeviceGray => vec![0.0],
            ColorSpace::DeviceRgb => vec![0.0; 3],
            ColorSpace::DeviceCmyk => vec![0.0, 0.0, 0.0, 1.0],
            ColorSpace::Pattern | ColorSpace::Named(_) => Vec::new(),
        };
        Self {
            space,
            components,
            pattern: None,
        }
    }

    /// The colour as RGB in [0, 1], approximately: the device spaces by §10.4's conversions.
    /// `None` for a pattern and for a named space: the interpreter doesn't read what it is, and
    /// its components mean different things in an `ICCBased`, `Separation` (1 is full ink),
    /// `Indexed` or `Lab` space.
    #[must_use]
    pub fn approx_rgb(&self) -> Option<[f64; 3]> {
        let c = |i: usize| self.components.get(i).copied().map(|v| v.clamp(0.0, 1.0));
        match self.space {
            ColorSpace::DeviceGray => c(0).map(|g| [g, g, g]),
            ColorSpace::DeviceRgb => Some([c(0)?, c(1)?, c(2)?]),
            ColorSpace::DeviceCmyk => {
                let k = c(3)?;
                Some([
                    (1.0 - c(0)?) * (1.0 - k),
                    (1.0 - c(1)?) * (1.0 - k),
                    (1.0 - c(2)?) * (1.0 - k),
                ])
            }
            ColorSpace::Pattern | ColorSpace::Named(_) => None,
        }
    }
}

/// The text state parameters (§9.3, Table 103). Part of the graphics state, so `q`/`Q` save
/// and restore it; `BT` does not reset it.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct TextState {
    /// `Tc`, character spacing (unscaled text space units).
    pub char_spacing: f64,
    /// `Tw`, word spacing.
    pub word_spacing: f64,
    /// `Tz` / 100: horizontal scaling as a factor (1.0 = 100 %).
    pub horizontal_scaling: f64,
    /// `TL`, leading.
    pub leading: f64,
    /// The font set by `Tf` or an `ExtGState`; `None` until one is set.
    pub font: Option<FontRef>,
    /// The font size (`Tfs`).
    pub font_size: f64,
    /// `Tr`, the text rendering mode (0–7; other values are kept as written).
    pub render_mode: i64,
    /// `Ts`, text rise.
    pub rise: f64,
}

impl Default for TextState {
    fn default() -> Self {
        Self {
            char_spacing: 0.0,
            word_spacing: 0.0,
            horizontal_scaling: 1.0,
            leading: 0.0,
            font: None,
            font_size: 0.0,
            render_mode: 0,
            rise: 0.0,
        }
    }
}

impl TextState {
    /// How far one glyph moves the text matrix (§9.4.4): `w0`/`w1` are its horizontal and
    /// vertical displacement in thousandths of text space, `tj` the `TJ` number before it
    /// (0 for `Tj`), `space` whether it is the single-byte code 32 (`Tw` applies only then).
    ///
    /// Horizontal writing: `tx = ((w0 − tj/1000) · Tfs + Tc + Tw) · Th`, `ty = 0`.
    /// Vertical writing: `tx = 0`, `ty = (w1 − tj/1000) · Tfs + Tc + Tw`.
    #[must_use]
    pub fn glyph_advance(&self, w0: f64, w1: f64, tj: f64, space: bool, vertical: bool) -> TextAdvance {
        let word = if space { self.word_spacing } else { 0.0 };
        if vertical {
            let ty = (w1 - tj / 1000.0) * self.font_size + self.char_spacing + word;
            TextAdvance { tx: 0.0, ty }
        } else {
            let tx = ((w0 - tj / 1000.0) * self.font_size + self.char_spacing + word) * self.horizontal_scaling;
            TextAdvance { tx, ty: 0.0 }
        }
    }

    /// The matrix `[Tfs·Th 0 0 Tfs 0 Trise]` that maps text space to the text rendering
    /// matrix's input (§9.4.4): `Trm = this × Tm × CTM`.
    #[must_use]
    pub fn text_space_matrix(&self) -> Matrix {
        Matrix::new([
            self.font_size * self.horizontal_scaling,
            0.0,
            0.0,
            self.font_size,
            0.0,
            self.rise,
        ])
    }
}

/// How far a text-showing operator moved the text matrix, in unscaled text space: the
/// interpreter applies `Tm = [1 0 0 1 tx ty] × Tm` (§9.4.4). A [`Visitor`](crate::Visitor)
/// that knows the glyph widths returns it from
/// [`show_text`](crate::Visitor::show_text); the default is no movement.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct TextAdvance {
    /// Horizontal displacement.
    pub tx: f64,
    /// Vertical displacement.
    pub ty: f64,
}

impl std::ops::Add for TextAdvance {
    type Output = Self;

    fn add(self, other: Self) -> Self {
        Self {
            tx: self.tx + other.tx,
            ty: self.ty + other.ty,
        }
    }
}

/// The graphics state (§8.4.1, Tables 51–52) as far as reading content needs it.
///
/// Saved by `q`, restored by `Q`. Parameters the interpreter does not track (the soft mask
/// itself, overprint, transfer functions, halftones) are left to the `ExtGState` dictionary.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct GraphicsState {
    /// The current transformation matrix (user space → default user space of the page).
    pub ctm: Matrix,
    /// `w` / `/LW`.
    pub line_width: f64,
    /// `J` / `/LC`.
    pub line_cap: i64,
    /// `j` / `/LJ`.
    pub line_join: i64,
    /// `M` / `/ML`.
    pub miter_limit: f64,
    /// `d` / `/D`.
    pub dash: Dash,
    /// `ri` / `/RI` (`None`: the default, `RelativeColorimetric`).
    pub rendering_intent: Option<Name>,
    /// `i` / `/FL`.
    pub flatness: f64,
    /// The stroking colour.
    pub stroke: CurrentColor,
    /// The nonstroking colour.
    pub fill: CurrentColor,
    /// `/CA`: stroking alpha.
    pub stroke_alpha: f64,
    /// `/ca`: nonstroking alpha.
    pub fill_alpha: f64,
    /// `/BM`: the blend mode (§11.3.5); an unknown name reads as `Normal`.
    pub blend_mode: BlendMode,
    /// Whether `/SMask` names a soft mask (anything but `/None`, §11.6.5.2). Only whether one is
    /// set: the mask itself isn't read.
    pub soft_mask: bool,
    /// The text state.
    pub text: TextState,
    /// The clip's bounding box in the page's default user space (§8.5.4): each `W`/`W*` path
    /// and each Form XObject's `/BBox` intersects it. A box, not the path, so a curved clip
    /// counts everything inside its box. `None` until something clips; an empty clip is a box
    /// of no area.
    pub clip_box: Option<UserSpaceRect>,
}

impl GraphicsState {
    /// Narrows the clip to `bounds` (in default user space).
    pub fn clip_to(&mut self, bounds: UserSpaceRect) {
        let clipped = match self.clip_box {
            Some(current) => current.intersect(&bounds),
            None => Some(bounds),
        };
        // Nothing left: a box of no area where the two last met.
        self.clip_box =
            Some(clipped.unwrap_or(UserSpaceRect::from_corners(bounds.x0, bounds.y0, bounds.x0, bounds.y0)));
    }
}

/// The bounding box of `rect` after `m`.
#[must_use]
pub fn transformed_box(rect: &UserSpaceRect, m: &Matrix) -> UserSpaceRect {
    let corners = [
        (rect.x0, rect.y0),
        (rect.x1, rect.y0),
        (rect.x0, rect.y1),
        (rect.x1, rect.y1),
    ]
    .map(|(x, y)| m.apply(x, y));
    let xs = corners.map(|p| p.0);
    let ys = corners.map(|p| p.1);
    let (x0, x1) = (
        xs.iter().copied().fold(f64::INFINITY, f64::min),
        xs.iter().copied().fold(f64::NEG_INFINITY, f64::max),
    );
    let (y0, y1) = (
        ys.iter().copied().fold(f64::INFINITY, f64::min),
        ys.iter().copied().fold(f64::NEG_INFINITY, f64::max),
    );
    UserSpaceRect::from_corners(x0, y0, x1, y1)
}

impl Default for GraphicsState {
    fn default() -> Self {
        Self {
            ctm: Matrix::IDENTITY,
            line_width: 1.0,
            line_cap: 0,
            line_join: 0,
            miter_limit: 10.0,
            dash: Dash::default(),
            rendering_intent: None,
            flatness: 1.0,
            stroke: CurrentColor::default(),
            fill: CurrentColor::default(),
            stroke_alpha: 1.0,
            fill_alpha: 1.0,
            blend_mode: BlendMode::Normal,
            soft_mask: false,
            text: TextState::default(),
            clip_box: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn horizontal_advance_follows_9_4_4() {
        let text = TextState {
            font_size: 24.0,
            char_spacing: 1.0,
            word_spacing: 2.0,
            horizontal_scaling: 0.5,
            ..TextState::default()
        };
        // ((0.5 − 100/1000) · 24 + 1 + 2) · 0.5 = (9.6 + 3) · 0.5 = 6.3
        let advance = text.glyph_advance(0.5, 0.0, 100.0, true, false);
        assert!((advance.tx - 6.3).abs() < 1e-12);
    }

    #[test]
    fn vertical_advance_ignores_horizontal_scaling() {
        let text = TextState {
            font_size: 10.0,
            horizontal_scaling: 0.5,
            ..TextState::default()
        };
        assert_eq!(
            text.glyph_advance(0.0, -1.0, 0.0, false, true),
            TextAdvance { tx: 0.0, ty: -10.0 }
        );
    }
}
