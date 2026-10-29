//! Shaped runs as PDF text operations (todo B4b), against values computed by hand from the
//! fonts' `hmtx` widths and HarfBuzz's positions (`hb-shape` output in the comments).
#![allow(
    clippy::unwrap_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "tests: a panic is a test failure"
)]

mod common;

use lamina_text::{EmbedPolicy, Error, FontRegistry, HarfRust, PdfRun, ShapeOptions, Shaper, TjElement, pdf_run};

fn codes(cids: &[u16]) -> TjElement {
    TjElement::Codes(cids.iter().flat_map(|c| c.to_be_bytes()).collect())
}

/// Shapes `text` with fixture font `name` into a fresh registry.
fn run(name: &str, text: &str) -> (PdfRun, FontRegistry, lamina_text::FontKey) {
    let font = common::font(name);
    let shaped = HarfRust.shape(&font, text, &ShapeOptions::new()).unwrap();
    let mut fonts = FontRegistry::new();
    let key = fonts.add(font, EmbedPolicy::Respect).unwrap();
    let pdf = pdf_run(&shaped, text, &mut fonts, key).unwrap();
    (pdf, fonts, key)
}

#[test]
fn kerning_becomes_tj_numbers() {
    // hb-shape "AVo": A 599, V 580, o 605; hmtx (upem 1000): A 639, V 600, o 605. After A the
    // pen is 40 units short of /W's advance: +40 pulls V back; after V, +20.
    let (pdf, _, _) = run("NotoSans-Regular.ttf", "AVo");
    assert_eq!(pdf.pieces.len(), 1);
    let segment = &pdf.pieces[0].segments[0];
    assert_eq!(segment.rise, 0.0);
    assert_eq!(
        segment.tj,
        [
            codes(&[1]),
            TjElement::Adjust(40.0),
            codes(&[2]),
            TjElement::Adjust(20.0),
            codes(&[3])
        ]
    );
    assert_eq!(pdf.advance, 599.0 + 580.0 + 605.0);
    assert!(!pdf.needs_actual_text());
}

#[test]
fn a_ligature_maps_to_its_letters_without_actual_text() {
    // hb-shape "fi": one glyph (368), advance 602 = its hmtx width.
    let (pdf, fonts, key) = run("NotoSans-Regular.ttf", "fi");
    assert_eq!(pdf.pieces[0].segments[0].tj, [codes(&[1])]);
    assert_eq!(fonts.usage(key).unwrap().embedded().unwrap().text()[&1], "fi");
    assert!(!pdf.needs_actual_text());
}

#[test]
fn a_reordered_matra_needs_actual_text() {
    // hb-shape "कि": i-matra (88) drawn before KA (7), both in cluster 0. Each glyph keeps
    // its own character in ToUnicode; read in drawing order they spell "िक", so the span
    // gets /ActualText with the logical "कि".
    let (pdf, fonts, key) = run("NotoSansDevanagari-Regular.ttf", "कि");
    assert_eq!(pdf.pieces.len(), 1);
    assert_eq!(pdf.pieces[0].actual_text.as_deref(), Some("कि"));
    assert_eq!(pdf.pieces[0].segments[0].tj, [codes(&[1, 2])]);
    let text = fonts.usage(key).unwrap().embedded().unwrap().text();
    assert_eq!(text.values().map(String::as_str).collect::<Vec<_>>(), ["\u{93F}", "क"]);
}

#[test]
fn a_reph_takes_the_ra_and_virama() {
    // hb-shape "र्म": MA (14) then the reph (82), both in cluster 0. MA keeps "म"; the reph,
    // the font's glyph for neither character, takes the rest, "र्"; read in order: "मर्".
    let (pdf, fonts, key) = run("NotoSansDevanagari-Regular.ttf", "र्म");
    assert_eq!(pdf.pieces[0].actual_text.as_deref(), Some("र्म"));
    let text = fonts.usage(key).unwrap().embedded().unwrap().text();
    assert_eq!(text.values().map(String::as_str).collect::<Vec<_>>(), ["म", "र्"]);
}

#[test]
fn a_base_and_its_mark_map_one_to_one() {
    // hb-shape "בְ" (RTL): sheva (63) at x 192, then BET (5): each the font's own glyph for
    // one character, and in visual right-to-left order they read BET, sheva, the text's
    // order. No /ActualText needed.
    let (pdf, fonts, key) = run("NotoSansHebrew-Regular.ttf", "בְ");
    assert!(!pdf.needs_actual_text());
    let text = fonts.usage(key).unwrap().embedded().unwrap().text();
    assert_eq!(text.values().map(String::as_str).collect::<Vec<_>>(), ["\u{5B0}", "ב"]);
}

#[test]
fn vertical_offsets_become_rises_and_horizontal_offsets_tj_numbers() {
    // hb-shape "كِ" (RTL): kasra 270 at (268, −21) advance 0, glyph 236 advance 0, KAF 48
    // advance 882; hmtx widths 0, 0, 882. The kasra is moved right by 268 and down by 21;
    // the pen comes back before glyph 236.
    let (pdf, _, _) = run("NotoSansArabic-Regular.ttf", "كِ");
    let segments = &pdf.pieces[0].segments;
    assert_eq!(segments.len(), 2);
    assert_eq!(segments[0].rise, -21.0);
    assert_eq!(segments[0].tj, [TjElement::Adjust(-268.0), codes(&[1])]);
    assert_eq!(segments[1].rise, 0.0);
    assert_eq!(segments[1].tj, [TjElement::Adjust(268.0), codes(&[2, 3])]);
    assert_eq!(pdf.advance, 882.0);
}

#[test]
fn rtl_runs_are_in_order_when_clusters_decrease() {
    let (pdf, _, _) = run("NotoSansHebrew-Regular.ttf", "שלום");
    assert!(!pdf.needs_actual_text());
}

#[test]
fn notdef_glyphs_are_missing_glyph_errors() {
    let font = common::font("NotoSans-Regular.ttf");
    let shaped = HarfRust.shape(&font, "a中", &ShapeOptions::new()).unwrap();
    assert!(shaped.has_notdef());
    let mut fonts = FontRegistry::new();
    let key = fonts.add(font, EmbedPolicy::Respect).unwrap();
    assert_eq!(
        pdf_run(&shaped, "a中", &mut fonts, key).unwrap_err(),
        Error::MissingGlyph {
            ch: '中',
            font: "NotoSans-Regular".to_owned()
        }
    );
}

#[test]
fn a_glyph_already_standing_for_other_text_gets_actual_text() {
    let font = common::font("NotoSans-Regular.ttf");
    let mut fonts = FontRegistry::new();
    let key = fonts.add(font.clone(), EmbedPolicy::Respect).unwrap();
    // The `fi` ligature glyph first drawn for "fi", then claimed for other text.
    let shaped = HarfRust.shape(&font, "fi", &ShapeOptions::new()).unwrap();
    pdf_run(&shaped, "fi", &mut fonts, key).unwrap();
    let gid = shaped.glyphs[0].gid;
    fonts.use_glyph(key, gid, "fi").unwrap();
    let mut other = shaped.clone();
    other.text_range = 0..1;
    other.glyphs[0].cluster = 0;
    let pdf = pdf_run(&other, "fx", &mut fonts, key).unwrap();
    assert_eq!(pdf.pieces[0].actual_text.as_deref(), Some("f"));
}
