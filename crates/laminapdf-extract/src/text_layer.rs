//! Whether a page's text is the page's text: the measures from its runs, glyphs, images and
//! paths, and the status they give, so a pipeline can send only the pages that need it to OCR.

use laminapdf_model::{Pt, RectPt};
use laminapdf_text::PageText;

use crate::marks::Drawn;
use crate::model::{TextLayer, TextLayerStatus};
use crate::options::TextLayerThresholds;
use crate::text_fix::visible_chars;
use crate::visibility::{Backdrops, Look, flags};

/// The grid the image share is measured on, per side: deterministic, and overlapping images
/// count once.
const GRID: u32 = 32;
/// Shares are rounded to this, as ADR 0030 §7 rounds lengths.
const ROUND: f64 = 1000.0;

/// The measures of a page whose text is `text` and whose painting is `drawn`, and its status
/// under `thresholds`.
pub(crate) fn measure(
    text: &PageText,
    backdrops: &Backdrops,
    drawn: &Drawn,
    thresholds: &TextLayerThresholds,
) -> TextLayer {
    let frame = &text.frame;
    let (width, height, unit) = (frame.width(), frame.height(), frame.user_unit());
    let (mut chars, mut invisible_chars, mut glyphs, mut unmapped) = (0, 0, 0, 0);
    // Text left out as off the page or tiny is still text: the page isn't blank.
    let mut other_chars = 0;
    for run in &text.runs {
        let size = Pt::new(run.font_size * unit);
        let f = flags(&Look::of(run), &run.bounds, size, width, height, backdrops);
        if f.invisible {
            invisible_chars += visible_chars(&run.text);
            continue;
        }
        if f.off_page || f.tiny {
            other_chars += visible_chars(&run.text);
            continue;
        }
        chars += visible_chars(&run.text);
        let run_glyphs = text.glyphs.get(run.glyphs.clone()).unwrap_or_default();
        glyphs += run_glyphs.len();
        unmapped += run_glyphs.iter().filter(|g| !g.mapped).count();
    }
    let mut layer = TextLayer {
        status: TextLayerStatus::Native,
        chars,
        invisible_chars,
        unmapped_share: share(unmapped, glyphs),
        images: drawn.images,
        image_share: image_share(&drawn.image_boxes, width, height),
        paths: drawn.paths,
    };
    layer.status = classify(&layer, other_chars > 0, thresholds);
    layer
}

/// `part / whole` rounded to 1/1000, at most 1; 0 when `whole` is 0. Integer arithmetic, so
/// any count gives the right share.
fn share(part: usize, whole: usize) -> f64 {
    let (Ok(part), Ok(whole)) = (u128::try_from(part), u128::try_from(whole)) else {
        return 0.0;
    };
    if whole == 0 {
        return 0.0;
    }
    let thousandths = (part.min(whole) * 1000 + whole / 2) / whole;
    u16::try_from(thousandths).map_or(1.0, |t| f64::from(t) / ROUND)
}

/// The share of a `width` × `height` page that `images` cover: the grid cells whose centre lies
/// in an image's box. Each image marks only the cells its box reaches.
fn image_share(images: &[RectPt], width: Pt, height: Pt) -> f64 {
    let (w, h) = (width.get(), height.get());
    if images.is_empty() || w <= 0.0 || h <= 0.0 {
        return 0.0;
    }
    let side = f64::from(GRID);
    // The cells whose centre `(i + 0.5) × step` lies in [lo, hi): from the first centre at or
    // past `lo` to the first at or past `hi`.
    let cells = |lo: f64, hi: f64, step: f64| {
        let at_or_past = |v: f64| (0..GRID).find(|&i| (f64::from(i) + 0.5) * step >= v).unwrap_or(GRID);
        (at_or_past(lo), at_or_past(hi))
    };
    let mut covered = vec![false; usize::try_from(GRID * GRID).unwrap_or(0)];
    for b in images {
        let (c0, c1) = cells(b.x0.get(), b.x1.get(), w / side);
        let (r0, r1) = cells(b.y0.get(), b.y1.get(), h / side);
        for row in r0..r1 {
            for col in c0..c1 {
                if let Some(cell) = usize::try_from(row * GRID + col).ok().and_then(|i| covered.get_mut(i)) {
                    *cell = true;
                }
            }
        }
    }
    share(covered.iter().filter(|&&c| c).count(), covered.len())
}

/// Whether `value`, a share from 0 to 1, is at least `thousandths` / 1000.
fn at_least(value: f64, thousandths: u16) -> bool {
    (value * ROUND).round() >= f64::from(thousandths)
}

/// The first status whose rule `layer` meets. `other_text`: the page has text left out of
/// `chars` as off the page or tiny, so it isn't blank.
fn classify(layer: &TextLayer, other_text: bool, t: &TextLayerThresholds) -> TextLayerStatus {
    let image_page = at_least(layer.image_share, t.image_page);
    let no_text = layer.chars == 0 && layer.invisible_chars == 0 && !other_text;
    if no_text && layer.images == 0 && layer.paths < t.drawn_paths {
        TextLayerStatus::Empty
    } else if image_page && layer.invisible_chars > layer.chars {
        TextLayerStatus::OcrLayer
    } else if layer.chars == 0
        || (image_page && layer.chars <= t.few_chars)
        || at_least(layer.unmapped_share, t.unmapped_needs_ocr)
    {
        TextLayerStatus::NeedsOcr
    } else if at_least(layer.unmapped_share, t.unmapped_mixed) || at_least(layer.image_share, t.image_mixed) {
        TextLayerStatus::Mixed
    } else {
        TextLayerStatus::Native
    }
}

#[cfg(test)]
mod tests {
    use laminapdf_model::{Pt, RectPt};

    use super::{classify, image_share, share};
    use crate::model::{TextLayer, TextLayerStatus};
    use crate::options::TextLayerThresholds;

    fn layer(chars: usize, invisible_chars: usize, unmapped_share: f64, image_share: f64, paths: usize) -> TextLayer {
        TextLayer {
            status: TextLayerStatus::Native,
            chars,
            invisible_chars,
            unmapped_share,
            images: usize::from(image_share > 0.0),
            image_share,
            paths,
        }
    }

    fn status(l: &TextLayer) -> TextLayerStatus {
        classify(l, false, &TextLayerThresholds::default())
    }

    #[test]
    fn a_blank_page_is_empty_until_it_paints_enough_paths() {
        assert_eq!(status(&layer(0, 0, 0.0, 0.0, 15)), TextLayerStatus::Empty);
        assert_eq!(status(&layer(0, 0, 0.0, 0.0, 16)), TextLayerStatus::NeedsOcr);
    }

    #[test]
    fn a_page_whose_only_text_is_tiny_or_off_the_page_is_not_blank() {
        let l = layer(0, 0, 0.0, 0.0, 0);
        assert_eq!(
            classify(&l, true, &TextLayerThresholds::default()),
            TextLayerStatus::NeedsOcr
        );
    }

    #[test]
    fn an_image_page_with_more_invisible_than_visible_text_has_an_ocr_layer() {
        assert_eq!(status(&layer(10, 300, 0.0, 0.5, 0)), TextLayerStatus::OcrLayer);
        assert_eq!(status(&layer(10, 300, 0.0, 0.499, 0)), TextLayerStatus::Mixed);
        assert_eq!(status(&layer(300, 300, 0.0, 0.9, 0)), TextLayerStatus::Mixed);
    }

    #[test]
    fn an_image_page_with_a_stamp_of_text_needs_ocr() {
        assert_eq!(status(&layer(200, 0, 0.0, 0.5, 0)), TextLayerStatus::NeedsOcr);
        assert_eq!(status(&layer(201, 0, 0.0, 0.5, 0)), TextLayerStatus::Mixed);
        assert_eq!(status(&layer(0, 0, 0.0, 1.0, 0)), TextLayerStatus::NeedsOcr);
    }

    #[test]
    fn unmapped_glyphs_make_a_page_mixed_then_need_ocr() {
        assert_eq!(status(&layer(500, 0, 0.099, 0.0, 0)), TextLayerStatus::Native);
        assert_eq!(status(&layer(500, 0, 0.1, 0.0, 0)), TextLayerStatus::Mixed);
        assert_eq!(status(&layer(500, 0, 0.5, 0.0, 0)), TextLayerStatus::NeedsOcr);
    }

    #[test]
    fn a_quarter_of_the_page_in_images_makes_a_page_mixed() {
        assert_eq!(status(&layer(500, 0, 0.0, 0.249, 0)), TextLayerStatus::Native);
        assert_eq!(status(&layer(500, 0, 0.0, 0.25, 0)), TextLayerStatus::Mixed);
    }

    #[test]
    fn the_image_share_counts_overlapping_images_once() {
        let pt = Pt::new;
        let left = RectPt::new(pt(0.0), pt(0.0), pt(50.0), pt(100.0));
        let same = RectPt::new(pt(0.0), pt(0.0), pt(50.0), pt(100.0));
        assert!((image_share(&[left, same], pt(100.0), pt(100.0)) - 0.5).abs() < 1e-9);
        assert!(image_share(&[], pt(100.0), pt(100.0)).abs() < 1e-9);
    }

    #[test]
    fn shares_round_to_a_thousandth() {
        assert!((share(1, 3) - 0.333).abs() < 1e-9);
        assert!((share(2, 3) - 0.667).abs() < 1e-9);
        assert!(share(0, 0).abs() < 1e-9);
        assert!((share(usize::MAX, usize::MAX) - 1.0).abs() < 1e-9);
    }

    #[test]
    fn an_image_covers_the_cells_whose_centres_it_holds() {
        let pt = Pt::new;
        // A quarter of a 320 × 320 page: the cells centred at 5, 15, …, 155.
        let quarter = RectPt::new(pt(0.0), pt(0.0), pt(160.0), pt(160.0));
        assert!((image_share(&[quarter], pt(320.0), pt(320.0)) - 0.25).abs() < 1e-9);
        let whole = RectPt::new(pt(-50.0), pt(-50.0), pt(400.0), pt(400.0));
        assert!((image_share(&[whole], pt(320.0), pt(320.0)) - 1.0).abs() < 1e-9);
    }
}
