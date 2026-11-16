//! Appearances of the drawing marks (ISO 32000-2 §12.5.6.7–§12.5.6.13): ink, squares and
//! circles, lines with their endings and leader lines, polygons and polylines, with solid,
//! dashed or cloudy outlines and interior colour. Everything is drawn in user space, and the
//! form's box is what it draws, grown by half the stroke, so `/Rect` holds the endings and
//! clouds too.

mod cloud;
mod ending;
mod ink;
mod line;
mod shape;

use laminapdf_model::{UserSpacePoint, UserSpaceRect};
use laminapdf_ops::{AnnotColor, AnnotSpec, Border, BorderStyle};
use pdf_writer::Content;
use pdf_writer::types::{LineCapStyle, LineJoinStyle};

use super::{DeviceColor, Drawn, Paint};
use crate::surface::n;

/// The colour a drawing mark strokes in when it has no `/C`: red, as Acrobat draws them.
const DEFAULT: DeviceColor = DeviceColor::Rgb(1.0, 0.0, 0.0);

/// Bézier control-point distance for a quarter circle of radius 1.
pub(super) const KAPPA: f64 = 0.552_284_75;

/// The appearance of a drawing mark; `None` for any other spec.
pub(super) fn draw(spec: &AnnotSpec, paint: &Paint) -> Option<Drawn> {
    let mut drawn = Drawn::new(UserSpaceRect::from_corners(0.0, 0.0, 0.0, 0.0), paint.opacity, false);
    let stroke = paint.color_or(DEFAULT);
    let mut pen = Pen::new(&mut drawn.content);
    let reach = match spec {
        AnnotSpec::Ink(ink) => ink::ink(&mut pen, ink, stroke),
        AnnotSpec::Shape(s) => shape::shape(&mut pen, s, stroke, interior(paint, s.interior)),
        AnnotSpec::Line(l) => line::line(&mut pen, l, stroke, interior(paint, l.interior)),
        AnnotSpec::Poly(p) => shape::poly(&mut pen, p, stroke, interior(paint, p.interior)),
        _ => return None,
    };
    let bounds = pen.bounds?;
    drawn.bbox = bounds.grown(reach);
    Some(drawn)
}

/// A text box's frame: its fill and border along `text.rect`, and a callout's leader line with
/// its ending, all in `color`. The drawing's box is the frame's, grown by half the stroke.
#[cfg(feature = "freetext")]
pub(super) fn freetext_frame(text: &laminapdf_ops::FreeText, paint: &Paint, color: DeviceColor) -> Drawn {
    use laminapdf_ops::{FreeTextIntent, LineEnding, Poly, PolyKind, Shape, ShapeKind};
    let mut drawn = Drawn::new(text.rect, paint.opacity, false);
    let border = text.drawn_border();
    let fill = match text.intent {
        FreeTextIntent::Typewriter => None,
        _ => interior(paint, text.fill),
    };
    let mut pen = Pen::new(&mut drawn.content);
    let box_shape = Shape::new(ShapeKind::Square, text.rect).with_border(border.clone());
    let mut reach = shape::shape(&mut pen, &box_shape, color, fill);
    if let FreeTextIntent::Callout(callout) = &text.intent {
        // The leader line keeps a visible stroke even when the box has no border.
        let width = if border.width > 0.0 { border.width } else { 1.0 };
        let leader = Poly::new(PolyKind::PolyLine, callout.line.clone())
            .with_ends(callout.end, LineEnding::None)
            .with_border(Border::solid(width));
        pen.c.set_dash_pattern([], 0.0);
        reach = reach.max(shape::poly(&mut pen, &leader, color, Some(color)));
    }
    if let Some(bounds) = pen.bounds {
        drawn.bbox = bounds.grown(reach);
    }
    drawn
}

/// The interior colour in the paint's output-intent space.
fn interior(paint: &Paint, color: Option<AnnotColor>) -> Option<DeviceColor> {
    let color = DeviceColor::of(color?)?;
    Some(
        Paint {
            color: Some(color),
            ..*paint
        }
        .color_or(color),
    )
}

/// Sets the stroke: colour, width, round caps and joins, and the dash pattern; `false` when the
/// border is 0 wide and nothing is stroked.
fn set_stroke(pen: &mut Pen<'_>, border: &Border, color: DeviceColor) -> bool {
    if border.width <= 0.0 {
        return false;
    }
    color.stroke(pen.c);
    pen.c.set_line_width(n(border.width));
    pen.c.set_line_cap(LineCapStyle::RoundCap);
    pen.c.set_line_join(LineJoinStyle::RoundJoin);
    if let BorderStyle::Dashed { pattern } = &border.style {
        pen.c.set_dash_pattern(pattern.iter().map(|v| n(*v)), 0.0);
    }
    true
}

/// Paints the path just drawn: filled with `fill` when there is one, stroked when `stroke`.
fn paint_path(pen: &mut Pen<'_>, fill: Option<DeviceColor>, stroke: bool) {
    if let Some(fill) = fill {
        fill.fill(pen.c);
    }
    match (fill.is_some(), stroke) {
        (true, true) => pen.c.fill_nonzero_and_stroke(),
        (true, false) => pen.c.fill_nonzero(),
        (false, true) => pen.c.stroke(),
        (false, false) => pen.c.end_path(),
    };
}

/// A vector in user space.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) struct V {
    pub(super) x: f64,
    pub(super) y: f64,
}

impl V {
    pub(super) const fn new(x: f64, y: f64) -> Self {
        Self { x, y }
    }

    pub(super) const fn of(p: UserSpacePoint) -> Self {
        Self { x: p.x, y: p.y }
    }

    pub(super) fn add(self, o: Self) -> Self {
        Self::new(self.x + o.x, self.y + o.y)
    }

    pub(super) fn sub(self, o: Self) -> Self {
        Self::new(self.x - o.x, self.y - o.y)
    }

    pub(super) fn scale(self, k: f64) -> Self {
        Self::new(self.x * k, self.y * k)
    }

    pub(super) fn len(self) -> f64 {
        self.x.hypot(self.y)
    }

    /// The unit vector along this one; zero stays zero.
    pub(super) fn unit(self) -> Self {
        let len = self.len();
        if len > 0.0 { self.scale(1.0 / len) } else { self }
    }

    /// This vector turned a quarter counter-clockwise (y up): the left normal of a direction.
    pub(super) const fn left(self) -> Self {
        Self::new(-self.y, self.x)
    }
}

/// The box of what has been drawn.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) struct Bounds {
    x0: f64,
    y0: f64,
    x1: f64,
    y1: f64,
}

impl Bounds {
    fn grown(self, by: f64) -> UserSpaceRect {
        UserSpaceRect::from_corners(self.x0 - by, self.y0 - by, self.x1 + by, self.y1 + by)
    }
}

/// Writes path operators and keeps the box of every point and control point it writes.
pub(super) struct Pen<'c> {
    pub(super) c: &'c mut Content,
    bounds: Option<Bounds>,
}

impl<'c> Pen<'c> {
    const fn new(c: &'c mut Content) -> Self {
        Self { c, bounds: None }
    }

    fn cover(&mut self, p: V) {
        self.bounds = Some(match self.bounds {
            Some(b) => Bounds {
                x0: b.x0.min(p.x),
                y0: b.y0.min(p.y),
                x1: b.x1.max(p.x),
                y1: b.y1.max(p.y),
            },
            None => Bounds {
                x0: p.x,
                y0: p.y,
                x1: p.x,
                y1: p.y,
            },
        });
    }

    pub(super) fn move_to(&mut self, p: V) {
        self.cover(p);
        self.c.move_to(n(p.x), n(p.y));
    }

    pub(super) fn line_to(&mut self, p: V) {
        self.cover(p);
        self.c.line_to(n(p.x), n(p.y));
    }

    pub(super) fn curve_to(&mut self, a: V, b: V, p: V) {
        for q in [a, b, p] {
            self.cover(q);
        }
        self.c.cubic_to(n(a.x), n(a.y), n(b.x), n(b.y), n(p.x), n(p.y));
    }

    pub(super) fn close(&mut self) {
        self.c.close_path();
    }

    /// A polygon through `points`, closed.
    pub(super) fn polygon(&mut self, points: &[V]) {
        let Some((&first, rest)) = points.split_first() else {
            return;
        };
        self.move_to(first);
        for &p in rest {
            self.line_to(p);
        }
        self.close();
    }

    /// An ellipse centred on `c` with radii `rx` and `ry`, as four quarter Béziers.
    pub(super) fn ellipse(&mut self, c: V, rx: f64, ry: f64) {
        let (kx, ky) = (rx * KAPPA, ry * KAPPA);
        let p = |x: f64, y: f64| V::new(c.x + x, c.y + y);
        self.move_to(p(rx, 0.0));
        self.curve_to(p(rx, ky), p(kx, ry), p(0.0, ry));
        self.curve_to(p(-kx, ry), p(-rx, ky), p(-rx, 0.0));
        self.curve_to(p(-rx, -ky), p(-kx, -ry), p(0.0, -ry));
        self.curve_to(p(kx, -ry), p(rx, -ky), p(rx, 0.0));
        self.close();
    }

    /// A smooth curve through `points` (Catmull-Rom splines as Béziers), or straight segments
    /// when `smooth` is off; a single point is just moved to.
    pub(super) fn through(&mut self, points: &[V], smooth: bool) {
        self.spline(points, smooth, false);
    }

    /// As [`Pen::through`], going on from the current point with a line to the first point.
    pub(super) fn on_through(&mut self, points: &[V], smooth: bool) {
        self.spline(points, smooth, true);
    }

    fn spline(&mut self, points: &[V], smooth: bool, joined: bool) {
        let Some(&first) = points.first() else {
            return;
        };
        if joined {
            self.line_to(first);
        } else {
            self.move_to(first);
        }
        if !smooth || points.len() < 3 {
            for &p in points.iter().skip(1) {
                self.line_to(p);
            }
            return;
        }
        let at = |i: usize| points.get(i).copied().unwrap_or(first);
        let last = points.len() - 1;
        for i in 0..last {
            let (p0, p1, p2) = (at(i.saturating_sub(1)), at(i), at(i + 1));
            let p3 = at((i + 2).min(last));
            let c1 = p1.add(p2.sub(p0).scale(1.0 / 6.0));
            let c2 = p2.sub(p3.sub(p1).scale(1.0 / 6.0));
            self.curve_to(c1, c2, p2);
        }
    }
}

#[cfg(test)]
#[path = "tests.rs"]
mod tests;
