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

use std::collections::{BTreeMap, BTreeSet};

use laminapdf_content::{
    ColorSpace, FormXObject, GraphicsState, ImageDraw, ImageSource, MarkedContent, Paint, PathPaint, PathSegment,
    ShadingPaint, Visitor, transformed_box,
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
    curved: BTreeSet<usize>,
    subpath: usize,
    start: Option<PointPt>,
    current: Option<PointPt>,
}

/// Whether the segment from `a` to `b` is horizontal or vertical.
fn on_axis(a: PointPt, b: PointPt) -> bool {
    (a.x.get() - b.x.get()).abs() <= AXIS_TOLERANCE || (a.y.get() - b.y.get()).abs() <= AXIS_TOLERANCE
}

/// A subpath of straight horizontal and vertical sides, as its bounding box.
struct Polygon {
    rect: RectPt,
    /// Whether it fills exactly its box: every point a corner of the box, and the side that
    /// closes it (back to its start, as a fill closes it) on an axis too. An L, a U or a frame
    /// drawn as one subpath doesn't.
    rectangle: bool,
}

/// Whether `p` is one of `r`'s corners.
fn at_corner(p: PointPt, r: &RectPt) -> bool {
    let near = |a: f64, b: f64| (a - b).abs() <= AXIS_TOLERANCE;
    let (x, y) = (p.x.get(), p.y.get());
    (near(x, r.x0.get()) || near(x, r.x1.get())) && (near(y, r.y0.get()) || near(y, r.y1.get()))
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

    /// Each subpath of straight horizontal and vertical sides.
    fn polygons(&self) -> Vec<Polygon> {
        let mut subpaths: BTreeMap<usize, Vec<(PointPt, PointPt)>> = BTreeMap::new();
        for &(a, b, n) in &self.segments {
            subpaths.entry(n).or_default().push((a, b));
        }
        subpaths
            .into_iter()
            .filter(|(n, sides)| !self.curved.contains(n) && sides.iter().all(|&(a, b)| on_axis(a, b)))
            .filter_map(|(n, sides)| {
                let points = || sides.iter().flat_map(|&(a, b)| [a, b]);
                let rect = points()
                    .map(|p| RectPt::new(p.x, p.y, p.x, p.y))
                    .reduce(|a, b| a.union(&b))?;
                let end = sides.last().map(|&(_, b)| b);
                let closes = match (self.starts.get(&n), end) {
                    (Some(&start), Some(end)) => on_axis(end, start),
                    _ => true,
                };
                let rectangle = closes && points().all(|p| at_corner(p, &rect));
                Some(Polygon { rect, rectangle })
            })
            .collect()
    }
}

/// Which of `boxes` hold, or lie in, another of them: a filled path drawn as boxes inside
/// boxes may be a frame with a hole (`f*`). A sweep in x, so boxes side by side (a page of
/// redactions, a QR code) cost little.
fn nested(boxes: &[RectPt]) -> Vec<bool> {
    let holds = |a: &RectPt, b: &RectPt| a.x0 <= b.x0 && a.y0 <= b.y0 && b.x1 <= a.x1 && b.y1 <= a.y1;
    let mut order: Vec<usize> = (0..boxes.len()).collect();
    order.sort_by(|&i, &j| {
        let x0 = |k: usize| boxes.get(k).map_or(0.0, |b| b.x0.get());
        x0(i).total_cmp(&x0(j))
    });
    let mut out = vec![false; boxes.len()];
    for (at, &i) in order.iter().enumerate() {
        let Some(a) = boxes.get(i) else { continue };
        for &j in order.iter().skip(at + 1) {
            let Some(b) = boxes.get(j) else { continue };
            if b.x0 > a.x1 {
                break;
            }
            if holds(a, b) || holds(b, a) {
                for k in [i, j] {
                    if let Some(flag) = out.get_mut(k) {
                        *flag = true;
                    }
                }
            }
        }
    }
    out
}

/// A content visitor that gathers a page's rules and covers.
pub(crate) struct Collector<'f> {
    frame: &'f PageFrame,
    /// Maps the content's default user space to the page's (an annotation's appearance
    /// matrix; the identity for the page's own content).
    outer: Matrix,
    /// What the content may paint on: the page, or an annotation's `/Rect` on it.
    bounds: RectPt,
    /// The `after` of every cover: `None` counts paints, `Some` gives every cover that value
    /// (annotations, drawn after the whole page).
    fixed_after: Option<u32>,
    /// Whether the table rules are wanted.
    rules: bool,
    edges: Edges,
    covers: Vec<Cover>,
    /// The images' covers, kept apart until [`Collector::finish`] knows whether the images
    /// together are a scan.
    image_covers: Vec<Cover>,
    /// The page area the images paint (overlaps counted twice).
    image_area: f64,
    path: Path,
    paints: u32,
    /// For each open marked-content sequence, whether it is optional content (`/OC`).
    marked: Vec<bool>,
    /// For each open form, whether nothing in it may cover: it has `/OC`, or a `/Group` drawn
    /// while the state wasn't opaque.
    forms: Vec<bool>,
}

impl<'f> Collector<'f> {
    /// A collector for a page's own content, gathering its table rules when `rules` is set.
    pub(crate) fn new(frame: &'f PageFrame, rules: bool) -> Self {
        Self {
            frame,
            outer: Matrix::IDENTITY,
            bounds: RectPt::new(Pt::ZERO, Pt::ZERO, frame.width(), frame.height()),
            fixed_after: None,
            rules,
            edges: Edges::default(),
            covers: Vec::new(),
            image_covers: Vec::new(),
            image_area: 0.0,
            path: Path::default(),
            paints: 0,
            marked: Vec::new(),
            forms: Vec::new(),
        }
    }

    /// A collector for an annotation's appearance: content mapped by `outer` and kept inside
    /// `bounds`, every cover drawn after the page.
    pub(crate) fn appearance(frame: &'f PageFrame, outer: Matrix, bounds: RectPt) -> Self {
        let mut collector = Self::new(frame, false);
        collector.outer = outer;
        collector.bounds =
            intersect(&bounds, &collector.bounds).unwrap_or(RectPt::new(Pt::ZERO, Pt::ZERO, Pt::ZERO, Pt::ZERO));
        collector.fixed_after = Some(u32::MAX);
        collector
    }

    /// The rules and the covers. Images that together paint at least 90% of the page are a
    /// scan, whose text layer they don't cover, even when the scan is stored as strips.
    pub(crate) fn finish(mut self) -> (Edges, Vec<Cover>) {
        let page = area(&RectPt::new(
            Pt::ZERO,
            Pt::ZERO,
            self.frame.width(),
            self.frame.height(),
        ));
        if self.image_area < SCAN_SHARE * page {
            self.covers.append(&mut self.image_covers);
        }
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

    /// `rect` as a cover painted after `after` paints, cut to the clip and the bounds; `None`
    /// when nothing is left.
    fn cut(&self, rect: RectPt, after: u32, state: &GraphicsState) -> Option<Cover> {
        let mut rect = intersect(&rect, &self.bounds)?;
        if let Some(clip) = state.clip_box {
            rect = intersect(&rect, &self.page_rect(&clip))?;
        }
        Some(Cover {
            rect,
            after: self.fixed_after.unwrap_or(after),
        })
    }

    /// The covers of a filled path: its rectangles and the subpaths that fill exactly their
    /// box. A box holding another box of the same path, or lying in one, covers nothing.
    fn fill_covers(&mut self, polygons: &[Polygon], after: u32, state: &GraphicsState) {
        let mut boxes: Vec<RectPt> = self.path.rects.clone();
        boxes.extend(polygons.iter().filter(|p| p.rectangle).map(|p| p.rect));
        let nested = nested(&boxes);
        for (rect, _) in boxes.into_iter().zip(nested).filter(|(_, n)| !n) {
            if let Some(cover) = self.cut(rect, after, state) {
                self.covers.push(cover);
            }
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
        Some(RectPt::new(o.x, o.y, far.x, far.y))
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
                self.path.curved.insert(self.path.subpath);
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
        let polygons = if filled { self.path.polygons() } else { Vec::new() };
        if self.rules && stroked {
            for &(a, b, _) in &self.path.segments {
                self.edges.add_segment(a, b);
            }
            for r in &self.path.rects {
                self.edges.add_rect_outline(r);
            }
        } else if self.rules && filled {
            for r in &self.path.rects {
                self.edges.add_filled_rect(r);
            }
            for p in &polygons {
                self.edges.add_filled_rect(&p.rect);
            }
        }
        // A pattern may paint only part of its area (hatching, dots).
        let solid = !matches!(paint.state.fill.space, ColorSpace::Pattern);
        if filled && solid && !self.hidden() && opaque(paint.state, paint.state.fill.approx_rgb()) {
            self.fill_covers(&polygons, after, paint.state);
        }
        self.path = Path::default();
    }

    fn image(&mut self, image: &ImageDraw<'_>) {
        let after = self.count();
        if self.hidden() {
            return;
        }
        let Some(rect) = self.image_cover(image) else {
            return;
        };
        if let Some(cover) = self.cut(rect, after, image.state) {
            self.image_area += area(&cover.rect);
            self.image_covers.push(cover);
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
