//! Text extraction (todo B9b): the text goldens extract the text their samples drew (as pypdf,
//! pdf.js and pdftotext read it, `fixtures/text/extracted.json` and `fixtures/build/facts.json`),
//! glyph origins follow `/Widths`, `TJ`, `Tc`, `Tw` and `Tz` (§9.4.4), and broken fonts degrade
//! with warnings.
#![allow(
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "tests: a panic is a test failure (serde_json `Value` indexing returns `Null` when absent)"
)]

use lamina_cos::OpenOptions;
use lamina_model::Document;
use lamina_save::Writer;
use lamina_syntax::{Dict, Object, Stream};
use lamina_text::{PageText, TextExtractor};
use serde_json::Value;

fn fixture(rel: &str) -> Vec<u8> {
    std::fs::read(format!("{}/../../fixtures/{rel}", env!("CARGO_MANIFEST_DIR"))).expect("fixture")
}

fn pages_text(bytes: Vec<u8>) -> (Vec<PageText>, Document) {
    let doc = Document::open(bytes, OpenOptions::default()).expect("opens");
    let mut extractor = TextExtractor::new();
    let pages = doc.pages().expect("pages");
    let text = pages
        .iter()
        .map(|page| extractor.page(&doc, &page).expect("extracts"))
        .collect();
    drop(pages);
    (text, doc)
}

/// Right-to-left lines come out in the order the glyphs are drawn (visual); bidi reordering and
/// `ActualText` are todo B9d, so only left-to-right lines are compared here.
fn right_to_left(line: &str) -> bool {
    line.chars()
        .any(|c| matches!(c, '\u{0590}'..='\u{08FF}' | '\u{FB1D}'..='\u{FEFF}'))
}

fn compare(name: &str, expected: &[Vec<String>]) {
    let (pages, doc) = pages_text(fixture(&format!("golden/{name}")));
    assert_eq!(pages.len(), expected.len(), "{name}: pages");
    for (i, (page, want)) in pages.iter().zip(expected).enumerate() {
        let got = page.lines();
        assert_eq!(got.len(), want.len(), "{name} page {i}: lines {got:?}");
        for (g, w) in got.iter().zip(want) {
            if !right_to_left(w) {
                assert_eq!(g, w, "{name} page {i}");
            }
        }
        assert_eq!(page.unmapped, 0, "{name} page {i}");
    }
    assert_eq!(doc.warnings(), [], "{name}");
}

fn strings(value: &Value) -> Vec<String> {
    value
        .as_array()
        .expect("lines")
        .iter()
        .map(|s| s.as_str().expect("a line").to_owned())
        .collect()
}

#[test]
fn text_goldens_extract_the_drawn_text() {
    let facts: Value = serde_json::from_slice(&fixture("text/extracted.json")).expect("json");
    let files = facts["files"].as_object().expect("files");
    assert!(files.len() >= 5);
    for (name, file) in files {
        compare(name, &[strings(&file["pypdf"])]);
    }
}

#[test]
fn builder_goldens_extract_the_drawn_text() {
    let facts: Value = serde_json::from_slice(&fixture("build/facts.json")).expect("json");
    for name in [
        "build-text-latin.pdf",
        "build-text-multiscript.pdf",
        "links-and-outlines.pdf",
    ] {
        let pages: Vec<Vec<String>> = facts["files"][name]["pypdf"]["pages"]
            .as_array()
            .expect("pages")
            .iter()
            .map(strings)
            .collect();
        compare(name, &pages);
    }
}

/// A one-page document with `content` and the fonts `fonts` as `/F1`, `/F2`, ….
fn page(content: &[u8], fonts: Vec<Dict>, options: OpenOptions) -> Document {
    let dict = |entries: Vec<(&str, Object)>| entries.into_iter().collect::<Dict>();
    let mut w = Writer::new(1, 7);
    let pages = w.reserve();
    let font_refs: Dict = fonts
        .into_iter()
        .enumerate()
        .map(|(i, font)| {
            (
                lamina_syntax::Name::new(format!("F{}", i + 1)),
                Object::from(w.add(font)),
            )
        })
        .collect();
    let content = w.add(Stream::new(Dict::new(), content.to_vec()));
    let page = w.add(dict(vec![
        ("Type", Object::name("Page")),
        ("Parent", pages.into()),
        ("MediaBox", vec![0.into(), 0.into(), 612.into(), 792.into()].into()),
        ("Contents", content.into()),
        ("Resources", dict(vec![("Font", font_refs.into())]).into()),
    ]));
    w.set(
        pages,
        dict(vec![
            ("Type", Object::name("Pages")),
            ("Kids", vec![page.into()].into()),
            ("Count", 1.into()),
        ]),
    );
    let root = w.add(dict(vec![("Type", Object::name("Catalog")), ("Pages", pages.into())]));
    let bytes = w.finish(&[("Root", root)].into_iter().collect()).expect("writes");
    Document::open(bytes, options).expect("opens")
}

fn font(source: &str) -> Dict {
    match lamina_syntax::parse_object(source.as_bytes()) {
        Ok(Object::Dict(d)) => d,
        other => panic!("{other:?}"),
    }
}

fn helvetica() -> Dict {
    font("<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica /Encoding /WinAnsiEncoding >>")
}

fn extract(doc: &Document) -> lamina_cos::Result<PageText> {
    let pages = doc.pages()?;
    let page = pages.get(0).expect("a page");
    TextExtractor::new().page(doc, &page)
}

fn xs(text: &PageText) -> Vec<f64> {
    text.glyphs
        .iter()
        .map(|g| (g.origin.0 * 1000.0).round() / 1000.0)
        .collect()
}

#[test]
fn tj_adjustments_move_the_glyphs_as_research_03_computes() {
    // Research 03 §2.6: W = 944, A = V = E = 667 at 24 pt from x = 72; E ends at 139.8, and
    // the −300 adjustment adds 7.2 (a space: over a quarter of the size).
    let doc = page(
        b"BT /F1 24 Tf 72 700 Td [(W) 120 (AVE) -300 (k)] TJ ET",
        vec![helvetica()],
        OpenOptions::default(),
    );
    let text = extract(&doc).expect("extracts");
    assert_eq!(
        (xs(&text), text.lines()),
        (vec![72.0, 91.776, 107.784, 123.792, 147.0], vec!["WAVE k".to_owned()])
    );
}

#[test]
fn widths_move_the_text_matrix_across_shows() {
    let doc = page(
        b"BT /F1 10 Tf 0 0 Td (AB) Tj (C) Tj ET",
        vec![helvetica()],
        OpenOptions::default(),
    );
    // A = B = 667 at 10 pt.
    assert_eq!(xs(&extract(&doc).expect("extracts")), [0.0, 6.67, 13.34]);
}

#[test]
fn char_and_word_spacing_and_horizontal_scaling_apply() {
    // Space = 278, A = 667; Tc 1, Tw 2 (code 32 only), Tz 50.
    let doc = page(
        b"BT /F1 10 Tf 1 Tc 2 Tw 50 Tz ( A) Tj ET",
        vec![helvetica()],
        OpenOptions::default(),
    );
    assert_eq!(xs(&extract(&doc).expect("extracts")), [0.0, 2.89]);
}

#[test]
fn the_ctm_and_text_matrix_scale_positions_and_sizes() {
    let doc = page(
        b"2 0 0 2 10 20 cm BT /F1 10 Tf 1 0 0 1 5 5 Tm (A) Tj ET",
        vec![helvetica()],
        OpenOptions::default(),
    );
    let glyph = extract(&doc).expect("extracts").glyphs.remove(0);
    let round = |v: f64| (v * 1000.0).round() / 1000.0;
    assert_eq!(
        (glyph.origin, (round(glyph.advance.0), glyph.advance.1), glyph.size),
        ((20.0, 30.0), (13.34, 0.0), 20.0)
    );
}

#[test]
fn a_baseline_change_starts_a_new_line() {
    let doc = page(
        b"BT /F1 10 Tf 0 100 Td (Hello) Tj 0 -12 Td (world) Tj 0 1 Td (!) Tj ET",
        vec![helvetica()],
        OpenOptions::default(),
    );
    assert_eq!(extract(&doc).expect("extracts").lines(), ["Hello", "world!"]);
}

fn unmapped_font() -> Dict {
    font("<< /Type /Font /Subtype /Type1 /BaseFont /X /Encoding << /Differences [1 /g37 /A] >> >>")
}

#[test]
fn unmapped_glyphs_extract_as_replacement_characters_with_a_warning() {
    let doc = page(
        b"BT /F1 10 Tf <0102> Tj ET",
        vec![unmapped_font()],
        OpenOptions::default(),
    );
    let text = extract(&doc).expect("extracts");
    let codes: Vec<&str> = doc.warnings().iter().map(lamina_cos::Warning::code).collect();
    assert_eq!(
        (text.lines(), text.unmapped, codes),
        (vec!["\u{FFFD}A".to_owned()], 1, vec!["unmapped-glyph"])
    );
}

#[test]
fn strict_mode_fails_on_an_unmapped_glyph() {
    let doc = page(
        b"BT /F1 10 Tf <01> Tj ET",
        vec![unmapped_font()],
        OpenOptions::default().with_strict(true),
    );
    let error = extract(&doc).expect_err("strict");
    assert_eq!(error.code(), "strict");
}

#[test]
fn a_broken_font_warns_once_and_still_extracts() {
    let broken = font("<< /Type /Font /Subtype /Type0 /BaseFont /X /Encoding /Identity-H >>");
    let doc = page(
        b"BT /F1 10 Tf <0041> Tj <0042> Tj ET",
        vec![broken],
        OpenOptions::default(),
    );
    let text = extract(&doc).expect("extracts");
    let codes: Vec<&str> = doc.warnings().iter().map(lamina_cos::Warning::code).collect();
    assert_eq!((text.glyphs.len(), codes), (2, vec!["font-invalid", "unmapped-glyph"]));
}

#[test]
fn text_without_a_font_is_skipped() {
    let doc = page(b"BT (A) Tj ET", vec![], OpenOptions::default());
    assert_eq!(extract(&doc).expect("extracts").glyphs, []);
}
