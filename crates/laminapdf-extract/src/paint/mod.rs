//! What a page paints, seen in the same walk of its content as its text: the table rules
//! ([`Edges`]) and the opaque areas painted over earlier text ([`Cover`]s, ADR 0030 §4's
//! `covered` flag), from the page's content and, in [`annots`], its annotations' appearances.
//!
//! Paints are counted as `laminapdf-text` counts them for [`laminapdf_text::Glyph::paints_before`]
//! (paths that draw, images and shadings, forms included), so each cover knows which glyphs it
//! was drawn over. Only what is plainly opaque covers, and the rest leans towards keeping text
//! (ADR 0030 §4): filled rectangles and polygons with horizontal and vertical sides, and
//! unmasked images, each cut to the clip's box, painted with constant alpha 1, no soft mask and
//! the `Normal` blend mode (or `Multiply` or `Darken` with a fill near black), outside optional
//! content and outside a transparency group drawn translucently. Strokes, shadings, curves and
//! rotated shapes never cover.

pub(crate) mod annots;

use std::collections::BTreeMap;

use laminapdf_content::{
    FormXObject, GraphicsState, ImageDraw, ImageSource, MarkedContent, Paint, PathPaint, PathSegment, ShadingPaint,
    Visitor, transformed_box,
};
use laminapdf_model::{BlendMode, Matrix, PageFrame, PointPt, Pt, RectPt, UserSpacePoint, UserSpaceRect};
use laminapdf_syntax::{Dict, Object};

use crate::tables::{AXIS_TOLERANCE, Edges};

/// An image over at least this share of the page is a scan: the text under it is its OCR layer.
const SCAN_SHARE: f64 = 0.9;

/// A fill whose every RGB component is at most this is near black (for `Multiply` and `Darken`).
const NEAR_BLACK: f64 = 0.1;

/// An opaque area painted over the text drawn before it, in page space.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct Cover {
    pub(crate) rect: RectPt,
    /// The paints before it: it covers glyphs whose `paints_before` is at most this.
    pub(crate) after: u32,
}

/// The intersection of two rectangles, `None` when they don't overlap.
pub(crate) fn intersect(a: &RectPt, b: &RectPt) -> Option<RectPt> {
    let (x0, y0) = (a.x0.get().max(b.x0.get()), a.y0.get().max(b.y0.get()));
    let (x1, y1) = (a.x1.get().min(b.x1.get()), a.y1.get().min(b.y1.get()));
    (x0 < x1 && y0 < y1).then(|| RectPt::new(Pt::new(x0), Pt::new(y0), Pt::new(x1), Pt::new(y1)))
}

fn area(r: &RectPt) -> f64 {
    r.width().get() * r.height().get()
}

/// Whether a paint in `state` is opaque, for a fill of colour `rgb` (`None` for an image).
fn opaque(state: &GraphicsState, rgb: Option<[f64; 3]>) -> bool {
    if state.fill_alpha < 1.0 || state.soft_mask {
        return false;
    }
    match state.blend_mode {
        BlendMode::Normal => true,
        // Whatever is under a near-black fill turns black: a dark highlighter over dark text.
        BlendMode::Multiply | BlendMode::Darken => rgb.is_some_and(|c| c.iter().all(|&v| v <= NEAR_BLACK)),
        _ => false,
    }
}

/// Whether an image dictionary says the image is masked or in optional content (§8.9.6,
/// §11.6.5.3, §8.11.3.3), so it doesn't cover. Inline images write `/IM` for `/ImageMask`.
fn masked_or_optional(dict: &Dict) -> bool {
    let set = |key: &str| matches!(dict.get(key), Some(Object::Bool(true)));
    set("ImageMask")
        || set("IM")
        || dict.get("Mask").is_some()
        || dict.get("SMask").is_some()
        || dict.get("OC").is_some()
        || dict
            .get("SMaskInData")
            .and_then(Object::as_f64)
            .is_some_and(|v| v > 0.0)
}

/// The current path's subpaths, in page space: straight segments (with their subpath's number),
/// rectangles, each subpath's start, and the subpaths with a curve.
#[derive(Debug, Default)]
struct Path {
    segments: Vec<(PointPt, PointPt, usize)>,
    rects: Vec<RectPt>,
    starts: BTreeMap<usize, PointPt>,
    curved: Vec<usize>,
    subpath: usize,
    start: Option<PointPt>,
    current: Option<PointPt>,
}

/// Whether the segment from `a` to `b` is horizontal or vertical.
fn on_axis(a: PointPt, b: PointPt) -> bool {
    (a.x.get() - b.x.get()).abs() <= AXIS_TOLERANCE || (a.y.get() - b.y.get()).abs() <= AXIS_TOLERANCE
}

impl Path {
    /// `s`, `b` and `b*` close the current subpath before painting; the interpreter reports
    /// them as a stroke with `close` and no `ClosePath`.
    fn close(&mut self) {
        if let (Some(from), Some(to)) = (self.current, self.start) {
            self.segments.push((from, to, self.subpath));
        }
        self.current = self.start;
    }

    /// Each subpath of straight horizontal and vertical sides, as its bounding box, with
    /// whether the side that closes it (back to its start, as a fill closes it) is on an axis
    /// too: only then is the box the area filled.
    fn polygons(&self) -> Vec<(RectPt, bool)> {
        let mut boxes: BTreeMap<usize, (f64, f64, f64, f64, bool, PointPt)> = BTreeMap::new();
        for &(a, b, n) in &self.segments {
            let axis = on_axis(a, b);
            let e = boxes.entry(n).or_insert((
                f64::INFINITY,
                f64::INFINITY,
                f64::NEG_INFINITY,
                f64::NEG_INFINITY,
                true,
                a,
            ));
            for p in [a, b] {
                *e = (
                    e.0.min(p.x.get()),
                    e.1.min(p.y.get()),
                    e.2.max(p.x.get()),
                    e.3.max(p.y.get()),
                    e.4 && axis,
                    b,
                );
            }
        }
        boxes
            .into_iter()
            .filter(|(n, b)| b.4 && !self.curved.contains(n))
            .map(|(n, (x0, y0, x1, y1, _, end))| {
                let closes_on_axis = self.starts.get(&n).is_none_or(|&start| on_axis(end, start));
                (
                    RectPt::new(Pt::new(x0), Pt::new(y0), Pt::new(x1), Pt::new(y1)),
                    closes_on_axis,
                )
            })
            .collect()
    }
}

/// A content visitor that gathers a page's rules and covers.
pub(crate) struct Collector<'f> {
    frame: &'f PageFrame,
    /// Maps the content's default user space to the page's (an annotation's appearance
    /// matrix; the identity for the page's own content).
    outer: Matrix,
    /// What the content may paint on (an annotation's `/Rect`); `None` for the whole page.
    bounds: Option<RectPt>,
    /// The `after` of every cover: `None` counts paints, `Some` gives every cover that value
    /// (annotations, drawn after the whole page).
    fixed_after: Option<u32>,
    edges: Edges,
    covers: Vec<Cover>,
    path: Path,
    paints: u32,
    /// For each open marked-content sequence, whether it is optional content (`/OC`).
    marked: Vec<bool>,
    /// For each open form, whether nothing in it may cover: it has `/OC`, or a `/Group` drawn
    /// while the state wasn't opaque.
    forms: Vec<bool>,
}

impl<'f> Collector<'f> {
    pub(crate) fn new(frame: &'f PageFrame) -> Self {
        Self {
            frame,
            outer: Matrix::IDENTITY,
            bounds: None,
            fixed_after: None,
            edges: Edges::default(),
            covers: Vec::new(),
            path: Path::default(),
            paints: 0,
            marked: Vec::new(),
            forms: Vec::new(),
        }
    }

    /// A collector for an annotation's appearance: content mapped by `outer` and kept inside
    /// `bounds`, every cover drawn after the page.
    pub(crate) fn appearance(frame: &'f PageFrame, outer: Matrix, bounds: RectPt) -> Self {
        Self {
            outer,
            bounds: Some(bounds),
            fixed_after: Some(u32::MAX),
            ..Self::new(frame)
        }
    }

    /// The rules and the covers.
    pub(crate) fn finish(self) -> (Edges, Vec<Cover>) {
        (self.edges, self.covers)
    }

    fn page(&self, ctm: &Matrix, x: f64, y: f64) -> PointPt {
        let (ux, uy) = ctm.then(&self.outer).apply(x, y);
        self.frame.to_page(UserSpacePoint::new(ux, uy))
    }

    fn page_rect(&self, rect: &UserSpaceRect) -> RectPt {
        self.frame.rect_to_page(&transformed_box(rect, &self.outer))
    }

    /// The paint's number, counting it.
    fn count(&mut self) -> u32 {
        let n = self.paints;
        self.paints = self.paints.saturating_add(1);
        n
    }

    /// Whether optional content or a translucent group is open, where nothing covers.
    fn hidden(&self) -> bool {
        self.marked.iter().chain(&self.forms).any(|&h| h)
    }

    /// Records `rect` as a cover painted after `after` paints, cut to the clip and the bounds.
    fn cover(&mut self, rect: RectPt, after: u32, state: &GraphicsState) {
        let mut rect = Some(rect);
        if let Some(clip) = state.clip_box {
            let clip = self.page_rect(&clip);
            rect = rect.and_then(|r| intersect(&r, &clip));
        }
        if let Some(bounds) = self.bounds {
            rect = rect.and_then(|r| intersect(&r, &bounds));
        }
        if let Some(rect) = rect {
            self.covers.push(Cover {
                rect,
                after: self.fixed_after.unwrap_or(after),
            });
        }
    }

    /// The covers of a filled path: its rectangles and closed axis-aligned polygons. A box
    /// holding another box of the same path may be a frame with a hole (`f*`), so neither
    /// covers.
    fn fill_covers(&mut self, after: u32, state: &GraphicsState) {
        let mut boxes: Vec<RectPt> = self.path.rects.clone();
        boxes.extend(
            self.path
                .polygons()
                .into_iter()
                .filter(|(_, closed)| *closed)
                .map(|(r, _)| r),
        );
        let holds = |a: &RectPt, b: &RectPt| a.x0 <= b.x0 && a.y0 <= b.y0 && b.x1 <= a.x1 && b.y1 <= a.y1;
        let nested = |i: usize, r: &RectPt| {
            boxes
                .iter()
                .enumerate()
                .any(|(j, o)| j != i && (holds(r, o) || holds(o, r)))
        };
        let keep: Vec<RectPt> = boxes
            .iter()
            .enumerate()
            .filter(|(i, r)| !nested(*i, r))
            .map(|(_, r)| *r)
            .collect();
        for rect in keep {
            self.cover(rect, after, state);
        }
    }

    fn image_cover(&self, image: &ImageDraw<'_>) -> Option<RectPt> {
        let masked = match image.source {
            ImageSource::XObject {
                stream: Object::Stream(s),
                ..
            } => masked_or_optional(&s.dict),
            ImageSource::Inline(inline) => masked_or_optional(&inline.dict),
            _ => true,
        };
        if masked || !opaque(image.state, None) {
            return None;
        }
        // The CTM maps the unit square onto the page (§8.9.4); only an upright or quarter-turned
        // one is a box.
        let m = &image.state.ctm;
        let [o, x, y] = [(0.0, 0.0), (1.0, 0.0), (0.0, 1.0)].map(|(u, v)| self.page(m, u, v));
        if !(on_axis(o, x) && on_axis(o, y)) {
            return None;
        }
        let far = self.page(m, 1.0, 1.0);
        let rect = RectPt::new(o.x, o.y, far.x, far.y);
        let page = RectPt::new(Pt::ZERO, Pt::ZERO, self.frame.width(), self.frame.height());
        let on_page = intersect(&rect, &page)?;
        (area(&on_page) < SCAN_SHARE * area(&page)).then_some(rect)
    }
}

impl Visitor for Collector<'_> {
    fn path_segment(&mut self, segment: &PathSegment, ctm: &Matrix) {
        match *segment {
            PathSegment::MoveTo { x, y } => {
                let p = self.page(ctm, x, y);
                self.path.subpath += 1;
                self.path.starts.insert(self.path.subpath, p);
                self.path.start = Some(p);
                self.path.current = Some(p);
            }
            PathSegment::LineTo { x, y } => {
                let p = self.page(ctm, x, y);
                if let Some(from) = self.path.current {
                    self.path.segments.push((from, p, self.path.subpath));
                }
                self.path.current = Some(p);
            }
            PathSegment::CurveTo { end, .. } => {
                self.path.curved.push(self.path.subpath);
                self.path.current = Some(self.page(ctm, end.0, end.1));
            }
            PathSegment::ClosePath => self.path.close(),
            PathSegment::Rect { x, y, width, height } => {
                let a = self.page(ctm, x, y);
                let b = self.page(ctm, x + width, y + height);
                // A rotated CTM turns the rectangle; only an axis-aligned one is a box here.
                let c = self.page(ctm, x + width, y);
                if on_axis(c, b) {
                    self.path.rects.push(RectPt::new(a.x, a.y, b.x, b.y));
                }
                self.path.subpath += 1;
                self.path.start = Some(a);
                self.path.current = Some(a);
            }
        }
    }

    fn paint_path(&mut self, paint: &PathPaint<'_>) {
        let (stroked, filled, close) = match paint.paint {
            Paint::Stroke { close } => (true, false, close),
            Paint::Fill(_) => (false, true, false),
            Paint::FillStroke { close, .. } => (true, true, close),
            _ => (false, false, false),
        };
        let after = if paint.paint.draws() { self.count() } else { self.paints };
        if close {
            self.path.close();
        }
        if stroked {
            for &(a, b, _) in &self.path.segments {
                self.edges.add_segment(a, b);
            }
            for r in &self.path.rects {
                self.edges.add_rect_outline(r);
            }
        } else if filled {
            for r in &self.path.rects {
                self.edges.add_filled_rect(r);
            }
            for (r, _) in self.path.polygons() {
                self.edges.add_filled_rect(&r);
            }
        }
        if filled && !self.hidden() && opaque(paint.state, paint.state.fill.approx_rgb()) {
            self.fill_covers(after, paint.state);
        }
        self.path = Path::default();
    }

    fn image(&mut self, image: &ImageDraw<'_>) {
        let after = self.count();
        if self.hidden() {
            return;
        }
        if let Some(rect) = self.image_cover(image) {
            self.cover(rect, after, image.state);
        }
    }

    fn paint_shading(&mut self, _: &ShadingPaint<'_>) {
        self.count();
    }

    fn begin_marked(&mut self, marked: &MarkedContent<'_>) {
        self.marked.push(marked.tag.as_bytes() == b"OC");
    }

    fn end_marked(&mut self) {
        self.marked.pop();
    }

    fn enter_xobject(&mut self, form: &FormXObject<'_>) {
        let optional = form.dict.get("OC").is_some();
        let translucent_group = form.dict.get("Group").is_some() && !opaque(form.state, None);
        self.forms.push(optional || translucent_group);
    }

    fn leave_xobject(&mut self, _: &FormXObject<'_>) {
        self.forms.pop();
    }
}

#[cfg(test)]
mod tests {
    use laminapdf_content::{ColorSpace, GraphicsState};
    use laminapdf_model::{BlendMode, Pt, RectPt};
    use laminapdf_syntax::{Dict, Object};

    use super::{intersect, masked_or_optional, opaque};

    fn state(alpha: f64, blend: BlendMode, soft_mask: bool) -> GraphicsState {
        let mut s = GraphicsState::default();
        s.fill_alpha = alpha;
        s.blend_mode = blend;
        s.soft_mask = soft_mask;
        s
    }

    #[test]
    fn only_full_alpha_without_a_soft_mask_in_normal_is_opaque() {
        let grey = Some([0.5; 3]);
        assert!(opaque(&state(1.0, BlendMode::Normal, false), grey));
        assert!(!opaque(&state(0.99, BlendMode::Normal, false), grey));
        assert!(!opaque(&state(1.0, BlendMode::Normal, true), grey));
        assert!(!opaque(&state(1.0, BlendMode::Screen, false), grey));
    }

    #[test]
    fn multiply_and_darken_are_opaque_only_near_black() {
        for blend in [BlendMode::Multiply, BlendMode::Darken] {
            assert!(opaque(&state(1.0, blend, false), Some([0.1, 0.0, 0.05])));
            assert!(!opaque(&state(1.0, blend, false), Some([1.0, 1.0, 0.0])));
            // An image has no fill colour: never opaque in these modes.
            assert!(!opaque(&state(1.0, blend, false), None));
        }
    }

    #[test]
    fn a_pattern_fill_has_no_colour_so_never_covers_in_multiply() {
        let mut s = state(1.0, BlendMode::Multiply, false);
        s.fill.space = ColorSpace::Pattern;
        assert!(!opaque(&s, s.fill.approx_rgb()));
    }

    #[test]
    fn masks_and_optional_content_keep_an_image_from_covering() {
        let with = |key: &str, value: Object| [(key, value)].into_iter().collect::<Dict>();
        assert!(masked_or_optional(&with("ImageMask", Object::Bool(true))));
        assert!(masked_or_optional(&with("IM", Object::Bool(true))));
        assert!(masked_or_optional(&with("SMask", Object::Null)));
        assert!(masked_or_optional(&with("Mask", Object::Null)));
        assert!(masked_or_optional(&with("OC", Object::Null)));
        assert!(masked_or_optional(&with("SMaskInData", 1.into())));
        assert!(!masked_or_optional(&with("SMaskInData", 0.into())));
        assert!(!masked_or_optional(&with("ImageMask", Object::Bool(false))));
    }

    #[test]
    fn rectangles_that_only_touch_have_no_intersection() {
        let r = |x0: f64, x1: f64| RectPt::new(Pt::new(x0), Pt::ZERO, Pt::new(x1), Pt::new(10.0));
        assert_eq!(intersect(&r(0.0, 10.0), &r(10.0, 20.0)), None);
        assert_eq!(intersect(&r(0.0, 10.0), &r(5.0, 20.0)), Some(r(5.0, 10.0)));
    }
}
