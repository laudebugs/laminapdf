//! Hidden text ([ADR 0030] §4): the flags a run gets from how and where it is drawn. Clipped and
//! low-contrast text need the clip and the fill colour, which the text runs don't carry yet;
//! those two flags stay unset until they do.
//!
//! [ADR 0030]: https://github.com/laudebugs/laminapdf/blob/main/docs/decisions/0030-extraction-output-and-model-plugins.md

use laminapdf_model::{Pt, RectPt};

use crate::model::Flags;

/// Text drawn smaller than this, in points, is `tiny`.
pub(crate) const TINY: f64 = 1.0;

/// The flags of a run whose box is `bbox` and size `size`, on a page of `width` × `height`.
pub(crate) fn flags(invisible: bool, bbox: &RectPt, size: Pt, width: Pt, height: Pt) -> Flags {
    let outside = bbox.x1 <= Pt::ZERO || bbox.y1 <= Pt::ZERO || bbox.x0 >= width || bbox.y0 >= height;
    Flags {
        invisible,
        off_page: outside,
        tiny: size.get() < TINY,
        ..Flags::NONE
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

    use super::{flags, normalised};

    fn rect(x: f64, y: f64, w: f64, h: f64) -> RectPt {
        RectPt::from_points(x, y, w, h)
    }

    const W: Pt = Pt::new(600.0);
    const H: Pt = Pt::new(800.0);

    #[test]
    fn text_on_the_page_at_a_readable_size_is_visible() {
        assert!(!flags(false, &rect(10.0, 10.0, 50.0, 12.0), Pt::new(12.0), W, H).any());
    }

    #[test]
    fn text_wholly_outside_the_page_is_off_page_and_text_across_its_edge_is_not() {
        assert!(flags(false, &rect(-80.0, 10.0, 50.0, 12.0), Pt::new(12.0), W, H).off_page);
        assert!(flags(false, &rect(10.0, 900.0, 50.0, 12.0), Pt::new(12.0), W, H).off_page);
        assert!(!flags(false, &rect(-20.0, 10.0, 50.0, 12.0), Pt::new(12.0), W, H).off_page);
    }

    #[test]
    fn text_below_one_point_is_tiny() {
        assert!(flags(false, &rect(10.0, 10.0, 2.0, 0.5), Pt::new(0.5), W, H).tiny);
        assert!(!flags(false, &rect(10.0, 10.0, 2.0, 1.0), Pt::new(1.0), W, H).tiny);
    }

    #[test]
    fn render_mode_invisibility_is_kept() {
        assert!(flags(true, &rect(10.0, 10.0, 50.0, 12.0), Pt::new(12.0), W, H).invisible);
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
