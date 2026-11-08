//! Heron's outputs as regions: its 17 labels mapped onto [`RegionLabel`], low scores dropped,
//! near-duplicate boxes merged, boxes moved from pixels into page space.

use laminapdf_extract::{LayoutRegion, PageRaster, RegionLabel};

/// The model's outputs for one image: 300 candidates each.
pub(crate) struct Detections<'a> {
    pub(crate) labels: &'a [i64],
    /// `x0, y0, x1, y1` per candidate, in the original image's pixels.
    pub(crate) boxes: &'a [f32],
    pub(crate) scores: &'a [f32],
}

/// A box of the same page this much overlapped (intersection over union) is the same region
/// read twice: Heron sometimes names one box with two labels.
const SAME_BOX: f64 = 0.9;

/// Heron's label `id` (its `config.json`'s `id2label`) as a [`RegionLabel`].
#[must_use]
pub fn label(id: i64) -> RegionLabel {
    match id {
        0 => RegionLabel::Caption,
        1 => RegionLabel::Footnote,
        2 => RegionLabel::Formula,
        3 => RegionLabel::ListItem,
        4 => RegionLabel::PageFooter,
        5 => RegionLabel::PageHeader,
        6 => RegionLabel::Picture,
        7 => RegionLabel::SectionHeader,
        8 => RegionLabel::Table,
        9 => RegionLabel::Text,
        10 => RegionLabel::Title,
        // document_index, code, checkbox_selected, checkbox_unselected, form, key_value_region.
        _ => RegionLabel::Other,
    }
}

fn iou(a: [f64; 4], b: [f64; 4]) -> f64 {
    let w = (a[2].min(b[2]) - a[0].max(b[0])).max(0.0);
    let h = (a[3].min(b[3]) - a[1].max(b[1])).max(0.0);
    let inter = w * h;
    let area = |r: [f64; 4]| (r[2] - r[0]).max(0.0) * (r[3] - r[1]).max(0.0);
    let union = area(a) + area(b) - inter;
    if union > 0.0 { inter / union } else { 0.0 }
}

/// The regions scoring at least `threshold`, best first, each box kept once.
pub(crate) fn regions(d: &Detections<'_>, page: &PageRaster, threshold: f32) -> Vec<LayoutRegion> {
    let mut found: Vec<(f32, i64, [f64; 4])> = d
        .scores
        .iter()
        .zip(d.labels)
        .zip(d.boxes.as_chunks::<4>().0)
        .filter(|((score, _), _)| **score >= threshold)
        .map(|((score, label), [x0, y0, x1, y1])| {
            (
                *score,
                *label,
                [f64::from(*x0), f64::from(*y0), f64::from(*x1), f64::from(*y1)],
            )
        })
        .collect();
    found.sort_by(|a, b| b.0.total_cmp(&a.0));
    let mut kept: Vec<(f32, i64, [f64; 4])> = Vec::new();
    for candidate in found {
        if kept.iter().all(|k| iou(k.2, candidate.2) < SAME_BOX) {
            kept.push(candidate);
        }
    }
    kept.into_iter()
        .map(|(score, id, [x0, y0, x1, y1])| {
            LayoutRegion::new(label(id), page.to_page(x0, y0, x1, y1), f64::from(score))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use laminapdf_extract::{PageRaster, RegionLabel};
    use laminapdf_model::{Pt, RectPt};

    use super::{Detections, regions};

    #[test]
    fn regions_should_drop_low_scores_and_keep_one_label_per_box() {
        // 1240 × 1754 pixels of an A4 page at 150 dpi.
        let page = PageRaster::new(
            1240,
            1754,
            150,
            vec![0; 1240 * 1754 * 4],
            RectPt::new(Pt::new(0.0), Pt::new(0.0), Pt::new(595.2), Pt::new(841.92)),
        )
        .unwrap();
        let d = Detections {
            labels: &[7, 10, 9, 8],
            boxes: &[
                116.0, 114.0, 359.0, 166.0, // section header
                116.0, 114.0, 359.0, 166.0, // the same box as a title, lower
                116.0, 308.0, 1124.0, 534.0, // text
                200.0, 600.0, 300.0, 700.0, // a table below the threshold
            ],
            scores: &[0.62, 0.59, 0.71, 0.2],
        };
        let out = regions(&d, &page, 0.3);
        let labels: Vec<RegionLabel> = out.iter().map(|r| r.label).collect();
        assert_eq!(labels, [RegionLabel::Text, RegionLabel::SectionHeader]);
        // 116 px at 150 dpi is 55.68 pt.
        assert!((out[1].bbox.x0.get() - 55.68).abs() < 0.01);
    }
}
