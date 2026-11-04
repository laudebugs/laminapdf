//! The path operators: construction (§8.5.2), painting (§8.5.3) and clipping (§8.5.4).

use laminapdf_cos::{Object, Result, Warning};

use super::{Frame, Interpreter, numbers, offset_u64};
use crate::lexer::Op;
use crate::visitor::{FillRule, Paint, PathPaint, PathSegment, Visitor};

/// `m`.
pub(super) fn move_to(args: &[Object]) -> PathSegment {
    let [x, y] = numbers::<2>(args);
    PathSegment::MoveTo { x, y }
}

/// `l`.
pub(super) fn line_to(args: &[Object]) -> PathSegment {
    let [x, y] = numbers::<2>(args);
    PathSegment::LineTo { x, y }
}

/// `c`.
pub(super) fn curve_to(args: &[Object]) -> PathSegment {
    let [x1, y1, x2, y2, x3, y3] = numbers::<6>(args);
    PathSegment::CurveTo {
        c1: (x1, y1),
        c2: (x2, y2),
        end: (x3, y3),
    }
}

/// `y` (§8.5.2.2): the second control point is the end point.
pub(super) fn curve_to_y(args: &[Object]) -> PathSegment {
    let [x1, y1, x3, y3] = numbers::<4>(args);
    PathSegment::CurveTo {
        c1: (x1, y1),
        c2: (x3, y3),
        end: (x3, y3),
    }
}

/// `re`.
pub(super) fn rectangle(args: &[Object]) -> PathSegment {
    let [x, y, width, height] = numbers::<4>(args);
    PathSegment::Rect { x, y, width, height }
}

/// `B`, `B*`: fill with `rule`, then stroke.
pub(super) fn fill_stroke(rule: FillRule) -> Paint {
    Paint::FillStroke { rule, close: false }
}

/// `b`, `b*`: close the subpath, fill with `rule`, then stroke.
pub(super) fn close_fill_stroke(rule: FillRule) -> Paint {
    Paint::FillStroke { rule, close: true }
}

impl<V: Visitor + ?Sized> Interpreter<'_, '_, V> {
    /// Reports a path segment and moves the current point.
    pub(super) fn segment(&mut self, frame: &mut Frame, op: &Op<'_>, segment: PathSegment) {
        frame.path.get_or_insert(op.offset);
        match segment {
            PathSegment::MoveTo { x, y } | PathSegment::Rect { x, y, .. } => {
                frame.current = Some((x, y));
                frame.subpath_start = Some((x, y));
            }
            PathSegment::LineTo { x, y } => frame.current = Some((x, y)),
            PathSegment::CurveTo { end, .. } => frame.current = Some(end),
            PathSegment::ClosePath => frame.current = frame.subpath_start,
        }
        self.visitor.path_segment(&segment, &frame.gs.ctm);
    }

    /// `v` (§8.5.2.2): the first control point is the current point.
    pub(super) fn curve_to_v(&mut self, frame: &mut Frame, op: &Op<'_>, args: &[Object]) {
        let [x2, y2, x3, y3] = numbers::<4>(args);
        let curve = PathSegment::CurveTo {
            c1: frame.current.unwrap_or((x2, y2)),
            c2: (x2, y2),
            end: (x3, y3),
        };
        self.segment(frame, op, curve);
    }

    /// Ends the current path with `paint`, applying a pending `W`/`W*`.
    pub(super) fn paint(&mut self, frame: &mut Frame, paint: Paint) {
        frame.path = None;
        frame.current = None;
        frame.subpath_start = None;
        let clip = frame.clip.take();
        self.visitor.paint_path(&PathPaint {
            paint,
            clip,
            state: &frame.gs,
        });
    }

    /// Drops the unpainted path whose first segment is at `offset`.
    pub(super) fn discard_path(&mut self, frame: &mut Frame, offset: usize) -> Result<()> {
        self.paint(frame, Paint::Discarded);
        self.warn(Warning::ContentPathUnpainted {
            object: frame.object,
            offset: offset_u64(offset),
        })
    }
}
