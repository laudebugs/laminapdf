//! The graphics values that building and reading share: [`Matrix`] (ISO 32000-2 §8.3.3),
//! [`FillRule`] (§8.5.3.3) and [`Dash`] (§8.4.3.6). They hold numbers in whatever space they
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
    pub const IDENTITY: Self = Self::new([1.0, 0.0, 0.0, 1.0, 0.0, 0.0]);

    /// The matrix `[a b c d e f]`.
    #[must_use]
    #[expect(clippy::many_single_char_names, reason = "a–f are the matrix entries of §8.3.3")]
    pub const fn new([a, b, c, d, e, f]: [f64; 6]) -> Self {
        Self { a, b, c, d, e, f }
    }

    /// `[a b c d e f]`.
    #[must_use]
    pub const fn to_array(self) -> [f64; 6] {
        [self.a, self.b, self.c, self.d, self.e, self.f]
    }

    /// A translation by `(x, y)`.
    #[must_use]
    pub const fn translate(x: f64, y: f64) -> Self {
        Self::new([1.0, 0.0, 0.0, 1.0, x, y])
    }

    /// A scaling by `sx` horizontally and `sy` vertically.
    #[must_use]
    pub const fn scale(sx: f64, sy: f64) -> Self {
        Self::new([sx, 0.0, 0.0, sy, 0.0, 0.0])
    }

    /// A rotation by `degrees` about the origin, from the x axis towards the y axis: clockwise
    /// as displayed in page space (y down), counter-clockwise in user space (y up).
    #[must_use]
    pub fn rotate(degrees: f64) -> Self {
        let (s, c) = degrees.to_radians().sin_cos();
        Self::new([c, s, -s, c, 0.0, 0.0])
    }

    /// `self` first, then `next`: the matrix that maps `p` to `next(self(p))`, the product
    /// `self × next` of §8.3.4.
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
    #[must_use]
    pub fn after(&self, first: &Self) -> Self {
        first.then(self)
    }

    /// The inverse, `None` for a singular matrix.
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
    #[must_use]
    pub fn apply(&self, x: f64, y: f64) -> (f64, f64) {
        (self.a * x + self.c * y + self.e, self.b * x + self.d * y + self.f)
    }

    /// The matrix from six numbers (`None` unless there are exactly six).
    #[must_use]
    pub fn from_slice(values: &[f64]) -> Option<Self> {
        <[f64; 6]>::try_from(values).ok().map(Self::new)
    }
}

/// How a fill or clip decides what is inside a path (§8.5.3.3).
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
    pub const ALL: [Self; 2] = [Self::NonZero, Self::EvenOdd];
}

/// A line dash pattern (§8.4.3.6): alternating dash and gap lengths, and how far into the
/// pattern the line starts. Lengths are in the space the line is drawn in: points when
/// building (page space), user-space units when reading content.
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
    #[must_use]
    pub const fn new(array: Vec<f64>, phase: f64) -> Self {
        Self { array, phase }
    }
}

#[cfg(test)]
mod tests {
    use super::{FillRule, Matrix};

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
}
