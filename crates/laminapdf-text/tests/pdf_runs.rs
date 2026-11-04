//! Shaped runs written into a PDF: a glyph gets one CID per text it stands for, default
//! ignorables and textless glyphs carry no text, `TJ` widths match `/W` for a variable instance,
//! a run from another font is refused without side effects, and a missing glyph names its
//! character.
#![allow(
    clippy::unwrap_used,
    clippy::indexing_slicing,
    clippy::panic,
    reason = "tests: a panic is a test failure"
)]

mod common;

use common::{TestPdf, expand_widths, font, read_fonts};
use laminapdf_text::{
    EmbedPolicy, Error, FontRegistry, HarfRust, LayoutOptions, ShapeOptions, Shaper, Tag, TjElement, Warning, pdf_run,
};

fn latin() -> laminapdf_text::Font {
    font("NotoSans-Regular.ttf")
}

/// Empty text is a text like any other: in a CFF font (one CID per glyph), a glyph
/// first used for nothing keeps that, and a later real text is a conflict (not a silent
/// overwrite).
#[test]
fn empty_then_real_text_conflicts() {
    let font = font("NotoSans-Regular.otf");
    let a = font.glyph_for('a').unwrap();
    let mut fonts = FontRegistry::new();
    let key = fonts.add(font, EmbedPolicy::Respect).unwrap();
    let cid = fonts.use_glyph(key, a, "").unwrap();
    fonts.use_glyph(key, a, "a").unwrap();
    assert_eq!(
        fonts
            .usage(key)
            .unwrap()
            .embedded()
            .unwrap()
            .text()
            .get(&cid.cid().unwrap())
            .map(String::as_str),
        Some("")
    );
    assert!(matches!(fonts.warnings(), [Warning::ToUnicodeConflict { .. }]));
}

/// A real text then an empty one is a conflict too (CFF).
#[test]
fn real_then_empty_text_conflicts() {
    let font = font("NotoSans-Regular.otf");
    let a = font.glyph_for('a').unwrap();
    let mut fonts = FontRegistry::new();
    let key = fonts.add(font, EmbedPolicy::Respect).unwrap();
    fonts.use_glyph(key, a, "a").unwrap();
    fonts.use_glyph(key, a, "").unwrap();
    assert!(matches!(fonts.warnings(), [Warning::ToUnicodeConflict { .. }]));
}

/// With TrueType outlines, one CID per (glyph, text): the empty and the real text each get a CID,
/// with no warning.
#[test]
fn empty_and_real_text_get_two_cids_in_a_truetype_font() {
    let font = latin();
    let a = font.glyph_for('a').unwrap();
    let mut fonts = FontRegistry::new();
    let key = fonts.add(font, EmbedPolicy::Respect).unwrap();
    let empty = fonts.use_glyph(key, a, "").unwrap().cid().unwrap();
    let real = fonts.use_glyph(key, a, "a").unwrap().cid().unwrap();
    let text = fonts.usage(key).unwrap().embedded().unwrap().text().clone();
    assert_eq!(
        (text[&empty].as_str(), text[&real].as_str(), fonts.warnings().len()),
        ("", "a", 0)
    );
}

/// A default-ignorable character (ZWJ, drawn as a hidden space glyph) never becomes a
/// glyph's ToUnicode text, and the space glyph keeps " ".
#[test]
fn default_ignorables_get_no_glyph_text() {
    let font = latin();
    let space = font.glyph_for(' ').unwrap();
    let mut fonts = FontRegistry::new();
    let key = fonts.add(font.clone(), EmbedPolicy::Respect).unwrap();
    for text in ["a\u{200D}b", "a b"] {
        let run = HarfRust.shape(&font, text, &ShapeOptions::new()).unwrap();
        pdf_run(&run, text, &mut fonts, key).unwrap();
    }
    let usage = fonts.usage(key).unwrap().embedded().unwrap();
    assert!(
        usage.text().values().all(|t| !t.contains('\u{200D}')),
        "{:?}",
        usage.text()
    );
    let space_cid = usage.glyphs().iter().find(|(_, g)| *g == space).unwrap().0;
    assert_eq!(usage.text()[&space_cid], " ");
}

/// A glyph that stands for no text (Noto Sans Arabic's separate dots) is
/// drawn with a CID that is a format character's code point (pdf.js, which reads an empty
/// ToUnicode destination as the code, then skips it), mapped to `<>`, and its cluster needs
/// no `/ActualText`.
#[test]
fn textless_glyphs_get_format_character_cids_and_no_actual_text() {
    let font = font("NotoSansArabic-Regular.ttf");
    let text = "بالعالم";
    let run = HarfRust.shape(&font, text, &ShapeOptions::new()).unwrap();
    let mut fonts = FontRegistry::new();
    let key = fonts.add(font, EmbedPolicy::Respect).unwrap();
    let pdf = pdf_run(&run, text, &mut fonts, key).unwrap();
    let empty: Vec<u16> = fonts
        .usage(key)
        .unwrap()
        .embedded()
        .unwrap()
        .text()
        .iter()
        .filter(|(_, t)| t.is_empty())
        .map(|(c, _)| *c)
        .collect();
    assert!(!empty.is_empty(), "the fixture draws the beh's dot as its own glyph");
    assert!(
        empty.iter().all(|&c| (0x200B..=0x200F).contains(&c)) && !pdf.needs_actual_text(),
        "{empty:X?} {pdf:?}"
    );
}

/// The `TJ` corrections use the widths `/W` is written with, also for a variable-font
/// instance: drawing a run then reading `/W` back gives the shaper's advances.
#[test]
fn tj_widths_match_w_for_a_variable_instance() {
    let instance = || font("NotoSans-VF.ttf").instance(&[(Tag::WEIGHT, 700.0)]).unwrap();
    let text = "Wave AVATAR office";
    let run = HarfRust.shape(&instance(), text, &ShapeOptions::new()).unwrap();
    let mut pdf = TestPdf::new();
    let key = pdf.add_font(instance()).unwrap();
    let lines = pdf.paragraph(&[key], text, &LayoutOptions::new(500.0, 10.0)).unwrap();
    assert_eq!(lines, [text]);
    let (bytes, _, _) = pdf.finish().unwrap();
    let fonts = read_fonts(&bytes);
    let widths = expand_widths(fonts[0].cid_font.get("W").unwrap().as_array().unwrap());
    let dw = fonts[0].cid_font.get("DW").map_or(1000.0, |d| d.as_f64().unwrap());
    // Recompute the pen from /W and the TJ numbers: it must land on the shaper's advance.
    let mut registry = FontRegistry::new();
    let key = registry.add(instance(), EmbedPolicy::Respect).unwrap();
    let pdf = pdf_run(&run, text, &mut registry, key).unwrap();
    let mut pen = 0.0;
    for element in pdf.pieces.iter().flat_map(|p| &p.segments).flat_map(|s| &s.tj) {
        match element {
            TjElement::Codes(codes) => {
                for pair in codes.chunks(2) {
                    let cid = u16::from_be_bytes([pair[0], pair[1]]);
                    pen += widths.get(&cid).copied().unwrap_or(dw);
                }
            }
            TjElement::Adjust(a) => pen -= a,
        }
    }
    let shaped: f64 =
        run.glyphs.iter().map(|g| f64::from(g.x_advance)).sum::<f64>() * 1000.0 / f64::from(run.units_per_em);
    assert!((pen - shaped).abs() < 1e-6, "{pen} vs {shaped}");
}

/// A run shaped with another font is refused before anything is recorded.
#[test]
fn runs_from_another_font_are_refused_without_side_effects() {
    let hebrew = font("NotoSansHebrew-Regular.ttf");
    let run = HarfRust.shape(&hebrew, "שלום", &ShapeOptions::new()).unwrap();
    let mut fonts = FontRegistry::new();
    let key = fonts.add(latin(), EmbedPolicy::Respect).unwrap();
    let err = pdf_run(&run, "שלום", &mut fonts, key).unwrap_err();
    assert_eq!(err.code(), "font-mismatch");
    assert_eq!(
        fonts.usage(key).unwrap().embedded().unwrap().glyphs().len(),
        1,
        "only .notdef"
    );
}

/// A missing glyph names the character the font lacks, not the cluster's first.
#[test]
fn missing_glyph_names_the_missing_character() {
    let font = latin();
    let text = "a\u{0360}";
    let run = HarfRust.shape(&font, text, &ShapeOptions::new()).unwrap();
    assert!(run.has_notdef());
    let mut fonts = FontRegistry::new();
    let key = fonts.add(font, EmbedPolicy::Respect).unwrap();
    match pdf_run(&run, text, &mut fonts, key).unwrap_err() {
        Error::MissingGlyph { ch, .. } => assert_eq!(ch, '\u{0360}'),
        other => panic!("{other:?}"),
    }
}
