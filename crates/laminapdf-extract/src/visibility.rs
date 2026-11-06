//! Hidden text ([ADR 0030] §4): the flags a run gets from how and where it is drawn. The clip and
//! the colour are approximations, and both lean towards keeping text: the clip is its bounding
//! box, so text inside a curved clip's box counts as visible, and near-white text counts as
//! hidden only when nothing that isn't white is painted where it is, before or after it.
//!
//! [ADR 0030]: https://github.com/laudebugs/laminapdf/blob/main/docs/decisions/0030-extraction-output-and-model-plugins.md

use laminapdf_content::{ImageDraw, Matrix, Paint, PathPaint, PathSegment, ShadingPaint, Visitor, transformed_box};
use laminapdf_model::{PageFrame, Pt, RectPt, UserSpaceRect};
use laminapdf_text::PageTextRun;

use crate::model::Flags;

/// Text drawn smaller than this, in points, is `tiny`.
pub(crate) const TINY: f64 = 1.0;

/// A colour whose every RGB component is at least this is near white.
const NEAR_WHITE: f64 = 0.9;

/// Whether `rgb` is near white.
fn near_white(rgb: [f64; 3]) -> bool {
    rgb.iter().all(|&c| c >= NEAR_WHITE)
}

/// What a page paints that text can stand on: the boxes, in page space, of fills that aren't
/// near white, images and shadings. Collected in the same walk of the content as the text.
#[derive(Debug, Clone, Default)]
pub(crate) struct Backdrops {
    frame: Option<PageFrame>,
    /// The current path's box in default user space.
    path: Option<UserSpaceRect>,
    boxes: Vec<RectPt>,
}

impl Backdrops {
    pub(crate) fn new(frame: &PageFrame) -> Self {
        Self {
            frame: Some(*frame),
            ..Self::default()
        }
    }

    fn add(&mut self, rect: UserSpaceRect) {
        if let Some(frame) = &self.frame {
            self.boxes.push(frame.rect_to_page(&rect));
        }
    }

    /// Whether anything painted overlaps `bbox`.
    pub(crate) fn under(&self, bbox: &RectPt) -> bool {
        self.boxes.iter().any(|b| overlaps(b, bbox))
    }
}

fn overlaps(a: &RectPt, b: &RectPt) -> bool {
    a.x0 < b.x1 && b.x0 < a.x1 && a.y0 < b.y1 && b.y0 < a.y1
}

impl Visitor for Backdrops {
    fn path_segment(&mut self, segment: &PathSegment, ctm: &Matrix) {
        let points: &[(f64, f64)] = &match *segment {
            PathSegment::MoveTo { x, y } | PathSegment::LineTo { x, y } => vec![(x, y)],
            PathSegment::Rect { x, y, width, height } => vec![(x, y), (x + width, y + height)],
            PathSegment::CurveTo { c1, c2, end } => vec![c1, c2, end],
            _ => Vec::new(),
        };
        for &(x, y) in points {
            let (x, y) = ctm.apply(x, y);
            let point = UserSpaceRect::from_corners(x, y, x, y);
            self.path = Some(self.path.map_or(point, |b| {
                UserSpaceRect::from_corners(b.x0.min(x), b.y0.min(y), b.x1.max(x), b.y1.max(y))
            }));
        }
    }

    fn paint_path(&mut self, paint: &PathPaint<'_>) {
        let path = self.path.take();
        let filled = matches!(paint.paint, Paint::Fill(_) | Paint::FillStroke { .. });
        // A pattern or a space read no other way counts as dark: keep the text.
        let light = paint.state.fill.approx_rgb().is_some_and(near_white);
        if let (true, false, Some(path)) = (filled, light, path) {
            self.add(path);
        }
    }

    fn image(&mut self, image: &ImageDraw<'_>) {
        // The CTM maps the unit square onto the page (§8.9.4).
        self.add(transformed_box(
            &UserSpaceRect::from_corners(0.0, 0.0, 1.0, 1.0),
            &image.state.ctm,
        ));
    }

    fn paint_shading(&mut self, shading: &ShadingPaint<'_>) {
        // `sh` paints the whole clip; with no clip, the whole page.
        match shading.state.clip_box {
            Some(clip) => self.add(clip),
            None => {
                if let Some(frame) = &self.frame {
                    let page = RectPt::new(Pt::ZERO, Pt::ZERO, frame.width(), frame.height());
                    self.boxes.push(page);
                }
            }
        }
    }
}

/// How a run is drawn, as far as the flags care.
#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct Look {
    pub(crate) invisible: bool,
    pub(crate) clip: Option<RectPt>,
    pub(crate) color: Option<[f64; 3]>,
}

impl Look {
    pub(crate) const fn of(run: &PageTextRun) -> Self {
        Self {
            invisible: run.invisible,
            clip: run.clip,
            color: run.color,
        }
    }
}

/// The flags of a run drawn as `run`, whose box is `bbox` and size `size`, on a page of
/// `width` × `height` with `backdrops` painted on it.
pub(crate) fn flags(run: &Look, bbox: &RectPt, size: Pt, width: Pt, height: Pt, backdrops: &Backdrops) -> Flags {
    let outside = bbox.x1 <= Pt::ZERO || bbox.y1 <= Pt::ZERO || bbox.x0 >= width || bbox.y0 >= height;
    // Wholly outside the clip's box, or inside a clip that left nothing.
    let clipped = run.clip.is_some_and(|clip| !overlaps(&clip, bbox));
    let low_contrast = run.color.is_some_and(near_white) && !backdrops.under(bbox);
    Flags {
        invisible: run.invisible,
        off_page: outside,
        tiny: size.get() < TINY,
        clipped,
        low_contrast,
    }
}

/// `bbox` divided by the page size and clamped to [0, 1], `None` when it lies wholly outside
/// the page (ADR 0027 §4).
pub(crate) fn normalised(bbox: &RectPt, width: Pt, height: Pt) -> Option<[f64; 4]> {
    let (w, h) = (width.get(), height.get());
    if w <= 0.0 || h <= 0.0 {
        return None;
    }
    let x0 = (bbox.x0.get() / w).clamp(0.0, 1.0);
    let y0 = (bbox.y0.get() / h).clamp(0.0, 1.0);
    let x1 = (bbox.x1.get() / w).clamp(0.0, 1.0);
    let y1 = (bbox.y1.get() / h).clamp(0.0, 1.0);
    (x1 > x0 && y1 > y0).then_some([x0, y0, x1 - x0, y1 - y0])
}

#[cfg(test)]
mod tests {
    use laminapdf_model::{Pt, RectPt};

    use super::{Backdrops, Look, flags, normalised};

    fn rect(x: f64, y: f64, w: f64, h: f64) -> RectPt {
        RectPt::from_points(x, y, w, h)
    }

    const W: Pt = Pt::new(600.0);
    const H: Pt = Pt::new(800.0);

    #[test]
    fn text_on_the_page_at_a_readable_size_is_visible() {
        assert!(
            !flags(
                &Look::default(),
                &rect(10.0, 10.0, 50.0, 12.0),
                Pt::new(12.0),
                W,
                H,
                &Backdrops::default()
            )
            .any()
        );
    }

    #[test]
    fn text_wholly_outside_the_page_is_off_page_and_text_across_its_edge_is_not() {
        assert!(
            flags(
                &Look::default(),
                &rect(-80.0, 10.0, 50.0, 12.0),
                Pt::new(12.0),
                W,
                H,
                &Backdrops::default()
            )
            .off_page
        );
        assert!(
            flags(
                &Look::default(),
                &rect(10.0, 900.0, 50.0, 12.0),
                Pt::new(12.0),
                W,
                H,
                &Backdrops::default()
            )
            .off_page
        );
        assert!(
            !flags(
                &Look::default(),
                &rect(-20.0, 10.0, 50.0, 12.0),
                Pt::new(12.0),
                W,
                H,
                &Backdrops::default()
            )
            .off_page
        );
    }

    #[test]
    fn text_below_one_point_is_tiny() {
        assert!(
            flags(
                &Look::default(),
                &rect(10.0, 10.0, 2.0, 0.5),
                Pt::new(0.5),
                W,
                H,
                &Backdrops::default()
            )
            .tiny
        );
        assert!(
            !flags(
                &Look::default(),
                &rect(10.0, 10.0, 2.0, 1.0),
                Pt::new(1.0),
                W,
                H,
                &Backdrops::default()
            )
            .tiny
        );
    }

    #[test]
    fn render_mode_invisibility_is_kept() {
        assert!(
            flags(
                &Look {
                    invisible: true,
                    ..Look::default()
                },
                &rect(10.0, 10.0, 50.0, 12.0),
                Pt::new(12.0),
                W,
                H,
                &Backdrops::default()
            )
            .invisible
        );
    }

    #[test]
    fn text_wholly_outside_its_clip_is_clipped_and_text_across_its_edge_is_not() {
        let clip = Some(rect(0.0, 0.0, 100.0, 100.0));
        let look = Look {
            clip,
            ..Look::default()
        };
        let none = Backdrops::default();
        assert!(flags(&look, &rect(200.0, 10.0, 50.0, 12.0), Pt::new(12.0), W, H, &none).clipped);
        assert!(!flags(&look, &rect(90.0, 10.0, 50.0, 12.0), Pt::new(12.0), W, H, &none).clipped);
    }

    #[test]
    fn near_white_text_is_low_contrast_only_with_nothing_painted_under_it() {
        let white = Look {
            color: Some([1.0; 3]),
            ..Look::default()
        };
        let text = rect(10.0, 10.0, 50.0, 12.0);
        let mut painted = Backdrops::default();
        assert!(flags(&white, &text, Pt::new(12.0), W, H, &painted).low_contrast);
        painted.boxes.push(rect(0.0, 0.0, 200.0, 40.0));
        assert!(!flags(&white, &text, Pt::new(12.0), W, H, &painted).low_contrast);
        let grey = Look {
            color: Some([0.5; 3]),
            ..Look::default()
        };
        assert!(!flags(&grey, &text, Pt::new(12.0), W, H, &Backdrops::default()).low_contrast);
    }

    #[test]
    fn normalised_boxes_are_clamped_and_absent_outside_the_page() {
        assert_eq!(
            normalised(&rect(60.0, 80.0, 300.0, 400.0), W, H),
            Some([0.1, 0.1, 0.5, 0.5])
        );
        assert_eq!(
            normalised(&rect(-60.0, 0.0, 120.0, 800.0), W, H),
            Some([0.0, 0.0, 0.1, 1.0])
        );
        assert_eq!(normalised(&rect(700.0, 0.0, 10.0, 10.0), W, H), None);
    }
}
