//! Ink (§12.5.6.13). A stroke of the pen's width is stroked through its points with round caps
//! and joins. A pressure-varying stroke is filled as an outline: each point offset to both sides
//! by half its width, along the stroke's normal there, with a round cap at each end.

use laminapdf_ops::{Border, Ink};

use super::{DeviceColor, KAPPA, Pen, V, set_stroke};

/// Draws `ink` in `color`; returns how far the drawing reaches past the pen's box: half the
/// pen's width when a stroke is stroked, else nothing, since a pressure stroke's filled outline
/// is in the box already.
pub(super) fn ink(pen: &mut Pen<'_>, ink: &Ink, color: DeviceColor) -> f64 {
    let mut reach: f64 = 0.0;
    let mut stroking = false;
    for stroke in &ink.strokes {
        let points: Vec<V> = stroke.points.iter().map(|p| V::of(*p)).collect();
        if let Some(widths) = &stroke.widths {
            let halves: Vec<f64> = widths.iter().map(|w| f64::from(*w) / 2.0).collect();
            color.fill(pen.c);
            outline(pen, &points, &halves, ink.smoothing);
            pen.c.fill_nonzero();
            continue;
        }
        if !stroking {
            stroking = set_stroke(pen, &Border::solid(ink.width), color);
        }
        reach = ink.width / 2.0;
        if let [only] = points.as_slice() {
            // One point is a dot: a zero-length line shows nothing in some readers.
            color.fill(pen.c);
            pen.ellipse(*only, ink.width / 2.0, ink.width / 2.0);
            pen.c.fill_nonzero();
        } else {
            pen.through(&points, ink.smoothing);
            pen.c.stroke();
        }
    }
    reach
}

/// The outline of a stroke whose half-width at each point is `halves`'s: the left side forward,
/// a round cap, the right side back, a round cap.
fn outline(pen: &mut Pen<'_>, points: &[V], halves: &[f64], smooth: bool) {
    let normals = normals(points);
    let ends = (
        points.first(),
        points.last(),
        normals.first(),
        normals.last(),
        halves.first(),
        halves.last(),
    );
    let (Some(&start), Some(&end), Some(&n0), Some(&n1), Some(&h0), Some(&h1)) = ends else {
        return;
    };
    if points.len() == 1 {
        pen.ellipse(start, h0, h0);
        return;
    }
    let side = |sign: f64| -> Vec<V> {
        points
            .iter()
            .zip(&normals)
            .zip(halves)
            .map(|((p, n), h)| p.add(n.scale(sign * h)))
            .collect()
    };
    let left = side(1.0);
    let mut right = side(-1.0);
    right.reverse();
    pen.through(&left, smooth);
    cap(pen, end, n1, h1);
    pen.on_through(&right, smooth);
    cap(pen, start, n0.scale(-1.0), h0);
    pen.close();
}

/// The normal at each point: the left normal of the direction through it, from the point before
/// to the point after.
fn normals(points: &[V]) -> Vec<V> {
    let last = points.len().saturating_sub(1);
    (0..points.len())
        .map(|i| {
            let before = points.get(i.saturating_sub(1)).copied();
            let after = points.get((i + 1).min(last)).copied();
            let direction = match (before, after) {
                (Some(b), Some(a)) => a.sub(b).unit(),
                _ => V::new(1.0, 0.0),
            };
            if direction.len() > 0.0 {
                direction.left()
            } else {
                V::new(0.0, 1.0)
            }
        })
        .collect()
}

/// A half circle of radius `r` around `c`, from the side `normal` points to round to the other,
/// bulging the way the stroke goes there (`normal` turned a quarter clockwise).
fn cap(pen: &mut Pen<'_>, c: V, normal: V, r: f64) {
    let forward = V::new(normal.y, -normal.x);
    let k = r * KAPPA;
    let from = c.add(normal.scale(r));
    let tip = c.add(forward.scale(r));
    let to = c.sub(normal.scale(r));
    pen.line_to(from);
    pen.curve_to(from.add(forward.scale(k)), tip.add(normal.scale(k)), tip);
    pen.curve_to(tip.sub(normal.scale(k)), to.add(forward.scale(k)), to);
}
