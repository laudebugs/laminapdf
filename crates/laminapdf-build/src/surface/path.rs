//! Paths (ISO 32000-2 §8.5.2) and transformation matrices (§8.3.4), in page space.

use laminapdf_model::{PointPt, Pt, RectPt};

/// The circle-approximation constant κ = 4(√2 − 1)/3: four cubic Béziers with control points
/// at κ·r approximate a circle to within 0.03 %.
pub(crate) const KAPPA: f64 = 4.0 * (std::f64::consts::SQRT_2 - 1.0) / 3.0;

/// How a fill or clip decides what is inside a path (§8.5.3.3).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[non_exhaustive]
pub enum FillRule {
    /// Non-zero winding number (`f`, `W`).
    #[default]
    NonZero,
    /// Even-odd (`f*`, `W*`).
    EvenOdd,
}

/// One path segment, in page space.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) enum Segment {
    MoveTo(PointPt),
    LineTo(PointPt),
    CubicTo(PointPt, PointPt, PointPt),
    Close,
    Rect(RectPt),
}

/// Builds a path in page space (points from the top-left of the page as displayed, y down).
/// Quadratic curves become cubic ones; rectangles stay `re`; ellipses are four cubics. A
/// segment drawn without a current point (a `line_to`, curve or `close` first, as PDF
/// forbids) starts a subpath where the segment starts instead: `line_to(p)` moves to `p`,
/// curves move to their first control point, and `close` is dropped.
///
/// ```
/// use laminapdf_build::PathBuilder;
/// use laminapdf_model::{PointPt, Pt, RectPt};
///
/// let mut p = PathBuilder::default();
/// p.move_to(PointPt::from_points(10.0, 10.0))
///     .line_to(PointPt::from_points(100.0, 10.0))
///     .quad_to(PointPt::from_points(120.0, 50.0), PointPt::from_points(100.0, 90.0))
///     .close()
///     .ellipse(RectPt::from_xywh(Pt::new(200.0), Pt::new(10.0), Pt::new(80.0), Pt::new(40.0)));
/// assert!(!p.is_empty());
/// ```
#[derive(Debug, Clone, Default, PartialEq)]
pub struct PathBuilder {
    segments: Vec<Segment>,
    /// The current point, for quadratic curves.
    current: Option<PointPt>,
    /// Where the current subpath started (`close` returns there).
    start: Option<PointPt>,
}

fn lerp(a: PointPt, b: PointPt, t: f64) -> PointPt {
    PointPt::new(a.x + (b.x - a.x) * t, a.y + (b.y - a.y) * t)
}

impl PathBuilder {
    /// Whether the path has no segments.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.segments.is_empty()
    }

    /// Starts a new subpath at `p` (`m`).
    pub fn move_to(&mut self, p: PointPt) -> &mut Self {
        self.segments.push(Segment::MoveTo(p));
        self.current = Some(p);
        self.start = Some(p);
        self
    }

    /// Starts a subpath at `p` when there is no current point.
    fn ensure_current(&mut self, p: PointPt) {
        if self.current.is_none() {
            self.move_to(p);
        }
    }

    /// A straight line to `p` (`l`).
    pub fn line_to(&mut self, p: PointPt) -> &mut Self {
        if self.current.is_none() {
            return self.move_to(p);
        }
        self.segments.push(Segment::LineTo(p));
        self.current = Some(p);
        self
    }

    /// A cubic Bézier curve to `p` with control points `c1` and `c2` (`c`).
    pub fn cubic_to(&mut self, c1: PointPt, c2: PointPt, p: PointPt) -> &mut Self {
        self.ensure_current(c1);
        self.segments.push(Segment::CubicTo(c1, c2, p));
        self.current = Some(p);
        self
    }

    /// A quadratic Bézier curve to `p` with control point `c`, written as the equivalent cubic
    /// (PDF has no quadratic operator). Without a current point it starts at `c`.
    pub fn quad_to(&mut self, c: PointPt, p: PointPt) -> &mut Self {
        self.ensure_current(c);
        let from = self.current.unwrap_or(c);
        self.cubic_to(lerp(from, c, 2.0 / 3.0), lerp(p, c, 2.0 / 3.0), p)
    }

    /// Closes the current subpath with a straight line to its start (`h`).
    pub fn close(&mut self) -> &mut Self {
        if self.current.is_none() {
            return self;
        }
        self.segments.push(Segment::Close);
        self.current = self.start;
        self
    }

    /// A rectangle as a closed subpath of its own (`re`).
    pub fn rect(&mut self, r: RectPt) -> &mut Self {
        self.segments.push(Segment::Rect(r));
        let corner = PointPt::new(r.x0, r.y0);
        self.current = Some(corner);
        self.start = Some(corner);
        self
    }

    /// The ellipse inscribed in `r`, as a closed subpath of four cubic curves.
    pub fn ellipse(&mut self, r: RectPt) -> &mut Self {
        let (rx, ry) = (r.width() / 2.0, r.height() / 2.0);
        let (cx, cy) = (r.x0 + rx, r.y0 + ry);
        let (kx, ky) = (rx * KAPPA, ry * KAPPA);
        let p = PointPt::new;
        self.move_to(p(cx + rx, cy))
            .cubic_to(p(cx + rx, cy + ky), p(cx + kx, cy + ry), p(cx, cy + ry))
            .cubic_to(p(cx - kx, cy + ry), p(cx - rx, cy + ky), p(cx - rx, cy))
            .cubic_to(p(cx - rx, cy - ky), p(cx - kx, cy - ry), p(cx, cy - ry))
            .cubic_to(p(cx + kx, cy - ry), p(cx + rx, cy - ky), p(cx + rx, cy))
            .close()
    }

    /// The circle of radius `radius` around `center`.
    pub fn circle(&mut self, center: PointPt, radius: Pt) -> &mut Self {
        self.ellipse(RectPt::new(
            center.x - radius,
            center.y - radius,
            center.x + radius,
            center.y + radius,
        ))
    }

    /// Whether every coordinate is finite.
    pub(crate) fn is_finite(&self) -> bool {
        let ok = |p: &PointPt| p.x.get().is_finite() && p.y.get().is_finite();
        self.segments.iter().all(|s| match s {
            Segment::MoveTo(p) | Segment::LineTo(p) => ok(p),
            Segment::CubicTo(a, b, c) => ok(a) && ok(b) && ok(c),
            Segment::Close => true,
            Segment::Rect(r) => r.is_finite(),
        })
    }

    pub(crate) fn take(&mut self) -> Vec<Segment> {
        self.current = None;
        self.start = None;
        std::mem::take(&mut self.segments)
    }
}

/// An affine transformation in page space (§8.3.4): a point `(x, y)` maps to
/// `(a·x + c·y + e, b·x + d·y + f)`, with `e` and `f` in points. Because page space has y
/// down, [`Matrix::rotate`] with a positive angle turns clockwise on the displayed page.
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
    /// `e`, in points.
    pub e: f64,
    /// `f`, in points.
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
    #[expect(clippy::many_single_char_names, reason = "a–f are the matrix entries of §8.3.4")]
    pub const fn new([a, b, c, d, e, f]: [f64; 6]) -> Self {
        Self { a, b, c, d, e, f }
    }

    /// A translation by `(x, y)`.
    #[must_use]
    pub const fn translate(x: Pt, y: Pt) -> Self {
        Self::new([1.0, 0.0, 0.0, 1.0, x.get(), y.get()])
    }

    /// A scaling by `sx` horizontally and `sy` vertically.
    #[must_use]
    pub const fn scale(sx: f64, sy: f64) -> Self {
        Self::new([sx, 0.0, 0.0, sy, 0.0, 0.0])
    }

    /// A rotation by `degrees` about the origin, clockwise as displayed.
    #[must_use]
    pub fn rotate(degrees: f64) -> Self {
        let (s, c) = degrees.to_radians().sin_cos();
        Self::new([c, s, -s, c, 0.0, 0.0])
    }

    /// `self` applied after `inner`: the transformation that maps `p` to
    /// `self(inner(p))`.
    #[must_use]
    pub fn after(&self, inner: &Self) -> Self {
        Self::new([
            self.a * inner.a + self.c * inner.b,
            self.b * inner.a + self.d * inner.b,
            self.a * inner.c + self.c * inner.d,
            self.b * inner.c + self.d * inner.d,
            self.a * inner.e + self.c * inner.f + self.e,
            self.b * inner.e + self.d * inner.f + self.f,
        ])
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

    /// `(x, y)` transformed.
    #[must_use]
    pub fn apply(&self, x: f64, y: f64) -> (f64, f64) {
        (self.a * x + self.c * y + self.e, self.b * x + self.d * y + self.f)
    }

    pub(crate) const fn to_array(self) -> [f64; 6] {
        [self.a, self.b, self.c, self.d, self.e, self.f]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pt(x: f64, y: f64) -> PointPt {
        PointPt::from_points(x, y)
    }

    #[test]
    fn quad_to_becomes_the_equivalent_cubic() {
        let mut p = PathBuilder::default();
        p.move_to(pt(0.0, 0.0)).quad_to(pt(30.0, 30.0), pt(60.0, 0.0));
        assert_eq!(
            p.take()[1],
            Segment::CubicTo(pt(20.0, 20.0), pt(40.0, 20.0), pt(60.0, 0.0))
        );
    }

    #[test]
    fn ellipse_is_a_closed_subpath_of_four_cubics_through_the_axis_ends() {
        let mut p = PathBuilder::default();
        p.ellipse(RectPt::from_xywh(
            Pt::new(0.0),
            Pt::new(0.0),
            Pt::new(20.0),
            Pt::new(10.0),
        ));
        let segments = p.take();
        let ends: Vec<_> = segments
            .iter()
            .filter_map(|s| match s {
                Segment::CubicTo(_, _, e) => Some((e.x.get(), e.y.get())),
                _ => None,
            })
            .collect();
        assert_eq!(
            (ends, segments.last()),
            (
                vec![(10.0, 10.0), (0.0, 5.0), (10.0, 0.0), (20.0, 5.0)],
                Some(&Segment::Close)
            )
        );
    }

    #[test]
    fn a_line_without_a_current_point_starts_a_subpath_there() {
        let mut p = PathBuilder::default();
        p.line_to(pt(1.0, 2.0)).line_to(pt(3.0, 4.0));
        assert_eq!(p.take(), [Segment::MoveTo(pt(1.0, 2.0)), Segment::LineTo(pt(3.0, 4.0))]);
    }

    #[test]
    fn a_curve_without_a_current_point_moves_to_its_first_control_point() {
        let mut p = PathBuilder::default();
        p.quad_to(pt(30.0, 30.0), pt(60.0, 0.0));
        let segments = p.take();
        assert_eq!(segments[0], Segment::MoveTo(pt(30.0, 30.0)));
    }

    #[test]
    fn close_without_a_current_point_is_dropped() {
        let mut p = PathBuilder::default();
        p.close();
        assert!(p.is_empty());
    }

    #[test]
    fn a_nan_coordinate_is_reported() {
        let mut p = PathBuilder::default();
        p.move_to(pt(f64::NAN, 0.0));
        assert!(!p.is_finite());
    }

    #[test]
    fn close_returns_the_current_point_to_the_subpath_start() {
        let mut p = PathBuilder::default();
        p.move_to(pt(5.0, 5.0)).line_to(pt(9.0, 9.0)).close();
        assert_eq!(p.current, Some(pt(5.0, 5.0)));
    }

    #[test]
    fn after_composes_inner_first() {
        let m = Matrix::translate(Pt::new(10.0), Pt::ZERO).after(&Matrix::scale(2.0, 2.0));
        assert_eq!(m.apply(1.0, 1.0), (12.0, 2.0));
    }

    #[test]
    fn invert_undoes_the_matrix() {
        let m = Matrix::new([0.0, 2.0, -2.0, 0.0, 5.0, 7.0]);
        let inv = m.invert().unwrap();
        assert_eq!(inv.after(&m).apply(3.0, 4.0), (3.0, 4.0));
    }

    #[test]
    fn a_singular_matrix_has_no_inverse() {
        assert_eq!(Matrix::scale(0.0, 1.0).invert(), None);
    }

    #[test]
    fn rotate_90_turns_x_into_y_down() {
        let (x, y) = Matrix::rotate(90.0).apply(1.0, 0.0);
        assert!((x.abs() < 1e-12) && (y - 1.0).abs() < 1e-12);
    }
}
