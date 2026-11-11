//! The approximate reader mode (tier 2) on built pages and on the reading-order corpus: a scan
//! and a page whose column order extraction doubts are shown as their images; a picture big
//! enough to be a figure is a crop placed where it stands in the text, with its caption after it;
//! an icon stays out.
#![allow(clippy::unwrap_used, clippy::panic, clippy::indexing_slicing, reason = "test code")]

use laminapdf_build::{BuildOptions, Builder, Image, PageSize, Standard14, Surface, TextRenderMode};
use laminapdf_extract::{PageImageReason, Reflow, ReflowKind, ReflowSource, reflow};
use laminapdf_model::{Document, OpenOptions, PointPt, Pt, RectPt};
use laminapdf_save::{Save as _, SaveOptions};
use laminapdf_syntax::CancelToken;

const LINE: &str = "Orchards along the river were planted in rows that follow the slope of the land.";

/// One untagged A4 page drawn by `draw`, saved and opened again.
fn page(draw: impl FnOnce(&mut Surface, &laminapdf_build::FontHandle)) -> Reflow {
    let mut b = Builder::new(BuildOptions::default());
    let font = b.standard_font(Standard14::Helvetica).unwrap();
    let mut page = b.page(PageSize::A4);
    draw(page.surface(), &font);
    page.finish().unwrap();
    let mut doc = b.finish().unwrap();
    let mut bytes = Vec::new();
    doc.save(&mut bytes, &SaveOptions::new()).unwrap();
    let doc = Document::open(bytes, OpenOptions::default()).unwrap();
    reflow(&doc, &CancelToken::new()).unwrap()
}

/// Lines of text from `y` down, 14 pt apart.
fn lines(s: &mut Surface, font: &laminapdf_build::FontHandle, text: &str, y: f64, count: u32) {
    for i in 0..count {
        let at = PointPt::from_points(72.0, y + 14.0 * f64::from(i));
        s.text(text, font, Pt::new(10.0), at).unwrap();
    }
}

fn gray() -> Image {
    Image::rgba(2, 2, &[128; 16]).unwrap()
}

#[test]
fn a_scanned_page_is_shown_as_its_image() {
    let got = page(|s, _| {
        s.image(&gray(), RectPt::from_points(0.0, 0.0, 595.0, 842.0));
    });
    assert_eq!(got.source, ReflowSource::Approximate);
    let kinds: Vec<&ReflowKind> = got.blocks.iter().map(|b| &b.kind).collect();
    assert_eq!(
        kinds,
        [&ReflowKind::PageImage {
            why: PageImageReason::Scanned
        }]
    );
    assert_eq!(got.checks.page_images, 1);
}

#[test]
fn a_scan_with_an_ocr_layer_keeps_the_layers_text_for_searching() {
    let got = page(|s, font| {
        s.image(&gray(), RectPt::from_points(0.0, 0.0, 595.0, 842.0));
        for i in 0..4 {
            let at = PointPt::from_points(72.0, 100.0 + 14.0 * f64::from(i));
            s.text(LINE, font, Pt::new(10.0), at)
                .unwrap()
                .render_mode(TextRenderMode::Invisible);
        }
    });
    assert_eq!(got.blocks.len(), 1, "{:#?}", got.blocks);
    assert!(
        matches!(
            got.blocks[0].kind,
            ReflowKind::PageImage {
                why: PageImageReason::Scanned
            }
        ) && got.blocks[0].text.contains("Orchards along the river"),
        "{:#?}",
        got.blocks
    );
}

#[test]
fn a_figure_is_a_crop_where_it_stands_with_its_caption_after_it() {
    let got = page(|s, font| {
        lines(s, font, LINE, 100.0, 4);
        s.image(&gray(), RectPt::from_points(72.0, 180.0, 300.0, 200.0));
        s.text(
            "Figure 1. The orchard in spring",
            font,
            Pt::new(9.0),
            PointPt::from_points(72.0, 400.0),
        )
        .unwrap();
        lines(
            s,
            font,
            "The second paragraph comes after the picture in the column.",
            440.0,
            4,
        );
        // An icon is no figure.
        s.image(&gray(), RectPt::from_points(500.0, 40.0, 12.0, 12.0));
    });
    let kinds: Vec<&ReflowKind> = got.blocks.iter().map(|b| &b.kind).collect();
    assert_eq!(
        kinds,
        [
            &ReflowKind::Paragraph,
            &ReflowKind::Figure,
            &ReflowKind::Caption,
            &ReflowKind::Paragraph
        ],
        "{:#?}",
        got.blocks
    );
    let figure = got.blocks[1].bbox.unwrap();
    assert!((figure.width().get() - 300.0).abs() < 0.5 && (figure.height().get() - 200.0).abs() < 0.5);
    assert!(got.blocks[2].text.starts_with("Figure 1."));
}

#[test]
fn a_blank_document_has_nothing_to_show() {
    let got = page(|_, _| {});
    assert!(!got.available);
    assert_eq!(
        got.reason.map(laminapdf_extract::ReflowRefusal::as_str),
        Some("nothing")
    );
}

/// The kinds of `name`'s blocks, from `fixtures/reading-order/`.
fn corpus_kinds(name: &str) -> Vec<ReflowKind> {
    let path = laminapdf_test_fixtures::fixtures_dir().join(format!("reading-order/{name}.pdf"));
    let doc = Document::open(std::fs::read(path).unwrap(), OpenOptions::default()).unwrap();
    reflow(&doc, &CancelToken::new())
        .unwrap()
        .blocks
        .into_iter()
        .map(|b| b.kind)
        .collect()
}

#[test]
fn a_page_whose_column_order_extraction_doubts_is_shown_as_its_image() {
    let uncertain = [ReflowKind::PageImage {
        why: PageImageReason::Uncertain,
    }];
    for name in ["narrow-gutter-ordered", "wrapped-grid-ordered", "pull-quote-ordered"] {
        assert_eq!(corpus_kinds(name), uncertain, "{name}");
    }
    // A layout extraction reads with confidence reflows.
    let kinds = corpus_kinds("two-columns-reversed");
    assert!(!kinds.is_empty() && kinds.iter().all(|k| !k.is_crop()), "{kinds:?}");
}
