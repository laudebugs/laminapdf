//! Shaped runs as PDF text operations (todos B4b, B4d), against values computed by hand from the
//! fonts' `hmtx` widths and HarfBuzz's positions (`hb-shape` output in the comments).
#![allow(
    clippy::unwrap_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "tests: a panic is a test failure"
)]

mod common;

use laminapdf_text::{EmbedPolicy, Error, FontRegistry, HarfRust, PdfRun, ShapeOptions, Shaper, TjElement, pdf_run};

fn codes(cids: &[u16]) -> TjElement {
    TjElement::Codes(cids.iter().flat_map(|c| c.to_be_bytes()).collect())
}

/// Shapes `text` with fixture font `name` into a fresh registry.
fn run(name: &str, text: &str) -> (PdfRun, FontRegistry, laminapdf_text::FontKey) {
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
fn a_reordered_matra_reads_in_order_without_actual_text() {
    // hb-shape "कि": i-matra (88) drawn before KA (7), both in cluster 0 (B4d). The cluster's
    // characters are dealt out in drawing order: the i-matra's CID stands for "क", KA's for
    // "ि", so extractors that read codes in order get "कि" without /ActualText.
    let (pdf, fonts, key) = run("NotoSansDevanagari-Regular.ttf", "कि");
    assert_eq!(pdf.pieces.len(), 1);
    assert!(!pdf.needs_actual_text());
    assert_eq!(pdf.pieces[0].segments[0].tj, [codes(&[1, 2])]);
    let text = fonts.usage(key).unwrap().embedded().unwrap().text();
    assert_eq!(text.values().map(String::as_str).collect::<Vec<_>>(), ["क", "\u{93F}"]);
}

#[test]
fn a_reph_reads_in_order_without_actual_text() {
    // hb-shape "र्म": MA (14) then the reph (82), both in cluster 0. MA would keep "म" and the
    // reph the rest, "र्" (three characters: one, two); dealt out in drawing order MA's CID
    // stands for "र" and the reph's for "्म".
    let (pdf, fonts, key) = run("NotoSansDevanagari-Regular.ttf", "र्म");
    assert!(!pdf.needs_actual_text());
    let text = fonts.usage(key).unwrap().embedded().unwrap().text();
    assert_eq!(text.values().map(String::as_str).collect::<Vec<_>>(), ["र", "्म"]);
}

#[test]
fn a_reordered_matra_in_a_cff_font_keeps_actual_text() {
    // CFF outlines can't give a glyph a second CID (B4d), so B4b's mapping stays: each glyph
    // its own character and /ActualText for the reordered cluster. Noto Sans (CFF) has no
    // Devanagari, so the reordering is simulated: "ab" shaped as one cluster drawn "ba".
    let font = common::font("NotoSans-Regular.otf");
    let mut shaped = HarfRust.shape(&font, "ab", &ShapeOptions::new()).unwrap();
    shaped.glyphs.swap(0, 1);
    for g in &mut shaped.glyphs {
        g.cluster = 0;
    }
    let mut fonts = FontRegistry::new();
    let key = fonts.add(font, EmbedPolicy::Respect).unwrap();
    let pdf = pdf_run(&shaped, "ab", &mut fonts, key).unwrap();
    assert_eq!(pdf.pieces[0].actual_text.as_deref(), Some("ab"));
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
    // the pen comes back before glyph 236, which stands for no text: it is drawn with the
    // first textless CID, 0x200B (B4d).
    let (pdf, _, _) = run("NotoSansArabic-Regular.ttf", "كِ");
    let segments = &pdf.pieces[0].segments;
    assert_eq!(segments.len(), 2);
    assert_eq!(segments[0].rise, -21.0);
    assert_eq!(segments[0].tj, [TjElement::Adjust(-268.0), codes(&[1])]);
    assert_eq!(segments[1].rise, 0.0);
    assert_eq!(segments[1].tj, [TjElement::Adjust(268.0), codes(&[0x200B, 2])]);
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
fn a_glyph_drawn_for_other_text_gets_a_second_cid() {
    // B4d: the `fi` ligature glyph drawn for "fi", then (a contrived run) for "f": two CIDs,
    // each with its own text, and no /ActualText.
    let font = common::font("NotoSans-Regular.ttf");
    let mut fonts = FontRegistry::new();
    let key = fonts.add(font.clone(), EmbedPolicy::Respect).unwrap();
    let shaped = HarfRust.shape(&font, "fi", &ShapeOptions::new()).unwrap();
    pdf_run(&shaped, "fi", &mut fonts, key).unwrap();
    let mut other = shaped.clone();
    other.text_range = 0..1;
    other.glyphs[0].cluster = 0;
    let pdf = pdf_run(&other, "fx", &mut fonts, key).unwrap();
    assert!(!pdf.needs_actual_text());
    let usage = fonts.usage(key).unwrap().embedded().unwrap();
    assert_eq!(
        (
            usage.text()[&1].as_str(),
            usage.text()[&2].as_str(),
            usage.glyphs()[1].1,
            usage.glyphs()[2].1
        ),
        ("fi", "f", shaped.glyphs[0].gid, shaped.glyphs[0].gid)
    );
}

#[test]
fn a_cff_glyph_already_standing_for_other_text_gets_actual_text() {
    let font = common::font("NotoSans-Regular.otf");
    let mut fonts = FontRegistry::new();
    let key = fonts.add(font.clone(), EmbedPolicy::Respect).unwrap();
    // The `fi` ligature glyph first drawn for "fi", then claimed for other text.
    let shaped = HarfRust.shape(&font, "fi", &ShapeOptions::new()).unwrap();
    pdf_run(&shaped, "fi", &mut fonts, key).unwrap();
    let mut other = shaped.clone();
    other.text_range = 0..1;
    other.glyphs[0].cluster = 0;
    let pdf = pdf_run(&other, "fx", &mut fonts, key).unwrap();
    assert_eq!(pdf.pieces[0].actual_text.as_deref(), Some("f"));
}

#[test]
fn a_textless_glyph_past_the_textless_cids_gets_actual_text() {
    // Review of B4d: 29 textless CIDs; a cluster whose 30th textless glyph has to take an
    // ordinary CID (which pdf.js would read as its code) is covered by /ActualText.
    let font = common::font("NotoSans-Regular.ttf");
    let mut shaped = HarfRust.shape(&font, "a", &ShapeOptions::new()).unwrap();
    let base = shaped.glyphs[0];
    let a = base.gid;
    shaped.glyphs.extend(
        (100..)
            .map(laminapdf_text::GlyphId::new)
            .filter(|&g| g != a)
            .take(30)
            .map(|gid| {
                let mut g = base;
                g.gid = gid;
                g.x_advance = 0;
                g
            }),
    );
    let mut fonts = FontRegistry::new();
    let key = fonts.add(font, EmbedPolicy::Respect).unwrap();
    let pdf = pdf_run(&shaped, "a", &mut fonts, key).unwrap();
    assert_eq!(pdf.pieces[0].actual_text.as_deref(), Some("a"));
}

#[test]
fn textless_glyphs_within_the_textless_cids_need_no_actual_text() {
    let font = common::font("NotoSans-Regular.ttf");
    let mut shaped = HarfRust.shape(&font, "a", &ShapeOptions::new()).unwrap();
    let mut extra = shaped.glyphs[0];
    extra.gid = laminapdf_text::GlyphId::new(100);
    extra.x_advance = 0;
    shaped.glyphs.push(extra);
    let mut fonts = FontRegistry::new();
    let key = fonts.add(font, EmbedPolicy::Respect).unwrap();
    assert!(!pdf_run(&shaped, "a", &mut fonts, key).unwrap().needs_actual_text());
}
