//! Lines (§12.5.6.7): the line, its endings, and leader lines. With leader lines the line is
//! drawn `/LL` points off its two points, to the left of its direction for a positive `/LL`
//! (counter-clockwise from the line, Table 178), and each leader line runs from `/LLO` off a
//! point to `/LLE` past the line.

use laminapdf_ops::{LineEnding, LineMark};

use super::ending::ending;
use super::{DeviceColor, Pen, V, set_stroke};

/// Draws `line` in `color`, closed endings filled with `fill`; returns how far the drawing
/// reaches past the points: half the stroke.
pub(super) fn line(pen: &mut Pen<'_>, line: &LineMark, color: DeviceColor, fill: Option<DeviceColor>) -> f64 {
    let (from, to) = (V::of(line.from), V::of(line.to));
    let direction = to.sub(from).unit();
    let direction = if direction.len() > 0.0 {
        direction
    } else {
        V::new(1.0, 0.0)
    };
    let stroke = set_stroke(pen, &line.border, color);
    let (start, end) = match &line.leader {
        Some(leader) => {
            let side = direction.left();
            let sign = if leader.length < 0.0 { -1.0 } else { 1.0 };
            for p in [from, to] {
                pen.move_to(p.add(side.scale(sign * leader.offset)));
                pen.line_to(p.add(side.scale(leader.length + sign * leader.extension)));
            }
            let shift = side.scale(leader.length);
            (from.add(shift), to.add(shift))
        }
        None => (from, to),
    };
    pen.move_to(start);
    pen.line_to(end);
    if stroke {
        pen.c.stroke();
    } else {
        pen.c.end_path();
    }
    draw_ends(
        pen,
        line.ends,
        [(start, direction.scale(-1.0)), (end, direction)],
        line.border.width,
        fill,
        stroke,
    );
    line.border.width / 2.0
}

/// Draws the two endings, solid whatever the line's dashes, at each `(tip, outward direction)`.
pub(super) fn draw_ends(
    pen: &mut Pen<'_>,
    ends: [LineEnding; 2],
    at: [(V, V); 2],
    width: f64,
    fill: Option<DeviceColor>,
    stroke: bool,
) {
    if ends == [LineEnding::None; 2] {
        return;
    }
    pen.c.set_dash_pattern([], 0.0);
    for (kind, (tip, d)) in ends.into_iter().zip(at) {
        ending(pen, kind, tip, d, width, fill, stroke);
    }
}
