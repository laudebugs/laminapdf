//! Squares and circles (§12.5.6.8), polygons and polylines (§12.5.6.9): filled with the interior
//! colour, then outlined, solid, dashed or cloudy. A shape's outline runs along its box, so half
//! the stroke falls outside it; a cloud's bumps reach further out.

use laminapdf_ops::{BorderStyle, Poly, PolyKind, Shape, ShapeKind};

use super::cloud::cloud;
use super::line::draw_ends;
use super::{DeviceColor, Pen, V, paint_path, set_stroke};

/// How many points stand for an ellipse under a cloud: the bumps follow the points' polygon.
const ELLIPSE_POINTS: u32 = 48;

/// Draws `shape` outlined in `color` and filled with `fill`; returns how far the drawing reaches
/// past the box.
pub(super) fn shape(pen: &mut Pen<'_>, shape: &Shape, color: DeviceColor, fill: Option<DeviceColor>) -> f64 {
    let r = shape.rect;
    let stroke = set_stroke(pen, &shape.border, color);
    let half = shape.border.width / 2.0;
    match (&shape.border.style, shape.kind) {
        (BorderStyle::Cloudy { intensity }, kind) if *intensity > 0.0 => {
            let outline = match kind {
                ShapeKind::Square => corners(r.x0, r.y0, r.x1, r.y1),
                ShapeKind::Circle => ellipse_points(r.x0, r.y0, r.x1, r.y1),
            };
            cloud(pen, &outline, *intensity);
            paint_path(pen, fill, stroke);
            half
        }
        (_, ShapeKind::Circle) => {
            let c = V::new(f64::midpoint(r.x0, r.x1), f64::midpoint(r.y0, r.y1));
            pen.ellipse(c, (r.x1 - r.x0) / 2.0, (r.y1 - r.y0) / 2.0);
            paint_path(pen, fill, stroke);
            half
        }
        _ => {
            pen.polygon(&corners(r.x0, r.y0, r.x1, r.y1));
            paint_path(pen, fill, stroke);
            half
        }
    }
}

/// Draws `poly` in `color`: a polygon filled with `fill`, or a polyline whose closed endings
/// are; returns how far the drawing reaches past the vertices.
pub(super) fn poly(pen: &mut Pen<'_>, poly: &Poly, color: DeviceColor, fill: Option<DeviceColor>) -> f64 {
    let points: Vec<V> = poly.vertices.iter().map(|p| V::of(*p)).collect();
    let stroke = set_stroke(pen, &poly.border, color);
    let half = poly.border.width / 2.0;
    match (poly.kind, &poly.border.style) {
        (PolyKind::Polygon, BorderStyle::Cloudy { intensity }) if *intensity > 0.0 => {
            cloud(pen, &points, *intensity);
            paint_path(pen, fill, stroke);
            half
        }
        (PolyKind::Polygon, _) => {
            pen.polygon(&points);
            paint_path(pen, fill, stroke);
            half
        }
        _ => {
            pen.through(&points, false);
            paint_path(pen, None, stroke);
            if let (Some(&a), Some(&z)) = (points.first(), points.last()) {
                let start = away(a, points.iter().skip(1), V::new(-1.0, 0.0));
                let end = away(z, points.iter().rev().skip(1), V::new(1.0, 0.0));
                draw_ends(pen, poly.ends, [(a, start), (z, end)], poly.border.width, fill, stroke);
            }
            half
        }
    }
}

/// The direction from the first of `others` that differs from `tip` to `tip`, so a repeated end
/// vertex (a polyline finished by a double click) keeps its ending; `fallback` when every vertex
/// is `tip`, as a line of one point falls back to x.
fn away<'a>(tip: V, mut others: impl Iterator<Item = &'a V>, fallback: V) -> V {
    others
        .find(|v| v.sub(tip).len() > 0.0)
        .map_or(fallback, |&v| tip.sub(v).unit())
}

/// A box's corners, counter-clockwise from the lower left.
fn corners(x0: f64, y0: f64, x1: f64, y1: f64) -> Vec<V> {
    vec![V::new(x0, y0), V::new(x1, y0), V::new(x1, y1), V::new(x0, y1)]
}

/// Points around the ellipse inside a box, counter-clockwise.
fn ellipse_points(x0: f64, y0: f64, x1: f64, y1: f64) -> Vec<V> {
    let (cx, cy) = (f64::midpoint(x0, x1), f64::midpoint(y0, y1));
    let (rx, ry) = ((x1 - x0) / 2.0, (y1 - y0) / 2.0);
    (0..ELLIPSE_POINTS)
        .map(|i| {
            let a = std::f64::consts::TAU * f64::from(i) / f64::from(ELLIPSE_POINTS);
            V::new(cx + rx * a.cos(), cy + ry * a.sin())
        })
        .collect()
}
