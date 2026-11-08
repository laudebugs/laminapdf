//! Each page's text-layer status on pages built for each rule: text, a scan, a scan with an OCR
//! layer or a stamp, text beside a picture, a blank page, outlined shapes, glyphs without
//! Unicode, and the warning that names the pages that need OCR.
#![allow(clippy::unwrap_used, clippy::panic, clippy::indexing_slicing, reason = "test code")]

use std::fmt::Write as _;

use laminapdf_build::{BuildOptions, Builder, FillRule, Image, PageSize, Standard14, Surface, TextRenderMode};
use laminapdf_cos::Warning;
use laminapdf_extract::{
    DropReason, ExtractOptions, Extraction, TextLayer, TextLayerStatus, TextLayerThresholds, extract,
};
use laminapdf_model::{Document, OpenOptions, PointPt, Pt, RectPt};
use laminapdf_save::{Save as _, SaveOptions};
use laminapdf_syntax::CancelToken;
use laminapdf_test_fixtures::{classic, fixture, fixtures_dir, stream_body};

/// A4 in points.
const A4: (f64, f64) = (595.28, 841.89);
const PARAGRAPH: &str = "Extraction reads the words a page draws with its fonts.";

/// What one built page draws.
#[derive(Clone, Copy)]
enum Draw {
    /// Lines of Helvetica.
    Text,
    /// One image over the whole page.
    Scan,
    /// The image, under an invisible line of 300 characters.
    ScanWithLayer,
    /// The image, with a visible 70-character stamp across its foot.
    ScanWithStamp,
    /// Lines of text beside an image covering a third of the page.
    TextBesidePicture,
    /// Nothing.
    Blank,
    /// Forty filled shapes made of curves.
    Shapes,
}

fn draw(s: &mut Surface, what: Draw, font: &laminapdf_build::FontHandle) {
    let gray = Image::rgba(2, 2, &[128; 16]).unwrap();
    let text = |s: &mut Surface, line: &str, x: f64, y: f64, size: f64| {
        s.text(line, font, Pt::new(size), PointPt::from_points(x, y)).unwrap();
    };
    let paragraph = |s: &mut Surface, x: f64| {
        for i in 0..8 {
            text(s, PARAGRAPH, x, 100.0 + 20.0 * f64::from(i), 10.0);
        }
    };
    match what {
        Draw::Text => paragraph(s, 72.0),
        Draw::Scan => {
            s.image(&gray, RectPt::from_points(0.0, 0.0, A4.0, A4.1));
        }
        Draw::ScanWithLayer => {
            s.image(&gray, RectPt::from_points(0.0, 0.0, A4.0, A4.1));
            let layer = "x".repeat(300);
            s.text(&layer, font, Pt::new(2.0), PointPt::from_points(20.0, 400.0))
                .unwrap()
                .render_mode(TextRenderMode::Invisible);
        }
        Draw::ScanWithStamp => {
            s.image(&gray, RectPt::from_points(0.0, 0.0, A4.0, A4.1));
            text(s, &"S".repeat(70), 20.0, 820.0, 6.0);
        }
        Draw::TextBesidePicture => {
            s.image(&gray, RectPt::from_points(0.0, 0.0, A4.0 / 3.0, A4.1));
            paragraph(s, 220.0);
        }
        Draw::Blank => {}
        Draw::Shapes => {
            for i in 0..40 {
                let x = 40.0 + 12.0 * f64::from(i);
                s.path(|p| {
                    p.move_to(PointPt::from_points(x, 100.0))
                        .cubic_to(
                            PointPt::from_points(x + 4.0, 80.0),
                            PointPt::from_points(x + 8.0, 120.0),
                            PointPt::from_points(x + 10.0, 100.0),
                        )
                        .close();
                })
                .fill(FillRule::NonZero);
            }
        }
    }
}

fn build(pages: &[Draw]) -> Document {
    let mut b = Builder::new(BuildOptions::default());
    let font = b.standard_font(Standard14::Helvetica).unwrap();
    for &what in pages {
        let mut page = b.page(PageSize::A4);
        draw(page.surface(), what, &font);
        page.finish().unwrap();
    }
    let mut doc = b.finish().unwrap();
    let mut bytes = Vec::new();
    doc.save(&mut bytes, &SaveOptions::new()).unwrap();
    Document::open(bytes, OpenOptions::default()).unwrap()
}

fn run(doc: &Document, options: &ExtractOptions) -> Extraction {
    extract(doc, options, &CancelToken::new()).unwrap()
}

fn layer(what: Draw) -> TextLayer {
    run(&build(&[what]), &ExtractOptions::new()).pages[0].text_layer
}

/// The pages `extract-needs-ocr` names, with their status.
fn needs_ocr(doc: &Document) -> Vec<(u64, &'static str)> {
    doc.warnings()
        .iter()
        .filter_map(|w| match w {
            Warning::ExtractNeedsOcr { page, status } => Some((*page, *status)),
            _ => None,
        })
        .collect()
}

#[test]
fn a_page_of_text_is_native() {
    let l = layer(Draw::Text);
    assert_eq!(l.status, TextLayerStatus::Native);
    assert_eq!(l.chars, 8 * PARAGRAPH.chars().filter(|c| !c.is_whitespace()).count());
    assert_eq!((l.invisible_chars, l.images, l.paths), (0, 0, 0));
}

#[test]
fn a_page_that_is_one_image_needs_ocr() {
    let l = layer(Draw::Scan);
    assert_eq!(l.status, TextLayerStatus::NeedsOcr);
    assert_eq!((l.chars, l.images), (0, 1));
    assert!((l.image_share - 1.0).abs() < 1e-9);
}

#[test]
fn a_scan_under_an_invisible_layer_has_an_ocr_layer_left_out_of_the_blocks() {
    let doc = build(&[Draw::ScanWithLayer]);
    let out = run(&doc, &ExtractOptions::new());
    let l = out.pages[0].text_layer;
    assert_eq!(l.status, TextLayerStatus::OcrLayer);
    assert_eq!((l.chars, l.invisible_chars), (0, 300));
    assert_eq!(out.blocks.len(), 0);
    assert!(
        out.dropped
            .iter()
            .any(|d| d.reason == DropReason::Invisible && d.chars == 300)
    );
    assert_eq!(needs_ocr(&doc), [(0, "ocr-layer")]);
}

#[test]
fn a_scan_with_a_stamp_still_needs_ocr() {
    let l = layer(Draw::ScanWithStamp);
    assert_eq!(l.status, TextLayerStatus::NeedsOcr);
    assert_eq!(l.chars, 70);
}

#[test]
fn text_beside_a_picture_is_mixed() {
    let l = layer(Draw::TextBesidePicture);
    assert_eq!(l.status, TextLayerStatus::Mixed);
    assert!((l.image_share - 0.344).abs() < 0.02, "{l:?}");
}

#[test]
fn a_blank_page_is_empty_and_raises_nothing() {
    let doc = build(&[Draw::Blank]);
    let out = run(&doc, &ExtractOptions::new());
    assert_eq!(out.pages[0].text_layer.status, TextLayerStatus::Empty);
    assert_eq!(needs_ocr(&doc), []);
}

#[test]
fn filled_shapes_without_text_need_ocr() {
    let l = layer(Draw::Shapes);
    assert_eq!(l.status, TextLayerStatus::NeedsOcr);
    assert_eq!((l.chars, l.paths), (0, 40));
}

#[test]
fn the_warning_names_each_page_that_needs_ocr() {
    let doc = build(&[Draw::Text, Draw::Scan, Draw::Blank, Draw::ScanWithLayer]);
    let out = run(&doc, &ExtractOptions::new());
    let statuses: Vec<TextLayerStatus> = out.pages.iter().map(|p| p.text_layer.status).collect();
    assert_eq!(
        statuses,
        [
            TextLayerStatus::Native,
            TextLayerStatus::NeedsOcr,
            TextLayerStatus::Empty,
            TextLayerStatus::OcrLayer
        ]
    );
    assert_eq!(needs_ocr(&doc), [(1, "needs-ocr"), (3, "ocr-layer")]);
}

#[test]
fn a_cut_off_from_the_options_changes_the_status_it_governs() {
    let doc = build(&[Draw::TextBesidePicture]);
    let mut thresholds = TextLayerThresholds::default();
    thresholds.image_mixed = 500;
    let out = run(&doc, &ExtractOptions::new().with_text_layer(thresholds));
    assert_eq!(out.pages[0].text_layer.status, TextLayerStatus::Native);
}

/// One page drawing `runs` (text, Type 3 or not) at 20 pt intervals: Helvetica as `/F1`, and as
/// `/F3` a Type 3 font whose one glyph, `g1`, has a name outside the Adobe Glyph List and no
/// `ToUnicode`, so it maps to no character.
fn type3_page(runs: &[(&str, bool)]) -> Document {
    let mut content = String::new();
    for (i, (text, type3)) in runs.iter().enumerate() {
        let font = if *type3 { "F3" } else { "F1" };
        let y = 760.0 - 20.0 * f64::from(u8::try_from(i).unwrap());
        writeln!(content, "BT /{font} 10 Tf 72 {y} Td ({text}) Tj ET").unwrap();
    }
    let glyph = "500 0 0 0 500 500 d1 0 0 500 500 re f";
    let objects: Vec<Vec<u8>> = vec![
        b"<< /Type /Catalog /Pages 2 0 R >>".to_vec(),
        b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_vec(),
        b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 595 842] /Contents 4 0 R \
           /Resources << /Font << /F1 5 0 R /F3 6 0 R >> >> >>"
            .to_vec(),
        stream_body("", content.as_bytes()),
        b"<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica /Encoding /WinAnsiEncoding >>".to_vec(),
        b"<< /Type /Font /Subtype /Type3 /FontBBox [0 0 500 500] /FontMatrix [0.001 0 0 0.001 0 0] \
           /CharProcs << /g1 7 0 R >> /Encoding << /Type /Encoding /Differences [65 /g1] >> \
           /FirstChar 65 /LastChar 65 /Widths [500] /Resources << >> >>"
            .to_vec(),
        stream_body("", glyph.as_bytes()),
    ];
    let refs: Vec<&[u8]> = objects.iter().map(Vec::as_slice).collect();
    Document::open(classic(&refs, "<< /Size 8 /Root 1 0 R >>"), OpenOptions::default()).unwrap()
}

#[test]
fn glyphs_without_unicode_need_ocr_and_some_of_them_make_a_page_mixed() {
    let a = "A".repeat(20);
    let only = run(&type3_page(&[(&a, true)]), &ExtractOptions::new());
    let l = only.pages[0].text_layer;
    assert_eq!(l.status, TextLayerStatus::NeedsOcr, "{l:?}");
    assert!((l.unmapped_share - 1.0).abs() < 1e-9);
    let words = "Mapped words here.";
    let doc = type3_page(&[
        (words, false),
        (words, false),
        (words, false),
        (words, false),
        (&a, true),
    ]);
    let l = run(&doc, &ExtractOptions::new()).pages[0].text_layer;
    assert_eq!(l.status, TextLayerStatus::Mixed, "{l:?}");
}

#[test]
fn every_page_of_the_reading_order_corpus_and_the_text_goldens_is_native() {
    let mut files: Vec<std::path::PathBuf> = std::fs::read_dir(fixtures_dir().join("reading-order"))
        .unwrap()
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.extension().is_some_and(|e| e == "pdf"))
        .collect();
    files.extend(
        std::fs::read_dir(fixtures_dir().join("golden"))
            .unwrap()
            .filter_map(|e| e.ok().map(|e| e.path()))
            .filter(|p| p.file_name().is_some_and(|n| n.to_string_lossy().starts_with("text-")))
            .filter(|p| p.extension().is_some_and(|e| e == "pdf")),
    );
    assert!(files.len() > 5);
    for path in files {
        let doc = Document::open(std::fs::read(&path).unwrap(), OpenOptions::default()).unwrap();
        let out = run(&doc, &ExtractOptions::new());
        for page in &out.pages {
            assert_eq!(
                page.text_layer.status,
                TextLayerStatus::Native,
                "{} p{}",
                path.display(),
                page.index
            );
        }
        assert!(needs_ocr(&doc).is_empty(), "{}", path.display());
    }
}

#[test]
fn the_builders_image_and_vector_goldens_have_no_text_so_they_need_ocr() {
    // Both draw only pictures: images.pdf nine images over about a third of the page, vectors.pdf
    // nineteen painted paths. Neither has a glyph, so OCR is the only way to read them.
    let layer = |name: &str| {
        let doc = Document::open(fixture(&format!("golden/{name}")), OpenOptions::default()).unwrap();
        run(&doc, &ExtractOptions::new()).pages[0].text_layer
    };
    let images = layer("images.pdf");
    assert_eq!(
        (images.status, images.chars, images.images),
        (TextLayerStatus::NeedsOcr, 0, 9)
    );
    assert!((images.image_share - 0.302).abs() < 1e-9, "{images:?}");
    let vectors = layer("vectors.pdf");
    assert_eq!(
        (vectors.status, vectors.chars, vectors.paths),
        (TextLayerStatus::NeedsOcr, 0, 19)
    );
}
