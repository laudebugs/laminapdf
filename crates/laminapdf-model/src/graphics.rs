//! The graphics values that building and reading share: [`Matrix`] (ISO 32000-2 §8.3.3),
//! [`FillRule`] (§8.5.3.3), [`Dash`] (§8.4.3.6) and [`BlendMode`] (§11.3.5). They hold numbers in whatever space they
//! are used in: page space when building, user space when reading content.

/// An affine transformation `[a b c d e f]` (§8.3.3): a point `(x, y)` maps to
/// `(a·x + c·y + e, b·x + d·y + f)`.
///
/// Composition has two names, one per order: `p.then(&q)` applies `p` first, `q.after(&p)`
/// is the same matrix. `cm` sets `CTM′ = M × CTM` (§8.4.4), which is `m.then(&ctm)`.
///
/// ```
/// use laminapdf_model::Matrix;
///
/// let scale = Matrix::scale(2.0, 2.0);
/// let shift = Matrix::translate(10.0, 0.0);
/// assert_eq!(scale.then(&shift).apply(1.0, 1.0), (12.0, 2.0));
/// assert_eq!(shift.after(&scale), scale.then(&shift));
/// ```
///
/// Added in 0.2.0.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Matrix {
    /// `a`.
    pub a: f64,
    /// `b`.
    pub b: f64,
    /// `c`.
    pub c: f64,
    /// `d`.
    pub d: f64,
    /// `e`, the x translation.
    pub e: f64,
    /// `f`, the y translation.
    pub f: f64,
}

impl Default for Matrix {
    fn default() -> Self {
        Self::IDENTITY
    }
}

impl Matrix {
    /// The identity.
    ///
    /// Added in 0.2.0.
    pub const IDENTITY: Self = Self::new([1.0, 0.0, 0.0, 1.0, 0.0, 0.0]);

    /// The matrix `[a b c d e f]`.
    ///
    /// Added in 0.2.0.
    #[must_use]
    #[expect(clippy::many_single_char_names, reason = "a–f are the matrix entries of §8.3.3")]
    pub const fn new([a, b, c, d, e, f]: [f64; 6]) -> Self {
        Self { a, b, c, d, e, f }
    }

    /// `[a b c d e f]`.
    ///
    /// Added in 0.2.0.
    #[must_use]
    pub const fn to_array(self) -> [f64; 6] {
        [self.a, self.b, self.c, self.d, self.e, self.f]
    }

    /// A translation by `(x, y)`.
    ///
    /// Added in 0.2.0.
    #[must_use]
    pub const fn translate(x: f64, y: f64) -> Self {
        Self::new([1.0, 0.0, 0.0, 1.0, x, y])
    }

    /// A scaling by `sx` horizontally and `sy` vertically.
    ///
    /// Added in 0.2.0.
    #[must_use]
    pub const fn scale(sx: f64, sy: f64) -> Self {
        Self::new([sx, 0.0, 0.0, sy, 0.0, 0.0])
    }

    /// A rotation by `degrees` about the origin, from the x axis towards the y axis: clockwise
    /// as displayed in page space (y down), counter-clockwise in user space (y up).
    ///
    /// Added in 0.2.0.
    #[must_use]
    pub fn rotate(degrees: f64) -> Self {
        let (s, c) = degrees.to_radians().sin_cos();
        Self::new([c, s, -s, c, 0.0, 0.0])
    }

    /// `self` first, then `next`: the matrix that maps `p` to `next(self(p))`, the product
    /// `self × next` of §8.3.4.
    ///
    /// Added in 0.2.0.
    #[must_use]
    pub fn then(&self, next: &Self) -> Self {
        Self::new([
            self.a * next.a + self.b * next.c,
            self.a * next.b + self.b * next.d,
            self.c * next.a + self.d * next.c,
            self.c * next.b + self.d * next.d,
            self.e * next.a + self.f * next.c + next.e,
            self.e * next.b + self.f * next.d + next.f,
        ])
    }

    /// `self` after `first`: the matrix that maps `p` to `self(first(p))`, the same as
    /// `first.then(self)`.
    ///
    /// Added in 0.2.0.
    #[must_use]
    pub fn after(&self, first: &Self) -> Self {
        first.then(self)
    }

    /// The inverse, `None` for a singular matrix.
    ///
    /// Added in 0.2.0.
    #[must_use]
    pub fn invert(&self) -> Option<Self> {
        let det = self.a * self.d - self.b * self.c;
        if det == 0.0 || !det.is_finite() {
            return None;
        }
        let (a, b, c, d) = (self.d / det, -self.b / det, -self.c / det, self.a / det);
        Some(Self::new([
            a,
            b,
            c,
            d,
            -(a * self.e + c * self.f),
            -(b * self.e + d * self.f),
        ]))
    }

    /// The image of the point `(x, y)`.
    ///
    /// Added in 0.2.0.
    #[must_use]
    pub fn apply(&self, x: f64, y: f64) -> (f64, f64) {
        (self.a * x + self.c * y + self.e, self.b * x + self.d * y + self.f)
    }

    /// The matrix from six numbers (`None` unless there are exactly six).
    ///
    /// Added in 0.2.0.
    #[must_use]
    pub fn from_slice(values: &[f64]) -> Option<Self> {
        <[f64; 6]>::try_from(values).ok().map(Self::new)
    }
}

/// How a fill or clip decides what is inside a path (§8.5.3.3).
///
/// Added in 0.2.0.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum FillRule {
    /// Non-zero winding number (`f`, `W`).
    #[default]
    NonZero,
    /// Even-odd (`f*`, `W*`).
    EvenOdd,
}

impl FillRule {
    /// Every fill rule, in declaration order.
    ///
    /// Added in 0.2.0.
    pub const ALL: [Self; 2] = [Self::NonZero, Self::EvenOdd];
}

/// A line dash pattern (§8.4.3.6): alternating dash and gap lengths, and how far into the
/// pattern the line starts. Lengths are in the space the line is drawn in: points when
/// building (page space), user-space units when reading content.
///
/// Added in 0.2.0.
#[derive(Debug, Clone, PartialEq, Default)]
#[non_exhaustive]
pub struct Dash {
    /// Dash and gap lengths; an empty list draws a solid line.
    pub array: Vec<f64>,
    /// Where in the pattern the line starts.
    pub phase: f64,
}

impl Dash {
    /// A pattern of `array` lengths starting `phase` into it.
    ///
    /// Added in 0.2.0.
    #[must_use]
    pub const fn new(array: Vec<f64>, phase: f64) -> Self {
        Self { array, phase }
    }
}

/// A blend mode (§11.3.5, Tables 134 and 135).
///
/// Added in 0.2.0.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Default)]
#[non_exhaustive]
pub enum BlendMode {
    /// `Normal`.
    #[default]
    Normal,
    /// `Multiply`.
    Multiply,
    /// `Screen`.
    Screen,
    /// `Overlay`.
    Overlay,
    /// `Darken`.
    Darken,
    /// `Lighten`.
    Lighten,
    /// `ColorDodge`.
    ColorDodge,
    /// `ColorBurn`.
    ColorBurn,
    /// `HardLight`.
    HardLight,
    /// `SoftLight`.
    SoftLight,
    /// `Difference`.
    Difference,
    /// `Exclusion`.
    Exclusion,
    /// `Hue`.
    Hue,
    /// `Saturation`.
    Saturation,
    /// `Color`.
    Color,
    /// `Luminosity`.
    Luminosity,
}

impl BlendMode {
    /// Every blend mode, in declaration order.
    ///
    /// Added in 0.2.0.
    pub const ALL: [Self; 16] = [
        Self::Normal,
        Self::Multiply,
        Self::Screen,
        Self::Overlay,
        Self::Darken,
        Self::Lighten,
        Self::ColorDodge,
        Self::ColorBurn,
        Self::HardLight,
        Self::SoftLight,
        Self::Difference,
        Self::Exclusion,
        Self::Hue,
        Self::Saturation,
        Self::Color,
        Self::Luminosity,
    ];

    /// The PDF name (`/BM`).
    ///
    /// Added in 0.2.0.
    #[must_use]
    pub const fn pdf_name(self) -> &'static str {
        match self {
            Self::Normal => "Normal",
            Self::Multiply => "Multiply",
            Self::Screen => "Screen",
            Self::Overlay => "Overlay",
            Self::Darken => "Darken",
            Self::Lighten => "Lighten",
            Self::ColorDodge => "ColorDodge",
            Self::ColorBurn => "ColorBurn",
            Self::HardLight => "HardLight",
            Self::SoftLight => "SoftLight",
            Self::Difference => "Difference",
            Self::Exclusion => "Exclusion",
            Self::Hue => "Hue",
            Self::Saturation => "Saturation",
            Self::Color => "Color",
            Self::Luminosity => "Luminosity",
        }
    }
    /// The mode a `/BM` name names, `Compatible` read as `Normal` (§11.3.5), or `None` for a
    /// name §11.3.5 doesn't define.
    ///
    /// Added in 0.2.0.
    #[must_use]
    pub fn from_pdf_name(name: &[u8]) -> Option<Self> {
        if name == b"Compatible" {
            return Some(Self::Normal);
        }
        Self::ALL.into_iter().find(|mode| mode.pdf_name().as_bytes() == name)
    }
}

#[cfg(test)]
mod tests {
    use super::{BlendMode, FillRule, Matrix};

    const SCALE: Matrix = Matrix::scale(2.0, 3.0);
    const SHIFT: Matrix = Matrix::translate(10.0, 20.0);

    #[test]
    fn then_should_apply_the_receiver_first() {
        // Scale (1, 1) to (2, 3), then shift it to (12, 23).
        assert_eq!(SCALE.then(&SHIFT).apply(1.0, 1.0), (12.0, 23.0));
        // Shift (1, 1) to (11, 21), then scale it to (22, 63).
        assert_eq!(SHIFT.then(&SCALE).apply(1.0, 1.0), (22.0, 63.0));
    }

    #[test]
    fn after_should_apply_its_argument_first() {
        assert_eq!(SHIFT.after(&SCALE).apply(1.0, 1.0), (12.0, 23.0));
        assert_eq!(SHIFT.after(&SCALE), SCALE.then(&SHIFT));
    }

    #[test]
    fn then_should_be_the_spec_product_cm_uses() {
        // §8.4.4: `cm` sets CTM′ = M × CTM; with CTM a translation, M's scale applies first.
        let m = Matrix::new([2.0, 0.0, 0.0, 2.0, 0.0, 0.0]);
        let ctm = Matrix::new([1.0, 0.0, 0.0, 1.0, 5.0, 7.0]);
        assert_eq!(m.then(&ctm).to_array(), [2.0, 0.0, 0.0, 2.0, 5.0, 7.0]);
    }

    #[test]
    fn invert_should_undo_the_matrix_and_refuse_a_singular_one() {
        let m = Matrix::rotate(30.0).then(&SCALE).then(&SHIFT);
        let back = m.invert().map(|i| {
            let (x, y) = m.apply(4.0, -2.0);
            i.apply(x, y)
        });
        let Some((x, y)) = back else {
            panic!("a rotation, scale and shift is invertible");
        };
        assert!((x - 4.0).abs() < 1e-9 && (y + 2.0).abs() < 1e-9);
        assert_eq!(Matrix::scale(0.0, 1.0).invert(), None);
    }

    #[test]
    fn rotate_should_turn_the_x_axis_towards_the_y_axis() {
        let (x, y) = Matrix::rotate(90.0).apply(1.0, 0.0);
        assert!(x.abs() < 1e-12 && (y - 1.0).abs() < 1e-12);
    }

    #[test]
    fn from_slice_should_take_exactly_six_numbers() {
        assert_eq!(
            Matrix::from_slice(&[1.0, 0.0, 0.0, 1.0, 0.0, 0.0]),
            Some(Matrix::IDENTITY)
        );
        assert_eq!(Matrix::from_slice(&[1.0; 5]), None);
        assert_eq!(Matrix::from_slice(&[1.0; 7]), None);
    }

    /// The match is exhaustive, so a new variant fails to build here first: give it the next
    /// index and add it to `ALL` there, which this test then checks.
    #[test]
    fn all_should_list_every_fill_rule_once_in_declaration_order() {
        let index = |v: FillRule| match v {
            FillRule::NonZero => 0,
            FillRule::EvenOdd => 1,
        };
        assert!(FillRule::ALL.iter().enumerate().all(|(i, v)| index(*v) == i));
    }

    /// The match is exhaustive, so a new variant fails to build here first: give it the next
    /// index and add it to `ALL` there, which this test then checks.
    #[test]
    fn all_should_list_every_blend_mode_once_in_declaration_order() {
        let index = |v: BlendMode| match v {
            BlendMode::Normal => 0,
            BlendMode::Multiply => 1,
            BlendMode::Screen => 2,
            BlendMode::Overlay => 3,
            BlendMode::Darken => 4,
            BlendMode::Lighten => 5,
            BlendMode::ColorDodge => 6,
            BlendMode::ColorBurn => 7,
            BlendMode::HardLight => 8,
            BlendMode::SoftLight => 9,
            BlendMode::Difference => 10,
            BlendMode::Exclusion => 11,
            BlendMode::Hue => 12,
            BlendMode::Saturation => 13,
            BlendMode::Color => 14,
            BlendMode::Luminosity => 15,
        };
        assert_eq!(
            BlendMode::ALL.map(index),
            [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15]
        );
    }

    #[test]
    fn from_pdf_name_should_read_every_mode_and_compatible_as_normal() {
        let read = BlendMode::ALL.map(|m| BlendMode::from_pdf_name(m.pdf_name().as_bytes()));
        assert_eq!(read, BlendMode::ALL.map(Some));
        assert_eq!(BlendMode::from_pdf_name(b"Compatible"), Some(BlendMode::Normal));
        assert_eq!(BlendMode::from_pdf_name(b"Sparkle"), None);
    }
}
