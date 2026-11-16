//! The cloudy border (`/BE /S /C`, ISO 32000-2 §12.5.4): a closed outline drawn as bumps, half
//! circles bulging outward between points spaced evenly around it. The standard leaves the
//! bumps' size to the reader; here they are 4 + 8 × intensity points across, so intensity 1 and
//! 2, the values Acrobat writes, give 12- and 20-point bumps.

use super::{KAPPA, Pen, V};

/// How wide a bump is across, in points, at `intensity`.
fn bump(intensity: f64) -> f64 {
    4.0 + 8.0 * intensity
}

/// Draws the closed outline `points` as a cloud of `intensity` (above 0), as one closed path.
/// The pen's box takes in the bumps.
pub(super) fn cloud(pen: &mut Pen<'_>, points: &[V], intensity: f64) {
    let perimeter: f64 = edges(points).map(|(a, b)| b.sub(a).len()).sum();
    if perimeter <= 0.0 {
        return;
    }
    // Whole bumps around the outline, at least three.
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        clippy::cast_precision_loss,
        reason = "a positive perimeter over a bump's width, rounded: a small count, exact in f64"
    )]
    let (count, spacing) = {
        let count = ((perimeter / bump(intensity)).round() as usize).max(3);
        (count, perimeter / count as f64)
    };
    let marks = spaced(points, spacing, count);
    // Bumps bulge outward: right of the direction of travel for a counter-clockwise outline.
    let outward = if signed_area(points) >= 0.0 { -1.0 } else { 1.0 };
    let Some(&first) = marks.first() else {
        return;
    };
    pen.move_to(first);
    for (i, &a) in marks.iter().enumerate() {
        let b = marks.get(i + 1).copied().unwrap_or(first);
        half_circle(pen, a, b, outward);
    }
    pen.close();
}

/// A half circle from `from` to `to` on the side of their left normal times `side`.
fn half_circle(pen: &mut Pen<'_>, from: V, to: V, side: f64) {
    let chord = to.sub(from);
    let radius = chord.len() / 2.0;
    if radius <= 0.0 {
        return;
    }
    let along = chord.unit();
    let out = along.left().scale(side);
    let top = from.add(chord.scale(0.5)).add(out.scale(radius));
    let handle = radius * KAPPA;
    pen.curve_to(from.add(out.scale(handle)), top.sub(along.scale(handle)), top);
    pen.curve_to(top.add(along.scale(handle)), to.add(out.scale(handle)), to);
}

/// The outline's edges, the last back to the first.
fn edges(points: &[V]) -> impl Iterator<Item = (V, V)> + '_ {
    points
        .iter()
        .enumerate()
        .filter_map(|(i, &a)| points.get((i + 1) % points.len()).map(|&b| (a, b)))
}

/// `count` points spaced `spacing` apart along the closed outline, from its first point.
fn spaced(points: &[V], spacing: f64, count: usize) -> Vec<V> {
    let mut out = Vec::with_capacity(count);
    let mut next = 0.0;
    let mut walked = 0.0;
    for (a, b) in edges(points) {
        let len = b.sub(a).len();
        while next <= walked + len && out.len() < count {
            let t = if len > 0.0 { (next - walked) / len } else { 0.0 };
            out.push(a.add(b.sub(a).scale(t)));
            next += spacing;
        }
        walked += len;
    }
    out
}

/// Twice the outline's signed area: positive when its points go counter-clockwise (y up).
fn signed_area(points: &[V]) -> f64 {
    edges(points).map(|(a, b)| a.x * b.y - b.x * a.y).sum()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_square_of_side_twelve_gets_four_bumps_at_intensity_one() {
        let square = [
            V::new(0.0, 0.0),
            V::new(12.0, 0.0),
            V::new(12.0, 12.0),
            V::new(0.0, 12.0),
        ];
        assert_eq!(spaced(&square, 12.0, 4), square);
    }

    #[test]
    fn a_counter_clockwise_outline_has_a_positive_area() {
        let square = [V::new(0.0, 0.0), V::new(1.0, 0.0), V::new(1.0, 1.0), V::new(0.0, 1.0)];
        assert!(signed_area(&square) > 0.0);
    }
}
