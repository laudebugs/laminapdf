//! The ruling lines a page draws, in page space: stroked straight segments and rectangles, and
//! filled rectangles thin enough to be a rule (ISO 32000-2 §8.5.2, §8.5.3). Diagonal and curved
//! segments are left out; tables are ruled with horizontal and vertical lines.

use laminapdf_content::{Paint, PathPaint, PathSegment, Visitor};
use laminapdf_model::{Matrix, PageFrame, PointPt, RectPt, UserSpacePoint};

/// A filled rectangle thinner than this (in points) is a rule, not an area.
const THIN: f64 = 2.0;

/// How far from horizontal or vertical a segment may be (in points) and still count.
const AXIS_TOLERANCE: f64 = 0.5;

/// A horizontal or vertical rule: `at` is its y (horizontal) or x (vertical), `from..to` the
/// span along the other axis, `from <= to`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct Rule {
    pub(crate) at: f64,
    pub(crate) from: f64,
    pub(crate) to: f64,
}

/// The page's rules, and its filled areas (for shaded header rows).
#[derive(Debug, Default)]
pub(crate) struct Edges {
    pub(crate) horizontal: Vec<Rule>,
    pub(crate) vertical: Vec<Rule>,
    pub(crate) fills: Vec<RectPt>,
}

impl Edges {
    fn add_segment(&mut self, a: PointPt, b: PointPt) {
        let (ax, ay, bx, by) = (a.x.get(), a.y.get(), b.x.get(), b.y.get());
        if (ay - by).abs() <= AXIS_TOLERANCE && (ax - bx).abs() > AXIS_TOLERANCE {
            self.horizontal.push(Rule {
                at: f64::midpoint(ay, by),
                from: ax.min(bx),
                to: ax.max(bx),
            });
        } else if (ax - bx).abs() <= AXIS_TOLERANCE && (ay - by).abs() > AXIS_TOLERANCE {
            self.vertical.push(Rule {
                at: f64::midpoint(ax, bx),
                from: ay.min(by),
                to: ay.max(by),
            });
        }
    }

    fn add_rect_outline(&mut self, r: &RectPt) {
        let (x0, y0, x1, y1) = (r.x0.get(), r.y0.get(), r.x1.get(), r.y1.get());
        for (at, from, to) in [(y0, x0, x1), (y1, x0, x1)] {
            self.horizontal.push(Rule { at, from, to });
        }
        for (at, from, to) in [(x0, y0, y1), (x1, y0, y1)] {
            self.vertical.push(Rule { at, from, to });
        }
    }

    /// A filled rectangle: a rule along its long side when it is thin, else an area.
    fn add_filled_rect(&mut self, r: &RectPt) {
        let (w, h) = (r.width().get(), r.height().get());
        let (x0, y0, x1, y1) = (r.x0.get(), r.y0.get(), r.x1.get(), r.y1.get());
        if h <= THIN && w > h {
            self.horizontal.push(Rule {
                at: f64::midpoint(y0, y1),
                from: x0,
                to: x1,
            });
        } else if w <= THIN && h > w {
            self.vertical.push(Rule {
                at: f64::midpoint(x0, x1),
                from: y0,
                to: y1,
            });
        } else {
            self.fills.push(*r);
        }
    }
}

/// A content visitor that gathers the page's rules.
pub(crate) struct Collector<'f> {
    frame: &'f PageFrame,
    edges: Edges,
    /// The current path: straight segments and rectangles, in page space.
    segments: Vec<(PointPt, PointPt)>,
    rects: Vec<RectPt>,
    start: Option<PointPt>,
    current: Option<PointPt>,
}

impl<'f> Collector<'f> {
    pub(crate) const fn new(frame: &'f PageFrame) -> Self {
        Self {
            frame,
            edges: Edges {
                horizontal: Vec::new(),
                vertical: Vec::new(),
                fills: Vec::new(),
            },
            segments: Vec::new(),
            rects: Vec::new(),
            start: None,
            current: None,
        }
    }

    pub(crate) fn finish(self) -> Edges {
        self.edges
    }

    fn page(&self, ctm: &Matrix, x: f64, y: f64) -> PointPt {
        let (ux, uy) = ctm.apply(x, y);
        self.frame.to_page(UserSpacePoint::new(ux, uy))
    }

    fn clear(&mut self) {
        self.segments.clear();
        self.rects.clear();
        self.start = None;
        self.current = None;
    }
}

impl Visitor for Collector<'_> {
    fn path_segment(&mut self, segment: &PathSegment, ctm: &Matrix) {
        match *segment {
            PathSegment::MoveTo { x, y } => {
                let p = self.page(ctm, x, y);
                self.start = Some(p);
                self.current = Some(p);
            }
            PathSegment::LineTo { x, y } => {
                let p = self.page(ctm, x, y);
                if let Some(from) = self.current {
                    self.segments.push((from, p));
                }
                self.current = Some(p);
            }
            PathSegment::CurveTo { end, .. } => self.current = Some(self.page(ctm, end.0, end.1)),
            PathSegment::ClosePath => {
                if let (Some(from), Some(to)) = (self.current, self.start) {
                    self.segments.push((from, to));
                }
                self.current = self.start;
            }
            PathSegment::Rect { x, y, width, height } => {
                let a = self.page(ctm, x, y);
                let b = self.page(ctm, x + width, y + height);
                // A rotated CTM turns the rectangle; only an axis-aligned one is a box here.
                let c = self.page(ctm, x + width, y);
                let axis_aligned =
                    (c.x.get() - b.x.get()).abs() <= AXIS_TOLERANCE || (c.y.get() - b.y.get()).abs() <= AXIS_TOLERANCE;
                if axis_aligned {
                    self.rects.push(RectPt::new(a.x, a.y, b.x, b.y));
                }
                self.start = Some(a);
                self.current = Some(a);
            }
        }
    }

    fn paint_path(&mut self, paint: &PathPaint<'_>) {
        let (stroked, filled) = match paint.paint {
            Paint::Stroke { .. } => (true, false),
            Paint::Fill(_) => (false, true),
            Paint::FillStroke { .. } => (true, true),
            _ => (false, false),
        };
        if stroked {
            for &(a, b) in &self.segments {
                self.edges.add_segment(a, b);
            }
            for r in &self.rects {
                self.edges.add_rect_outline(r);
            }
        } else if filled {
            for r in &self.rects {
                self.edges.add_filled_rect(r);
            }
        }
        self.clear();
    }
}

#[cfg(test)]
mod tests {
    use laminapdf_model::{Pt, RectPt};

    use super::Edges;

    fn rect(x0: f64, y0: f64, x1: f64, y1: f64) -> RectPt {
        RectPt::new(Pt::new(x0), Pt::new(y0), Pt::new(x1), Pt::new(y1))
    }

    #[test]
    fn a_thin_filled_rectangle_is_a_rule_and_a_wide_one_an_area() {
        let mut edges = Edges::default();
        edges.add_filled_rect(&rect(10.0, 100.0, 200.0, 101.0));
        edges.add_filled_rect(&rect(50.0, 10.0, 50.5, 300.0));
        edges.add_filled_rect(&rect(10.0, 10.0, 200.0, 40.0));
        assert_eq!(edges.horizontal.len(), 1);
        assert_eq!(edges.vertical.len(), 1);
        assert_eq!(edges.fills.len(), 1);
        assert!((edges.horizontal[0].at - 100.5).abs() < 1e-9);
    }

    #[test]
    fn a_stroked_rectangle_gives_its_four_sides() {
        let mut edges = Edges::default();
        edges.add_rect_outline(&rect(0.0, 0.0, 10.0, 20.0));
        assert_eq!((edges.horizontal.len(), edges.vertical.len()), (2, 2));
    }
}
