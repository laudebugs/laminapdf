//! What a layout model's regions and an OCR engine's words change on a page ([ADR 0030] §6):
//! words become pieces like the text layer's, and regions decide which lines may share a
//! block, the order of those blocks and what kind each is.
//!
//! [ADR 0030]: https://github.com/laudebugs/laminapdf/blob/main/docs/decisions/0030-extraction-output-and-model-plugins.md

use laminapdf_model::{Pt, RectPt};

use crate::lines::Piece;
use crate::lists::split_marker;
use crate::model::{BlockKind, DropReason, Flags, Span};
use crate::plugins::{LayoutRegion, OcrWord, RegionLabel};

/// The font name OCR text carries, since no font drew it.
pub(crate) const OCR_FONT: &str = "OCR";

/// The OCR engine's words as pieces and spans: each word its own piece, on a baseline at its
/// box's bottom, sized by its box's height. Empty words are skipped.
pub(crate) fn ocr_pieces(words: &[OcrWord]) -> (Vec<Piece>, Vec<Span>) {
    let mut pieces = Vec::with_capacity(words.len());
    let mut spans = Vec::with_capacity(words.len());
    for word in words.iter().filter(|w| !w.text.trim().is_empty() && w.bbox.is_finite()) {
        let size = Pt::new(word.bbox.height().get().max(1.0));
        pieces.push(Piece {
            text: word.text.clone(),
            bbox: word.bbox,
            baseline: word.bbox.y1,
            size,
            font: OCR_FONT.to_owned(),
            vertical: false,
            mcid: None,
            artifact: false,
        });
        spans.push(Span {
            text: word.text.clone(),
            bbox: word.bbox,
            baseline: word.bbox.y1,
            font: OCR_FONT.to_owned(),
            size,
            flags: Flags::NONE,
            ocr: Some(word.confidence.clamp(0.0, 1.0)),
        });
    }
    (pieces, spans)
}

fn centre(bbox: &RectPt) -> (f64, f64) {
    (
        f64::midpoint(bbox.x0.get(), bbox.x1.get()),
        f64::midpoint(bbox.y0.get(), bbox.y1.get()),
    )
}

fn holds(region: &RectPt, (x, y): (f64, f64)) -> bool {
    region.x0.get() <= x && x <= region.x1.get() && region.y0.get() <= y && y <= region.y1.get()
}

fn area(bbox: &RectPt) -> f64 {
    bbox.width().get() * bbox.height().get()
}

/// The region a box belongs to: of those holding its centre, the one with the highest score,
/// then the smallest (a list item inside a text region is the list item).
pub(crate) fn region_of(regions: &[LayoutRegion], bbox: &RectPt) -> Option<usize> {
    let point = centre(bbox);
    regions
        .iter()
        .enumerate()
        .filter(|(_, r)| r.bbox.is_finite() && holds(&r.bbox, point))
        .min_by(|(_, a), (_, b)| {
            b.score
                .total_cmp(&a.score)
                .then_with(|| area(&a.bbox).total_cmp(&area(&b.bbox)))
        })
        .map(|(i, _)| i)
}

/// Whether a box's centre lies in `region`.
pub(crate) fn inside(region: &RectPt, bbox: &RectPt) -> bool {
    holds(region, centre(bbox))
}

/// What a block in a region of `label` is, with `text` the block's text; `Err` with the reason
/// for a running header or footer, which leaves the blocks when furniture is dropped. A list
/// item's text loses its marker, as the rules' list items do.
pub(crate) fn kind_for(label: RegionLabel, text: &str) -> Result<(BlockKind, Option<String>), DropReason> {
    let kind = match label {
        RegionLabel::Title => BlockKind::Heading { level: 1 },
        RegionLabel::SectionHeader => BlockKind::Heading { level: 2 },
        RegionLabel::ListItem => {
            return Ok(match split_marker(text) {
                Some((marker, _, rest)) => (
                    BlockKind::ListItem {
                        marker: marker.to_owned(),
                        depth: 0,
                    },
                    Some(rest.to_owned()),
                ),
                None => (
                    BlockKind::ListItem {
                        marker: String::new(),
                        depth: 0,
                    },
                    None,
                ),
            });
        }
        RegionLabel::Caption => BlockKind::Caption,
        RegionLabel::Footnote => BlockKind::Footnote,
        RegionLabel::PageHeader => return Err(DropReason::RunningHeader),
        RegionLabel::PageFooter => return Err(DropReason::RunningFooter),
        RegionLabel::Text => BlockKind::Paragraph,
        RegionLabel::Table | RegionLabel::Picture | RegionLabel::Formula | RegionLabel::Other => BlockKind::Other,
    };
    Ok((kind, None))
}

#[cfg(test)]
mod tests {
    use laminapdf_model::{Pt, RectPt};

    use super::{kind_for, ocr_pieces, region_of};
    use crate::model::{BlockKind, DropReason};
    use crate::plugins::{LayoutRegion, OcrWord, RegionLabel};

    fn rect(x0: f64, y0: f64, x1: f64, y1: f64) -> RectPt {
        RectPt::new(Pt::new(x0), Pt::new(y0), Pt::new(x1), Pt::new(y1))
    }

    #[test]
    fn region_of_should_prefer_the_higher_score_then_the_smaller_region() {
        let regions = [
            LayoutRegion::new(RegionLabel::Text, rect(0.0, 0.0, 100.0, 100.0), 0.9),
            LayoutRegion::new(RegionLabel::ListItem, rect(10.0, 10.0, 50.0, 20.0), 0.9),
            LayoutRegion::new(RegionLabel::Title, rect(0.0, 0.0, 100.0, 100.0), 0.5),
        ];
        assert_eq!(region_of(&regions, &rect(12.0, 12.0, 30.0, 18.0)), Some(1));
        assert_eq!(region_of(&regions, &rect(60.0, 60.0, 70.0, 70.0)), Some(0));
        assert_eq!(region_of(&regions, &rect(200.0, 200.0, 210.0, 210.0)), None);
    }

    #[test]
    fn kind_for_should_drop_running_headers_and_strip_list_markers() {
        assert_eq!(kind_for(RegionLabel::PageHeader, "Chapter 1"), Err(DropReason::RunningHeader));
        assert_eq!(
            kind_for(RegionLabel::ListItem, "• Apples"),
            Ok((
                BlockKind::ListItem {
                    marker: "•".to_owned(),
                    depth: 0
                },
                Some("Apples".to_owned())
            ))
        );
        assert_eq!(
            kind_for(RegionLabel::SectionHeader, "Results"),
            Ok((BlockKind::Heading { level: 2 }, None))
        );
    }

    #[test]
    fn ocr_pieces_should_skip_empty_words_and_keep_confidence() {
        let words = [
            OcrWord::new("Hello", rect(10.0, 10.0, 40.0, 22.0), 0.97),
            OcrWord::new("  ", rect(50.0, 10.0, 60.0, 22.0), 0.5),
        ];
        let (pieces, spans) = ocr_pieces(&words);
        assert_eq!(pieces.len(), 1);
        assert_eq!(spans.first().and_then(|s| s.ocr), Some(0.97));
    }
}
