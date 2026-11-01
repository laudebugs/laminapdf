//! ToUnicode CMaps read back from generated files: block sizes, ligatures, surrogate pairs,
//! conflicts (ISO 32000-2 §9.10.3; todo B4a).
#![allow(clippy::unwrap_used, reason = "tests: a panic is a test failure")]

mod common;

use common::samples::{self, with_ligatures};
use common::{TestPdf, font, parse_cmap, read_fonts};
use laminapdf_text::{EmbedPolicy, FontRegistry, Warning};

/// Every character Noto Sans's fixture subset has a glyph for (over 300, so the CMap needs
/// several blocks).
fn many_chars() -> String {
    let f = font("NotoSans-Regular.ttf");
    (0x20u32..0x2100)
        .filter_map(char::from_u32)
        .filter(|&c| f.glyph_for(c).is_some())
        .collect()
}

#[test]
fn blocks_hold_at_most_100_entries() {
    let mut pdf = TestPdf::new();
    let key = pdf.add_font(font("NotoSans-Regular.ttf")).unwrap();
    pdf.line(key, 6.0, &many_chars()).unwrap();
    let (_, blocks) = parse_cmap(&read_fonts(&pdf.finish().unwrap().0)[0].to_unicode);
    assert!(
        blocks.len() > 1 && blocks.iter().all(|&n| (1..=100).contains(&n)),
        "{blocks:?}"
    );
}

#[test]
fn every_character_round_trips_through_the_cmap() {
    let text = many_chars();
    let mut pdf = TestPdf::new();
    let key = pdf.add_font(font("NotoSans-Regular.ttf")).unwrap();
    let encoded = pdf.line(key, 6.0, &text).unwrap();
    let (cmap, _) = parse_cmap(&read_fonts(&pdf.finish().unwrap().0)[0].to_unicode);
    assert_eq!(common::extract(&encoded.bytes, &cmap), text);
}

#[test]
fn ligature_glyphs_map_to_their_letters() {
    let sans = font("NotoSans-Regular.ttf");
    let glyphs = with_ligatures(&sans, "ffi fi");
    let mut pdf = TestPdf::new();
    let key = pdf.add_font(sans).unwrap();
    let refs: Vec<_> = glyphs.iter().map(|(g, t)| (*g, t.as_str())).collect();
    pdf.glyphs(key, 12.0, &refs).unwrap();
    let cmap = &read_fonts(&pdf.finish().unwrap().0)[0].to_unicode;
    assert!(cmap.contains("<006600660069>") && cmap.contains("<00660069>"), "{cmap}");
}

#[test]
fn non_bmp_character_maps_to_a_surrogate_pair() {
    let fonts = read_fonts(&samples::latin().bytes);
    assert!(fonts.iter().any(|f| f.to_unicode.contains("<D835DC00>")));
}

#[test]
fn a_glyph_drawn_with_two_texts_gets_two_cids() {
    // B4d: a space glyph used for U+0020 and for U+00A0: two CIDs, one glyph, a
    // /CIDToGIDMap stream, no warning.
    let sans = font("NotoSans-Regular.ttf");
    let space = sans.glyph_for(' ').unwrap();
    let mut pdf = TestPdf::new();
    let key = pdf.add_font(sans).unwrap();
    pdf.glyphs(key, 12.0, &[(space, " "), (space, "\u{A0}")]).unwrap();
    let (bytes, _, warnings) = pdf.finish().unwrap();
    let fonts = read_fonts(&bytes);
    let (cmap, _) = parse_cmap(&fonts[0].to_unicode);
    assert_eq!(
        (cmap.get(&1).cloned(), cmap.get(&2).cloned(), warnings),
        (Some(" ".to_owned()), Some("\u{A0}".to_owned()), vec![])
    );
}

#[test]
fn conflicting_text_keeps_the_first_and_warns() {
    // CFF outlines: one CID per glyph (B4d), so the first text is kept.
    let sans = font("NotoSans-Regular.otf");
    let space = sans.glyph_for(' ').unwrap();
    let mut pdf = TestPdf::new();
    let key = pdf.add_font(sans).unwrap();
    // A space glyph used for U+0020 and for U+00A0.
    pdf.glyphs(key, 12.0, &[(space, " "), (space, "\u{A0}")]).unwrap();
    let (bytes, _, warnings) = pdf.finish().unwrap();
    let (cmap, _) = parse_cmap(&read_fonts(&bytes)[0].to_unicode);
    assert_eq!(
        (cmap.get(&1).cloned(), warnings),
        (
            Some(" ".to_owned()),
            vec![Warning::ToUnicodeConflict {
                font: "NotoSans-Regular".to_owned(),
                gid: space.get(),
                kept: " ".to_owned(),
                dropped: "\u{A0}".to_owned(),
            }]
        )
    );
}

#[test]
fn conflict_warning_has_its_code() {
    let sans = font("NotoSans-Regular.otf");
    let a = sans.glyph_for('a').unwrap();
    let mut fonts = FontRegistry::new();
    let key = fonts.add(sans, EmbedPolicy::Respect).unwrap();
    fonts.use_glyph(key, a, "a").unwrap();
    fonts.use_glyph(key, a, "b").unwrap();
    assert_eq!(fonts.warnings()[0].code(), "tounicode-conflict");
}

#[test]
fn glyph_first_used_without_text_keeps_it_and_reports_the_later_text() {
    // B4b review B1: the empty text is a text; a later different one is a conflict (CFF).
    let sans = font("NotoSans-Regular.otf");
    let a = sans.glyph_for('a').unwrap();
    let mut fonts = FontRegistry::new();
    let key = fonts.add(sans, EmbedPolicy::Respect).unwrap();
    fonts.use_glyph(key, a, "").unwrap();
    let cid = fonts.use_glyph(key, a, "a").unwrap().cid().unwrap();
    assert_eq!(
        (
            fonts.usage(key).unwrap().embedded().unwrap().text().get(&cid).cloned(),
            fonts.warnings().len()
        ),
        (Some(String::new()), 1)
    );
}

#[test]
fn notdef_has_no_mapping() {
    let (cmap, _) = parse_cmap(&read_fonts(&samples::latin().bytes)[0].to_unicode);
    assert!(!cmap.contains_key(&0));
}
