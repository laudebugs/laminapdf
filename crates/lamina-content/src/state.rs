//! Graphics and text state (ISO 32000-2 §8.4, §9.3) and the matrix arithmetic of §8.3.

use std::sync::Arc;

use lamina_syntax::{Name, ObjRef, Object};

/// An affine transform `[a b c d e f]` (§8.3.3), applied to row vectors:
/// `x' = a·x + c·y + e`, `y' = b·x + d·y + f`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Matrix {
    /// Row 1, column 1.
    pub a: f64,
    /// Row 1, column 2.
    pub b: f64,
    /// Row 2, column 1.
    pub c: f64,
    /// Row 2, column 2.
    pub d: f64,
    /// Translation x.
    pub e: f64,
    /// Translation y.
    pub f: f64,
}

impl Default for Matrix {
    fn default() -> Self {
        Self::IDENTITY
    }
}

impl Matrix {
    /// The identity matrix.
    pub const IDENTITY: Self = Self::new(1.0, 0.0, 0.0, 1.0, 0.0, 0.0);

    /// The matrix `[a b c d e f]`.
    #[must_use]
    #[expect(
        clippy::many_single_char_names,
        reason = "the spec names the six entries a to f (§8.3.3)"
    )]
    pub const fn new(a: f64, b: f64, c: f64, d: f64, e: f64, f: f64) -> Self {
        Self { a, b, c, d, e, f }
    }

    /// A translation by `(tx, ty)`.
    #[must_use]
    pub const fn translate(tx: f64, ty: f64) -> Self {
        Self::new(1.0, 0.0, 0.0, 1.0, tx, ty)
    }

    /// `self × other`: a point is transformed by `self` first, then by `other`. `cm` sets
    /// `CTM′ = M × CTM` (§8.4.4), so it is `m.multiply(&ctm)`.
    #[must_use]
    pub fn multiply(&self, other: &Self) -> Self {
        Self {
            a: self.a * other.a + self.b * other.c,
            b: self.a * other.b + self.b * other.d,
            c: self.c * other.a + self.d * other.c,
            d: self.c * other.b + self.d * other.d,
            e: self.e * other.a + self.f * other.c + other.e,
            f: self.e * other.b + self.f * other.d + other.f,
        }
    }

    /// The image of the point `(x, y)`.
    #[must_use]
    pub fn transform_point(&self, x: f64, y: f64) -> (f64, f64) {
        (x * self.a + y * self.c + self.e, x * self.b + y * self.d + self.f)
    }

    /// The matrix from six numbers (`None` unless there are exactly six).
    #[must_use]
    #[expect(
        clippy::many_single_char_names,
        reason = "the spec names the six entries a to f (§8.3.3)"
    )]
    pub fn from_slice(values: &[f64]) -> Option<Self> {
        match *values {
            [a, b, c, d, e, f] => Some(Self::new(a, b, c, d, e, f)),
            _ => None,
        }
    }

    /// The matrix in a PDF array of six numbers (a `/Matrix` entry), if it is one.
    #[must_use]
    pub fn from_object(object: &Object) -> Option<Self> {
        let values: Option<Vec<f64>> = object.as_array()?.iter().map(Object::as_f64).collect();
        Self::from_slice(&values?)
    }
}

/// A font selected by `Tf` (by resource name) or by an `ExtGState`'s `/Font` entry. Glyph
/// decoding and metrics are `lamina-text`'s job; this is where to find the font.
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
pub struct Color {
    /// The colour space.
    pub space: ColorSpace,
    /// The components, as given (the initial colour of a device space is black; a named
    /// space starts with none until `SC`/`SCN` sets them).
    pub components: Vec<f64>,
    /// The pattern name set by `SCN`/`scn` in a `Pattern` space.
    pub pattern: Option<Name>,
}

impl Default for Color {
    fn default() -> Self {
        Self::initial(ColorSpace::DeviceGray)
    }
}

impl Color {
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
}

/// A line dash pattern (§8.4.3.6).
#[derive(Debug, Clone, PartialEq, Default)]
#[non_exhaustive]
pub struct Dash {
    /// Dash and gap lengths (empty: a solid line).
    pub array: Vec<f64>,
    /// The phase.
    pub phase: f64,
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
        Matrix::new(
            self.font_size * self.horizontal_scaling,
            0.0,
            0.0,
            self.font_size,
            0.0,
            self.rise,
        )
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
/// Saved by `q`, restored by `Q`. Parameters the interpreter does not track (soft masks,
/// blend modes, overprint, transfer functions, halftones) are left to the `ExtGState`
/// dictionary itself.
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
    pub stroke: Color,
    /// The nonstroking colour.
    pub fill: Color,
    /// `/CA`: stroking alpha.
    pub stroke_alpha: f64,
    /// `/ca`: nonstroking alpha.
    pub fill_alpha: f64,
    /// The text state.
    pub text: TextState,
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
            stroke: Color::default(),
            fill: Color::default(),
            stroke_alpha: 1.0,
            fill_alpha: 1.0,
            text: TextState::default(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn multiply_applies_the_left_matrix_first() {
        let scale = Matrix::new(2.0, 0.0, 0.0, 2.0, 0.0, 0.0);
        let shift = Matrix::translate(10.0, 0.0);
        // Scale, then translate: (1, 1) → (2, 2) → (12, 2).
        assert_eq!(scale.multiply(&shift).transform_point(1.0, 1.0), (12.0, 2.0));
    }

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
