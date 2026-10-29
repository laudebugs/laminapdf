//! Type 0 embedding read back with lamina-cos: the font dictionaries, the font program, `/W`
//! against the subset's `hmtx`, subset tags and determinism (todo B4a).
#![allow(clippy::unwrap_used, reason = "tests: a panic is a test failure")]

mod common;

use common::samples::{self, Sample};
use common::{TestPdf, expand_widths, font, name_of, read_fonts};
use lamina_cos::Object;
use lamina_text::{Error, Font, FontRegistry, GlyphId};
use skrifa::MetadataProvider as _;
use skrifa::instance::{LocationRef, Size};

/// The goldens with embedded (Type 0) fonts: `text-std14.pdf` has simple fonts only
/// (`tests/std14.rs`).
fn all_samples() -> Vec<Sample> {
    samples::GOLDENS
        .iter()
        .filter(|(name, _, _)| *name != "text-std14.pdf")
        .map(|(_, build, _)| build())
        .collect()
}

fn is_subset_name(name: &str) -> bool {
    let (tag, rest) = name.split_at(7);
    tag.len() == 7 && tag[..6].bytes().all(|b| b.is_ascii_uppercase()) && tag.ends_with('+') && !rest.is_empty()
}

#[test]
fn same_text_twice_is_byte_identical() {
    assert_eq!(samples::latin().bytes, samples::latin().bytes);
}

#[test]
fn same_text_twice_has_the_same_subset_tags() {
    let names = |s: Sample| -> Vec<String> {
        read_fonts(&s.bytes)
            .iter()
            .map(|f| name_of(&f.type0, "BaseFont"))
            .collect()
    };
    assert_eq!(names(samples::cff()), names(samples::cff()));
}

#[test]
fn different_glyph_sets_have_different_tags() {
    let tag_for = |text: &str| {
        let mut pdf = TestPdf::new();
        let key = pdf.add_font(font("NotoSans-Regular.ttf")).unwrap();
        pdf.line(key, 12.0, text).unwrap();
        pdf.finish().unwrap().1[0].base_font.clone()
    };
    assert_ne!(tag_for("abc"), tag_for("abd"));
}

#[test]
fn base_font_carries_a_six_letter_subset_tag_everywhere() {
    for font in all_samples().iter().flat_map(|s| read_fonts(&s.bytes)) {
        let name = name_of(&font.cid_font, "BaseFont");
        assert!(is_subset_name(&name), "{name}");
        assert_eq!(name_of(&font.descriptor, "FontName"), name);
        // §9.7.6.1: the Type 0 name is the CIDFont's, plus "-Identity-H" for CIDFontType0.
        let type0 = name_of(&font.type0, "BaseFont");
        if name_of(&font.cid_font, "Subtype") == "CIDFontType0" {
            assert_eq!(type0, format!("{name}-Identity-H"));
        } else {
            assert_eq!(type0, name);
        }
    }
}

#[test]
fn type0_font_uses_identity_h_and_has_a_tounicode() {
    for font in read_fonts(&samples::latin().bytes) {
        assert_eq!(name_of(&font.type0, "Subtype"), "Type0");
        assert_eq!(name_of(&font.type0, "Encoding"), "Identity-H");
        assert!(font.type0.get("ToUnicode").is_some());
    }
}

#[test]
fn truetype_font_is_cidfonttype2_with_fontfile2_and_identity_map() {
    let fonts = read_fonts(&samples::latin().bytes);
    let f = common::named(&fonts, "NotoSans-Regular");
    assert_eq!(
        (
            name_of(&f.cid_font, "Subtype"),
            name_of(&f.cid_font, "CIDToGIDMap"),
            f.descriptor.get("FontFile2").is_some(),
            f.program_dict.get("Length1").and_then(Object::as_int),
        ),
        (
            "CIDFontType2".to_owned(),
            "Identity".to_owned(),
            true,
            Some(i64::try_from(f.program.len()).unwrap())
        )
    );
}

#[test]
fn truetype_subset_has_no_cmap_table() {
    // ISO 32000-1 §9.9: a TrueType program used by a CIDFont needs no cmap.
    let fonts = read_fonts(&samples::latin().bytes);
    let program = skrifa::FontRef::new(&common::named(&fonts, "NotoSans-Regular").program).unwrap();
    assert!(program.table_data(skrifa::Tag::new(b"cmap")).is_none());
}

#[test]
fn cff_font_is_cidfonttype0_with_a_bare_cff() {
    for f in read_fonts(&samples::cff().bytes) {
        assert_eq!(
            (
                name_of(&f.cid_font, "Subtype"),
                f.cid_font.get("CIDToGIDMap").is_none(),
                name_of(&f.program_dict, "Subtype"),
                f.program.first().copied(),
            ),
            ("CIDFontType0".to_owned(), true, "CIDFontType0C".to_owned(), Some(1))
        );
    }
}

#[test]
fn cff_programs_are_cid_keyed() {
    // A CID-keyed CFF's Top DICT starts with ROS (12 30, CFF spec §18): both the name-keyed
    // Noto Sans (converted by subsetter) and the CID-keyed Noto Sans SC.
    const ROS: u16 = 1230;
    for f in read_fonts(&samples::cff().bytes) {
        assert!(
            common::cff_top_dict_operators(&f.program).contains(&ROS),
            "{}",
            name_of(&f.cid_font, "BaseFont")
        );
    }
}

#[test]
fn cid_system_info_is_adobe_identity_0() {
    let fonts = read_fonts(&samples::latin().bytes);
    let f = common::named(&fonts, "NotoSans-Regular");
    let info = f.cid_font.get("CIDSystemInfo").unwrap().as_dict().unwrap();
    assert_eq!(
        String::from_utf8(info.get("Registry").unwrap().to_bytes()).unwrap()
            + &String::from_utf8(info.get("Ordering").unwrap().to_bytes()).unwrap()
            + &String::from_utf8(info.get("Supplement").unwrap().to_bytes()).unwrap(),
        "(Adobe)(Identity)0"
    );
}

/// Every CID the font's ToUnicode or `/W` mentions, with its width (`/W`, else `/DW`).
fn widths_of(f: &common::ReadFont) -> std::collections::BTreeMap<u16, f64> {
    let dw = f.cid_font.get("DW").and_then(Object::as_f64).unwrap_or(1000.0);
    let listed = f
        .cid_font
        .get("W")
        .map(|w| expand_widths(w.as_array().unwrap()))
        .unwrap_or_default();
    let (cmap, _) = common::parse_cmap(&f.to_unicode);
    std::iter::once(0)
        .chain(cmap.keys().copied())
        .chain(listed.keys().copied())
        .map(|cid| (cid, listed.get(&cid).copied().unwrap_or(dw)))
        .collect()
}

#[test]
fn widths_equal_the_subset_hmtx_advances_in_thousandths_of_an_em() {
    for sample in [samples::latin(), samples::variable_and_collection()] {
        for f in read_fonts(&sample.bytes) {
            let program = skrifa::FontRef::new(&f.program).unwrap();
            let upem = f64::from(skrifa::raw::TableProvider::head(&program).unwrap().units_per_em());
            let hmtx = program.glyph_metrics(Size::unscaled(), LocationRef::default());
            for (cid, width) in widths_of(&f) {
                let advance = f64::from(hmtx.advance_width(skrifa::GlyphId::from(cid)).unwrap());
                assert!(
                    (advance * 1000.0 / upem - width).abs() < 0.001,
                    "{} CID {cid}: /W {width}, hmtx {advance}/{upem}",
                    name_of(&f.type0, "BaseFont")
                );
            }
        }
    }
}

#[test]
fn cff_widths_equal_the_original_advances() {
    // The CFF program is embedded bare (no hmtx), so compare with the source font through the
    // CID → glyph mapping the registry recorded.
    let source = font("NotoSans-Regular.otf");
    let mut pdf = TestPdf::new();
    let key = pdf.add_font(source.clone()).unwrap();
    pdf.line(key, 12.0, "CFF widths: Ådne’s naïve café").unwrap();
    let glyphs = pdf.registry().usage(key).unwrap().embedded().unwrap().glyphs();
    let widths = widths_of(&read_fonts(&pdf.finish().unwrap().0)[0]);
    let upem = f64::from(source.info().units_per_em);
    for (cid, gid) in glyphs {
        let expected = f64::from(source.advance(gid).unwrap()) * 1000.0 / upem;
        assert!((widths[&cid] - expected).abs() < 0.001, "CID {cid}");
    }
}

#[test]
fn equal_cjk_widths_fold_into_dw() {
    let source = font("NotoSansSC-Regular.otf");
    let mut pdf = TestPdf::new();
    let key = pdf.add_font(source.clone()).unwrap();
    pdf.line(key, 12.0, samples::CFF[1]).unwrap();
    let glyphs = pdf.registry().usage(key).unwrap().embedded().unwrap().glyphs();
    let f = &read_fonts(&pdf.finish().unwrap().0)[0];
    let listed = f
        .cid_font
        .get("W")
        .map(|w| expand_widths(w.as_array().unwrap()))
        .unwrap_or_default();
    let widths = widths_of(f);
    let upem = f64::from(source.info().units_per_em);
    let cjk: Vec<(u16, f64)> = glyphs
        .iter()
        .skip(1)
        .map(|&(cid, gid)| (cid, f64::from(source.advance(gid).unwrap()) * 1000.0 / upem))
        .collect();
    assert_eq!(
        (
            f.cid_font.get("DW").and_then(Object::as_int),
            cjk.iter().all(|(cid, w)| (widths[cid] - w).abs() < 0.001),
            cjk.iter().filter(|(cid, _)| listed.contains_key(cid)).count(),
        ),
        (Some(1000), true, 0)
    );
}

#[test]
fn every_descriptor_has_the_required_metrics_and_x_height() {
    let keys = [
        "Flags",
        "FontBBox",
        "ItalicAngle",
        "Ascent",
        "Descent",
        "CapHeight",
        "XHeight",
        "StemV",
    ];
    for f in all_samples().iter().flat_map(|s| read_fonts(&s.bytes)) {
        let missing: Vec<_> = keys.iter().filter(|k| f.descriptor.get(k).is_none()).collect();
        assert!(missing.is_empty(), "{}: {missing:?}", name_of(&f.cid_font, "BaseFont"));
    }
}

#[test]
fn missing_glyph_reports_the_character_and_the_font() {
    let mut pdf = TestPdf::new();
    let key = pdf.add_font(font("NotoSansHebrew-Regular.ttf")).unwrap();
    assert_eq!(
        pdf.line(key, 12.0, "שלום 中").unwrap_err(),
        Error::MissingGlyph {
            ch: '中',
            font: "NotoSansHebrew-Regular".to_owned()
        }
    );
}

#[test]
fn missing_glyph_has_its_code() {
    let mut fonts = FontRegistry::new();
    let key = fonts
        .add(font("NotoSans-Regular.ttf"), lamina_text::EmbedPolicy::Respect)
        .unwrap();
    assert_eq!(fonts.encode(key, "\u{0915}").unwrap_err().code(), "missing-glyph");
}

#[test]
fn glyph_past_the_glyph_count_is_refused() {
    let f = font("NotoSansHebrew-Regular.ttf");
    let count = f.info().glyph_count;
    let mut fonts = FontRegistry::new();
    let key = fonts.add(f, lamina_text::EmbedPolicy::Respect).unwrap();
    assert_eq!(
        fonts.use_glyph(key, GlyphId::new(count), "x").unwrap_err().code(),
        "glyph-out-of-range"
    );
}

#[test]
fn key_from_another_registry_is_unknown() {
    let mut other = FontRegistry::new();
    let key = other
        .add(font("NotoSans-Regular.ttf"), lamina_text::EmbedPolicy::Respect)
        .unwrap();
    assert_eq!(
        FontRegistry::new().encode(key, "a").unwrap_err(),
        Error::UnknownFont { key: 0 }
    );
}

#[test]
fn unreferenced_font_writes_nothing() {
    let mut pdf = TestPdf::new();
    let used = pdf.add_font(font("NotoSans-Regular.ttf")).unwrap();
    pdf.add_font(font("NotoSansHebrew-Regular.ttf")).unwrap();
    pdf.line(used, 12.0, "only one").unwrap();
    assert_eq!(pdf.finish().unwrap().1.len(), 1);
}

#[test]
fn complex_scripts_embed_and_map_back_to_their_text() {
    // Unshaped (one glyph per character, B4b shapes): this checks embedding and ToUnicode.
    for (file, text) in [
        ("NotoSansArabic-Regular.ttf", "مرحبا بالعالم"),
        ("NotoSansHebrew-Regular.ttf", "שלום עולם"),
        ("NotoSansDevanagari-Regular.ttf", "नमस्ते दुनिया"),
    ] {
        let mut pdf = TestPdf::new();
        let key = pdf.add_font(font(file)).unwrap();
        let encoded = pdf.line(key, 12.0, text).unwrap();
        let (bytes, _, _) = pdf.finish().unwrap();
        let (cmap, _) = common::parse_cmap(&read_fonts(&bytes)[0].to_unicode);
        assert_eq!(common::extract(&encoded.bytes, &cmap), text, "{file}");
    }
}

#[test]
fn encode_advance_is_the_sum_of_glyph_advances() {
    let f: Font = font("NotoSans-Regular.ttf");
    let upem = f32::from(f.info().units_per_em);
    let expected: f32 = "AV"
        .chars()
        .map(|c| f.advance(f.glyph_for(c).unwrap()).unwrap() * 1000.0 / upem)
        .sum();
    let mut fonts = FontRegistry::new();
    let key = fonts.add(f, lamina_text::EmbedPolicy::Respect).unwrap();
    assert!((fonts.encode(key, "AV").unwrap().advance - f64::from(expected)).abs() < 0.01);
}

#[test]
fn embedded_font_counts_notdef_and_the_glyphs_drawn() {
    let mut pdf = TestPdf::new();
    let key = pdf.add_font(font("NotoSans-Regular.ttf")).unwrap();
    pdf.line(key, 12.0, "x").unwrap();
    let (_, embedded, _) = pdf.finish().unwrap();
    assert_eq!(embedded[0].glyphs, 2);
}
