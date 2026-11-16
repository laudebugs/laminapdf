//! Line endings (`/LE`, ISO 32000-2 Table 179), drawn at an end of a line or polyline. The table
//! names the shapes but leaves their size to the reader; here an arrowhead is 6 points plus 3
//! times the line's width long, its sides 30° from the line, and the square, circle, diamond,
//! butt and slash are as wide as an arrowhead is long. Closed endings are filled with the interior
//! colour when there is one, as the table says.

use laminapdf_ops::LineEnding;

use super::{DeviceColor, Pen, V, paint_path};

/// How long an arrowhead is for a line `width` points wide.
pub(super) fn size(width: f64) -> f64 {
    6.0 + 3.0 * width.max(0.0)
}

/// Draws `ending` at `tip`, the line arriving there going in direction `d` (a unit vector,
/// pointing out of the line), stroked when `stroke` and filled with `fill` where it is closed.
pub(super) fn ending(
    pen: &mut Pen<'_>,
    ending: LineEnding,
    tip: V,
    d: V,
    width: f64,
    fill: Option<DeviceColor>,
    stroke: bool,
) {
    let len = size(width);
    let half = len / 2.0;
    let across = d.left();
    // Sides 30° from the line: half as wide across as tan 30° of the length.
    let wing = len * 0.577_350_27;
    match ending {
        LineEnding::None => {}
        LineEnding::OpenArrow | LineEnding::ROpenArrow => {
            let back = if ending == LineEnding::OpenArrow { -1.0 } else { 1.0 };
            let base = tip.add(d.scale(back * len));
            pen.move_to(base.add(across.scale(wing)));
            pen.line_to(tip);
            pen.line_to(base.sub(across.scale(wing)));
            paint_path(pen, None, stroke);
        }
        LineEnding::ClosedArrow | LineEnding::RClosedArrow => {
            let back = if ending == LineEnding::ClosedArrow { -1.0 } else { 1.0 };
            let base = tip.add(d.scale(back * len));
            pen.polygon(&[base.add(across.scale(wing)), tip, base.sub(across.scale(wing))]);
            paint_path(pen, fill, stroke);
        }
        LineEnding::Square => {
            let (a, b) = (d.scale(half), across.scale(half));
            pen.polygon(&[
                tip.add(a).add(b),
                tip.sub(a).add(b),
                tip.sub(a).sub(b),
                tip.add(a).sub(b),
            ]);
            paint_path(pen, fill, stroke);
        }
        LineEnding::Circle => {
            pen.ellipse(tip, half, half);
            paint_path(pen, fill, stroke);
        }
        LineEnding::Diamond => {
            let (a, b) = (d.scale(half), across.scale(half));
            pen.polygon(&[tip.add(a), tip.add(b), tip.sub(a), tip.sub(b)]);
            paint_path(pen, fill, stroke);
        }
        LineEnding::Butt => {
            pen.move_to(tip.add(across.scale(half)));
            pen.line_to(tip.sub(across.scale(half)));
            paint_path(pen, None, stroke);
        }
        LineEnding::Slash => {
            // Across the line, turned 30° towards it.
            let slant = across.scale(0.866_025_4).add(d.scale(0.5)).scale(half);
            pen.move_to(tip.add(slant));
            pen.line_to(tip.sub(slant));
            paint_path(pen, None, stroke);
        }
    }
}
