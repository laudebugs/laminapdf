//! A page's painting, seen in the same walk of the content as its text: the table rules (when
//! tables are looked for), what is painted under the text, and the images and paths that tell a
//! scan or outlined text from a page of text.

use laminapdf_content::{ImageDraw, Matrix, Paint, PathPaint, PathSegment, ShadingPaint, Visitor, transformed_box};
use laminapdf_model::{PageFrame, Pt, RectPt, UserSpaceRect};

use crate::tables::{Collector, Edges};
use crate::visibility::Backdrops;

/// What a page draws besides its text, as the text-layer status measures it.
#[derive(Debug, Clone, Default)]
pub(crate) struct Drawn {
    /// Each image's box in page space, clipped to the page.
    pub(crate) images: Vec<RectPt>,
    /// How many paths it fills or strokes.
    pub(crate) paths: usize,
}

/// The visitor [`laminapdf_text::TextExtractor::page_with`] is given.
pub(crate) struct Marks<'f> {
    rules: Option<Collector<'f>>,
    backdrops: Backdrops,
    frame: PageFrame,
    drawn: Drawn,
}

impl<'f> Marks<'f> {
    /// `tables`: whether to collect the table rules.
    pub(crate) fn new(frame: &'f PageFrame, tables: bool) -> Self {
        Self {
            rules: tables.then(|| Collector::new(frame)),
            backdrops: Backdrops::new(frame),
            frame: *frame,
            drawn: Drawn::default(),
        }
    }

    /// The table rules (when collected), the backdrops and what was drawn.
    pub(crate) fn finish(self) -> (Option<Edges>, Backdrops, Drawn) {
        (self.rules.map(Collector::finish), self.backdrops, self.drawn)
    }
}

/// `rect` clipped to a page `width` × `height`; `None` when nothing is left.
fn clip_to_page(rect: &RectPt, width: Pt, height: Pt) -> Option<RectPt> {
    let x0 = Pt::new(rect.x0.get().max(0.0));
    let y0 = Pt::new(rect.y0.get().max(0.0));
    let x1 = Pt::new(rect.x1.get().min(width.get()));
    let y1 = Pt::new(rect.y1.get().min(height.get()));
    (x1 > x0 && y1 > y0).then(|| RectPt::new(x0, y0, x1, y1))
}

/// The page-space box of an image drawn with `ctm`, which maps the unit square onto the page
/// (§8.9.4), clipped to the page.
fn image_box(frame: &PageFrame, ctm: &Matrix) -> Option<RectPt> {
    let unit = UserSpaceRect::from_corners(0.0, 0.0, 1.0, 1.0);
    let page = frame.rect_to_page(&transformed_box(&unit, ctm));
    clip_to_page(&page, frame.width(), frame.height())
}

impl Visitor for Marks<'_> {
    fn path_segment(&mut self, segment: &PathSegment, ctm: &Matrix) {
        if let Some(rules) = &mut self.rules {
            rules.path_segment(segment, ctm);
        }
    }

    fn paint_path(&mut self, paint: &PathPaint<'_>) {
        if let Some(rules) = &mut self.rules {
            rules.paint_path(paint);
        }
        self.backdrops.paint_path(paint);
        if matches!(
            paint.paint,
            Paint::Stroke { .. } | Paint::Fill(_) | Paint::FillStroke { .. }
        ) {
            self.drawn.paths += 1;
        }
    }

    fn image(&mut self, image: &ImageDraw<'_>) {
        self.backdrops.image(image);
        if let Some(bbox) = image_box(&self.frame, &image.state.ctm) {
            self.drawn.images.push(bbox);
        }
    }

    fn paint_shading(&mut self, shading: &ShadingPaint<'_>) {
        self.backdrops.paint_shading(shading);
    }
}

#[cfg(test)]
mod tests {
    use laminapdf_content::Matrix;
    use laminapdf_model::{PageFrame, Pt, RectPt, Rotation, UserSpaceRect};

    use super::{clip_to_page, image_box};

    fn letter(rotate: Rotation) -> PageFrame {
        PageFrame::new(UserSpaceRect::from_corners(0.0, 0.0, 612.0, 792.0), rotate, 1.0)
    }

    fn close(rect: &RectPt, expected: [f64; 4]) -> bool {
        let got = [rect.x0.get(), rect.y0.get(), rect.x1.get(), rect.y1.get()];
        got.iter().zip(expected).all(|(a, b)| (a - b).abs() < 1e-6)
    }

    #[test]
    fn a_scaled_image_maps_to_its_box_in_top_left_page_space() {
        // 100 × 50 pt at (72, 600) in user space: top-left y = 792 − 650 = 142.
        let ctm = Matrix::new([100.0, 0.0, 0.0, 50.0, 72.0, 600.0]);
        let bbox = image_box(&letter(Rotation::None), &ctm).unwrap_or_default();
        assert!(close(&bbox, [72.0, 142.0, 172.0, 192.0]), "{bbox:?}");
    }

    #[test]
    fn a_quarter_turned_page_swaps_the_box() {
        let ctm = Matrix::new([100.0, 0.0, 0.0, 50.0, 0.0, 0.0]);
        let bbox = image_box(&letter(Rotation::Clockwise90), &ctm).unwrap_or_default();
        // On a page turned a quarter clockwise, the image's 100 pt run up the left side.
        assert!(close(&bbox, [0.0, 0.0, 50.0, 100.0]), "{bbox:?}");
    }

    #[test]
    fn a_box_is_clipped_to_the_page_and_dropped_when_nothing_is_left() {
        let pt = Pt::new;
        let over = RectPt::new(pt(-10.0), pt(-10.0), pt(50.0), pt(50.0));
        let clipped = clip_to_page(&over, pt(100.0), pt(100.0)).unwrap_or_default();
        assert!(close(&clipped, [0.0, 0.0, 50.0, 50.0]));
        let outside = RectPt::new(pt(200.0), pt(0.0), pt(300.0), pt(50.0));
        assert_eq!(clip_to_page(&outside, pt(100.0), pt(100.0)), None);
    }
}
