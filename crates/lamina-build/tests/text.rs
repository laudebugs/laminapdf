//! `Surface::text` (todo B5b): runs with embedded and standard 14 fonts, read back through
//! lamina's reader and `lamina-content`: the baseline is `y` (ADR 0027 §5), the text reads
//! as its input, render modes, versions and refusals.
#![allow(
    clippy::unwrap_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "tests: a panic is a test failure"
)]

mod common;

use common::b5b::{self, LATIN_PARAGRAPH, LATIN_RUNS, SCRIPTS};
use common::text::{lines, shown};
use common::{fixture, reopen, save};
use lamina_build::{
    BuildOptions, Builder, Feature, LinkTarget, Matrix, PageSize, PdfVersion, Rotation, Standard14, TextRenderMode,
};
use lamina_cos::Object;
use lamina_model::{PointPt, Pt};

fn close(a: (f64, f64), b: (f64, f64)) -> bool {
    (a.0 - b.0).abs() < 1e-3 && (a.1 - b.1).abs() < 1e-3
}

/// One page with `text` in `font` at (72, 100), saved and reopened.
fn one_run(
    font: impl FnOnce(&mut Builder) -> lamina_build::Font,
    text: &str,
    rotate: Rotation,
) -> lamina_model::Document {
    let mut b = Builder::new(BuildOptions::default());
    let f = font(&mut b);
    let mut page = b.page(PageSize::A4);
    page.rotate(rotate);
    let _ = page
        .surface()
        .text(text, &f, Pt::new(12.0), PointPt::from_points(72.0, 100.0))
        .unwrap();
    page.finish().unwrap();
    reopen(&save(b.finish().unwrap()))
}

#[test]
fn y_is_the_baseline_on_every_rotation() {
    for rotate in [
        Rotation::None,
        Rotation::Clockwise90,
        Rotation::Half,
        Rotation::Clockwise270,
    ] {
        let doc = one_run(|b| b.font(b5b::font("NotoSans-Regular.ttf")).unwrap(), "Hello", rotate);
        let frame = doc.pages().unwrap().get(0).unwrap().frame().unwrap();
        let want = frame.to_user_space(PointPt::from_points(72.0, 100.0));
        let (runs, _) = shown(&doc, 0);
        assert!(
            close(runs[0].origin, (want.x, want.y)),
            "{rotate:?}: {:?}",
            runs[0].origin
        );
    }
}

#[test]
fn the_latin_golden_reads_back_as_its_input() {
    let doc = reopen(&fixture("golden/build-text-latin.pdf"));
    let got = lines(&doc, 0);
    // Every run and every text box line, in drawing order.
    assert_eq!(vec![got.clone()], b5b::text_latin_drawn().1);
    assert_eq!(&got[..5], &LATIN_RUNS[..]);
    // Each of the four boxes holds the whole paragraph, broken at spaces.
    let boxed = got[7..].join(" ");
    assert_eq!(boxed, [LATIN_PARAGRAPH; 4].join(" "));
}

#[test]
fn every_script_reads_back_through_lamina_content() {
    let doc = reopen(&fixture("golden/build-text-multiscript.pdf"));
    let got = lines(&doc, 0);
    assert_eq!(got.len(), SCRIPTS.len());
    for ((_, want), got) in SCRIPTS.iter().zip(&got) {
        // Right-to-left runs are drawn left to right, so a reader without bidi gets their
        // glyphs in visual order: compare the characters (pypdf and pdf.js order them; see
        // tests/external.rs).
        let sorted = |s: &str| {
            let mut c: Vec<char> = s.chars().collect();
            c.sort_unstable();
            c
        };
        assert_eq!(sorted(got), sorted(want), "{want}");
    }
}

#[test]
fn ltr_scripts_read_back_in_order() {
    let doc = reopen(&fixture("golden/build-text-multiscript.pdf"));
    let got = lines(&doc, 0);
    for i in [0, 2, 4] {
        assert_eq!(got[i], SCRIPTS[i].1);
    }
}

#[test]
fn a_standard_14_run_is_a_simple_font_with_tounicode() {
    let doc = one_run(
        |b| b.standard_font(Standard14::Courier).unwrap(),
        "Courier: café",
        Rotation::None,
    );
    let pages = doc.pages().unwrap();
    let page = pages.get(0).unwrap();
    let fonts = page
        .resources()
        .unwrap()
        .get("Font")
        .unwrap()
        .as_dict()
        .unwrap()
        .clone();
    let Object::Ref(r) = fonts.get("F1").unwrap() else {
        panic!()
    };
    let font = doc.resolve(*r).unwrap();
    let d = font.as_dict().unwrap();
    assert_eq!(
        (
            d.get_name("Subtype"),
            d.get_name("BaseFont"),
            d.contains_key("ToUnicode")
        ),
        (Some(&b"Type1"[..]), Some(&b"Courier"[..]), true)
    );
    assert_eq!(lines(&doc, 0), ["Courier: café"]);
}

#[test]
fn a_line_feed_starts_a_new_line_lower_down() {
    let doc = one_run(
        |b| b.standard_font(Standard14::Helvetica).unwrap(),
        "one\ntwo",
        Rotation::None,
    );
    let (runs, _) = shown(&doc, 0);
    assert_eq!(runs.len(), 2);
    assert!((runs[0].origin.1 - runs[1].origin.1 - 14.4).abs() < 1e-3);
    assert_eq!(lines(&doc, 0), ["one", "two"]);
}

#[test]
fn render_modes_are_written_once_and_reset() {
    let mut b = Builder::new(BuildOptions::default());
    let f = b.standard_font(Standard14::Helvetica).unwrap();
    let mut page = b.page(PageSize::A4);
    let s = page.surface();
    let _ = s
        .text("hidden", &f, Pt::new(12.0), PointPt::from_points(72.0, 72.0))
        .unwrap()
        .render_mode(TextRenderMode::Invisible);
    let _ = s
        .text("shown", &f, Pt::new(12.0), PointPt::from_points(72.0, 92.0))
        .unwrap();
    page.finish().unwrap();
    let doc = reopen(&save(b.finish().unwrap()));
    let modes: Vec<i64> = shown(&doc, 0).0.iter().map(|s| s.render_mode).collect();
    assert_eq!(modes, [3, 0]);
}

/// The content of a page with "office" drawn, with `features`.
fn office(features: &[&str]) -> (Vec<u8>, Vec<String>) {
    let mut b = Builder::new(BuildOptions::default().with_compress(false));
    let f = b.font(b5b::font("NotoSans-Regular.ttf")).unwrap();
    let mut page = b.page(PageSize::A4);
    let features: Vec<Feature> = features.iter().map(|f| f.parse().unwrap()).collect();
    let _ = page
        .surface()
        .text("office", &f, Pt::new(12.0), PointPt::from_points(72.0, 72.0))
        .unwrap()
        .features(&features)
        .unwrap();
    page.finish().unwrap();
    let doc = reopen(&save(b.finish().unwrap()));
    let cos: &lamina_cos::Document = &doc;
    let content = cos.page_content(&cos.pages().unwrap().remove(0)).unwrap();
    (content, lines(&doc, 0))
}

#[test]
fn a_feature_change_reshapes_the_run() {
    let (ligated, read) = office(&[]);
    let (plain, read_plain) = office(&["liga=0"]);
    // The ffi ligature is one glyph (one two-byte code) instead of three.
    assert_ne!(ligated, plain);
    assert_eq!(
        (read, read_plain),
        (vec!["office".to_owned()], vec!["office".to_owned()])
    );
}

#[test]
fn a_character_the_font_lacks_is_missing_glyph() {
    let mut b = Builder::new(BuildOptions::default());
    let helvetica = b.standard_font(Standard14::Helvetica).unwrap();
    let noto = b.font(b5b::font("NotoSans-Regular.ttf")).unwrap();
    let mut page = b.page(PageSize::A4);
    let s = page.surface();
    let at = PointPt::from_points(72.0, 72.0);
    let std = s.text("中", &helvetica, Pt::new(12.0), at).map(|_| ()).unwrap_err();
    let embedded = s.text("a中", &noto, Pt::new(12.0), at).map(|_| ()).unwrap_err();
    assert_eq!((std.code(), embedded.code()), ("missing-glyph", "missing-glyph"));
    // Nothing was drawn: the page has no text.
    page.finish().unwrap();
    assert_eq!(lines(&reopen(&save(b.finish().unwrap())), 0), Vec::<String>::new());
}

#[test]
fn sizes_and_positions_must_be_finite() {
    let mut b = Builder::new(BuildOptions::default());
    let f = b.standard_font(Standard14::Helvetica).unwrap();
    let mut page = b.page(PageSize::A4);
    let s = page.surface();
    let codes: Vec<&str> = [
        s.text("x", &f, Pt::new(0.0), PointPt::from_points(1.0, 1.0))
            .map(|_| ()),
        s.text("x", &f, Pt::new(f64::NAN), PointPt::from_points(1.0, 1.0))
            .map(|_| ()),
        s.text("x", &f, Pt::new(12.0), PointPt::from_points(f64::INFINITY, 1.0))
            .map(|_| ()),
    ]
    .iter()
    .map(|r| r.as_ref().unwrap_err().code())
    .collect();
    assert_eq!(codes, ["invalid-geometry"; 3]);
}

#[test]
fn a_font_from_another_builder_is_unknown_font() {
    let mut other = Builder::new(BuildOptions::default());
    let foreign = other.standard_font(Standard14::Helvetica).unwrap();
    let mut b = Builder::new(BuildOptions::default());
    let mut page = b.page(PageSize::A4);
    let _ = page
        .surface()
        .text("x", &foreign, Pt::new(12.0), PointPt::from_points(1.0, 20.0))
        .unwrap();
    assert_eq!(page.finish().unwrap_err().code(), "unknown-font");
}

/// A PDF 1.4 document with `text` in `font`: its first page's content and catalog `/Version`.
fn at_1_4(font: &str, text: &str) -> (String, Option<Vec<u8>>) {
    let mut b = Builder::new(BuildOptions::default().with_version(PdfVersion::V1_4));
    let f = b.font(b5b::font(font)).unwrap();
    let mut page = b.page(PageSize::A4);
    let _ = page
        .surface()
        .text(text, &f, Pt::new(12.0), PointPt::from_points(72.0, 72.0))
        .unwrap();
    page.finish().unwrap();
    let doc = b.finish().unwrap();
    let cos: &lamina_cos::Document = &doc;
    let content = String::from_utf8_lossy(&cos.page_content(&cos.pages().unwrap().remove(0)).unwrap()).into_owned();
    (content, doc.catalog().unwrap().get_name("Version").map(<[u8]>::to_vec))
}

#[test]
fn actual_text_raises_an_old_version_to_1_5() {
    // A cluster holding a default-ignorable (the ZWJ asking for a half form) can't be spelt
    // by its glyphs' CIDs (ZWJ is never a glyph's text), so it gets /ActualText (B4d).
    let (content, version) = at_1_4("NotoSansDevanagari-Regular.ttf", "क्\u{200D}ष");
    assert!(content.contains("/ActualText"), "{content}");
    assert_eq!(version.as_deref(), Some(&b"1.5"[..]));
}

#[test]
fn plain_text_leaves_a_1_4_document_alone() {
    let (content, version) = at_1_4("NotoSans-Regular.ttf", "Hello");
    assert!(!content.contains("/ActualText"));
    assert_eq!(version, None);
}

/// The lines `text()` lays `text` out in with a standard 14 font and with an embedded one.
fn line_counts(text: &str) -> (usize, usize) {
    let mut b = Builder::new(BuildOptions::default());
    let std = b.standard_font(Standard14::Helvetica).unwrap();
    let noto = b.font(b5b::font("NotoSans-Regular.ttf")).unwrap();
    let mut page = b.page(PageSize::A4);
    let s = page.surface();
    let at = PointPt::from_points(72.0, 72.0);
    let simple = s.text(text, &std, Pt::new(12.0), at).unwrap().lines();
    let shaped = s.text(text, &noto, Pt::new(12.0), at).unwrap().lines();
    (simple, shaped)
}

#[test]
fn standard_14_and_shaped_layouts_break_paragraphs_alike() {
    for text in [
        "",
        "a",
        "a\n",
        "a\nb",
        "a\n\nb",
        "a\r\nb",
        "a\u{1C}b",
        "a\u{2029}b",
        "a\u{85}b",
        "a b  \n",
    ] {
        let (simple, shaped) = line_counts(text);
        assert_eq!(simple, shaped, "{text:?}");
    }
}

#[test]
fn a_failed_page_leaves_no_font_behind() {
    let mut other = Builder::new(BuildOptions::default());
    let foreign = other.standard_font(Standard14::Courier).unwrap();
    let mut b = Builder::new(BuildOptions::default().with_compress(false));
    let helvetica = b.standard_font(Standard14::Helvetica).unwrap();
    let mut page = b.page(PageSize::A4);
    let s = page.surface();
    let _ = s
        .text("recorded", &helvetica, Pt::new(12.0), PointPt::from_points(72.0, 72.0))
        .unwrap();
    let _ = s
        .text("refused", &foreign, Pt::new(12.0), PointPt::from_points(72.0, 92.0))
        .unwrap();
    assert_eq!(page.finish().unwrap_err().code(), "unknown-font");
    b.page(PageSize::A4).finish().unwrap();
    let bytes = save(b.finish().unwrap());
    let doc = reopen(&bytes);
    assert_eq!(doc.pages().unwrap().len(), 1);
    assert!(
        !String::from_utf8_lossy(&bytes).contains("/BaseFont"),
        "an orphan font was written"
    );
}

#[test]
fn a_run_link_covers_its_transformed_bounds() {
    let mut b = Builder::new(BuildOptions::default());
    let f = b.standard_font(Standard14::Helvetica).unwrap();
    let mut page = b.page(PageSize::A4);
    let s = page.surface();
    s.transform(Matrix::translate(Pt::new(100.0), Pt::new(50.0)));
    let run = s
        .text("link", &f, Pt::new(10.0), PointPt::from_points(0.0, 20.0))
        .unwrap()
        .link(LinkTarget::Uri("https://example.com".into()), None);
    let bounds = run.bounds();
    page.finish().unwrap();
    let doc = reopen(&save(b.finish().unwrap()));
    let pages = doc.pages().unwrap();
    let page = pages.get(0).unwrap();
    let frame = page.frame().unwrap();
    let annot = doc.resolve(page.annots().unwrap()[0]).unwrap();
    let rect: Vec<f64> = annot
        .as_dict()
        .unwrap()
        .get("Rect")
        .unwrap()
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_f64().unwrap())
        .collect();
    let tl = frame.to_user_space(PointPt::new(bounds.x0 + Pt::new(100.0), bounds.y0 + Pt::new(50.0)));
    let br = frame.to_user_space(PointPt::new(bounds.x1 + Pt::new(100.0), bounds.y1 + Pt::new(50.0)));
    assert!(
        (rect[0] - tl.x).abs() < 1e-3
            && (rect[3] - tl.y).abs() < 1e-3
            && (rect[2] - br.x).abs() < 1e-3
            && (rect[1] - br.y).abs() < 1e-3,
        "{rect:?}"
    );
}
