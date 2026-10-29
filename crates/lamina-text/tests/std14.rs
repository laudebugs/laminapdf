//! The standard 14 fonts and simple-font encodings (todo B4c): metrics against the bundled AFM
//! files, the encoding tables, text encoding, and the simple font dictionaries lamina writes.
#![allow(
    clippy::unwrap_used,
    clippy::indexing_slicing,
    reason = "tests: a panic is a test failure"
)]

mod common;

use std::collections::BTreeMap;

use lamina_cos::{Document, Object, OpenOptions};
use lamina_text::{Encoding, Error, Font, FontRegistry, GlyphId, Standard14, agl};

use common::{TestPdf, fixtures_dir};

/// (code, name, width) of every glyph in an AFM file of `tools/std14-gen/data/afm/`.
fn afm(font: Standard14) -> Vec<(i32, String, u16)> {
    let path = fixtures_dir().join(format!("../tools/std14-gen/data/afm/{}.afm", font.base_font()));
    let text = std::fs::read_to_string(path).unwrap();
    text.lines()
        .filter(|l| l.starts_with("C "))
        .map(|l| {
            let fields: BTreeMap<&str, &str> = l.split(';').filter_map(|f| f.trim().split_once(' ')).collect();
            (
                fields["C"].parse().unwrap(),
                fields["N"].to_owned(),
                fields["WX"].parse().unwrap(),
            )
        })
        .collect()
}

const LATIN_ENCODINGS: [Encoding; 3] = [Encoding::Standard, Encoding::WinAnsi, Encoding::MacRoman];
const ALL_ENCODINGS: [Encoding; 6] = [
    Encoding::Standard,
    Encoding::WinAnsi,
    Encoding::MacRoman,
    Encoding::PdfDoc,
    Encoding::Symbol,
    Encoding::ZapfDingbats,
];

mod metrics {
    use super::*;

    #[test]
    fn every_glyph_width_matches_the_afm() {
        for font in Standard14::ALL {
            let mismatched: Vec<String> = afm(font)
                .into_iter()
                .filter(|(_, name, width)| font.width(name) != Some(*width))
                .map(|(_, name, _)| name)
                .collect();
            assert!(mismatched.is_empty(), "{font:?}: {mismatched:?}");
        }
    }

    #[test]
    fn every_font_has_exactly_the_afm_glyphs() {
        for font in Standard14::ALL {
            let mut names: Vec<String> = afm(font).into_iter().map(|(_, n, _)| n).collect();
            names.sort();
            assert_eq!(font.glyph_names(), names, "{font:?}");
        }
    }

    #[test]
    fn symbolic_built_in_encodings_are_the_afm_codes() {
        for (font, encoding) in [
            (Standard14::Symbol, Encoding::Symbol),
            (Standard14::ZapfDingbats, Encoding::ZapfDingbats),
            (Standard14::Helvetica, Encoding::Standard),
        ] {
            for (code, name, _) in afm(font).into_iter().filter(|(c, _, _)| *c >= 0) {
                let code = u8::try_from(code).unwrap();
                assert_eq!(encoding.glyph_name(code), Some(name.as_str()), "{encoding:?} {code}");
            }
        }
    }

    #[test]
    fn font_info_comes_from_the_afm_header() {
        let info = Font::standard(Standard14::TimesItalic).info().clone();
        assert_eq!(
            (
                info.ascent,
                info.descent,
                info.cap_height,
                info.italic_angle,
                info.flags
            ),
            (683.0, -217.0, Some(653.0), -15.5, 2 | 32 | 64)
        );
    }

    #[test]
    fn symbolic_fonts_fall_back_to_the_bbox_for_ascent_and_descent() {
        let info = Font::standard(Standard14::Symbol).info().clone();
        assert_eq!((info.ascent, info.descent, info.flags), (1010.0, -293.0, 4));
    }
}

mod encodings {
    use super::*;

    #[test]
    fn every_defined_code_maps_back_to_its_canonical_code() {
        for encoding in ALL_ENCODINGS {
            for code in 0..=u8::MAX {
                let Some(name) = encoding.glyph_name(code) else {
                    continue;
                };
                let canonical = encoding.code(name).unwrap();
                assert!(
                    canonical <= code && encoding.glyph_name(canonical) == Some(name),
                    "{encoding:?} {code}"
                );
            }
        }
    }

    #[test]
    fn win_ansi_duplicates_name_the_same_glyph() {
        let names: Vec<_> = [0o40, 0o240, 0o55, 0o255]
            .into_iter()
            .map(|c| Encoding::WinAnsi.glyph_name(c))
            .collect();
        assert_eq!(names, [Some("space"), Some("space"), Some("hyphen"), Some("hyphen")]);
    }

    #[test]
    fn win_ansi_duplicates_encode_to_the_lower_code() {
        assert_eq!(
            (Encoding::WinAnsi.code("space"), Encoding::WinAnsi.code("hyphen")),
            (Some(0o40), Some(0o55))
        );
    }

    #[test]
    fn mac_roman_space_duplicate_encodes_to_the_lower_code() {
        assert_eq!(
            (Encoding::MacRoman.glyph_name(0o312), Encoding::MacRoman.code("space")),
            (Some("space"), Some(0o40))
        );
    }

    #[test]
    fn no_glyph_name_encodes_to_an_undefined_code() {
        for encoding in ALL_ENCODINGS {
            let names = match encoding {
                Encoding::Symbol => Standard14::Symbol.glyph_names(),
                Encoding::ZapfDingbats => Standard14::ZapfDingbats.glyph_names(),
                _ => Standard14::Helvetica.glyph_names(),
            };
            for name in names {
                if let Some(code) = encoding.code(name) {
                    assert_eq!(encoding.glyph_name(code), Some(*name), "{encoding:?} {name}");
                }
            }
        }
    }

    #[test]
    fn undefined_codes_have_no_glyph() {
        let undefined: Vec<u8> = (0..=u8::MAX)
            .filter(|&c| Encoding::WinAnsi.glyph_name(c).is_none())
            .collect();
        let mut expected: Vec<u8> = (0..0x20).collect();
        expected.extend([0x7F, 0x81, 0x8D, 0x8F, 0x90, 0x9D]);
        assert_eq!(undefined, expected);
    }

    #[test]
    fn defined_code_counts_match_annex_d() {
        let defined = |e: Encoding| (0..=u8::MAX).filter(|&c| e.glyph_name(c).is_some()).count();
        // Annex D.2: StandardEncoding 149 codes, WinAnsiEncoding 216 glyphs + `space` at 240
        // and `hyphen` at 255 (218), MacRomanEncoding 207 glyphs + `space` at 312 (208).
        assert_eq!(
            LATIN_ENCODINGS.map(defined),
            [149, 218, 208],
            "Standard, WinAnsi, MacRoman"
        );
    }

    #[test]
    fn every_defined_name_has_a_unicode_value() {
        for encoding in ALL_ENCODINGS {
            for code in 0..=u8::MAX {
                if encoding.glyph_name(code).is_some() {
                    assert!(encoding.to_unicode(code).is_some(), "{encoding:?} {code}");
                }
            }
        }
    }

    #[test]
    fn win_ansi_matches_windows_1252_text() {
        // Every defined WinAnsiEncoding code above 0x9F is its Latin-1 character (Annex D.2;
        // 240 and 255 aside, whose glyphs are `space` and `hyphen`).
        let mismatched: Vec<u8> = (0xA1..=0xFF)
            .filter(|&c| c != 0xAD)
            .filter(|&c| Encoding::WinAnsi.to_unicode(c) != Some(char::from(c).to_string()))
            .collect();
        assert!(mismatched.is_empty(), "{mismatched:x?}");
    }

    #[test]
    fn pdf_doc_names_glyphs_of_the_latin_set() {
        assert_eq!(
            (Encoding::PdfDoc.glyph_name(0x84), Encoding::PdfDoc.code("emdash")),
            (Some("emdash"), Some(0x84))
        );
    }

    #[test]
    fn zapf_dingbats_names_map_through_their_own_list() {
        assert_eq!(agl::zapf_dingbats_to_unicode("a191").as_deref(), Some("\u{27BE}"));
    }
}

mod encode {
    use super::*;

    fn encode(font: Standard14, text: &str) -> lamina_text::Result<Vec<u8>> {
        let mut fonts = FontRegistry::new();
        let key = fonts.add(Font::standard(font), lamina_text::EmbedPolicy::Respect)?;
        Ok(fonts.encode(key, text)?.bytes)
    }

    #[test]
    fn euro_is_0x80_in_helvetica() {
        assert_eq!(encode(Standard14::Helvetica, "€").unwrap(), [0x80]);
    }

    #[test]
    fn euro_is_160_in_symbol() {
        assert_eq!(encode(Standard14::Symbol, "€").unwrap(), [160]);
    }

    #[test]
    fn eacute_is_missing_in_symbol() {
        assert_eq!(
            encode(Standard14::Symbol, "é").unwrap_err(),
            Error::MissingGlyph {
                ch: 'é',
                font: "Symbol".to_owned()
            }
        );
    }

    #[test]
    fn cjk_is_missing_in_helvetica() {
        assert_eq!(encode(Standard14::Helvetica, "中").unwrap_err().code(), "missing-glyph");
    }

    #[test]
    fn latin_letters_are_missing_in_symbol() {
        // Symbol's `A` is `Alpha`: Latin letters must not silently become Greek.
        assert_eq!(encode(Standard14::Symbol, "A").unwrap_err().code(), "missing-glyph");
    }

    #[test]
    fn greek_capital_alpha_is_a_in_symbol() {
        assert_eq!(encode(Standard14::Symbol, "Α").unwrap(), b"A");
    }

    #[test]
    fn no_break_space_is_the_space_glyph() {
        assert_eq!(encode(Standard14::Courier, "a\u{A0}b").unwrap(), b"a b");
    }

    #[test]
    fn glyphs_outside_win_ansi_take_undefined_codes() {
        assert_eq!(encode(Standard14::TimesRoman, "Łł").unwrap(), [0x81, 0x8D]);
    }

    #[test]
    fn advance_is_the_sum_of_afm_widths() {
        let mut fonts = FontRegistry::new();
        let key = fonts
            .add(Font::standard(Standard14::Helvetica), lamina_text::EmbedPolicy::Respect)
            .unwrap();
        // H 722, i 222.
        assert!((fonts.encode(key, "Hi").unwrap().advance - 944.0).abs() < f64::EPSILON);
    }

    #[test]
    fn use_glyph_returns_the_one_byte_code() {
        let mut fonts = FontRegistry::new();
        let font = Font::standard(Standard14::Helvetica);
        let gid = font.glyph_for('é').unwrap();
        let key = fonts.add(font, lamina_text::EmbedPolicy::Respect).unwrap();
        assert_eq!(fonts.use_glyph(key, gid, "é").unwrap(), 0xE9);
    }

    #[test]
    fn glyph_ids_past_the_glyph_count_are_rejected() {
        let mut fonts = FontRegistry::new();
        let key = fonts
            .add(Font::standard(Standard14::Courier), lamina_text::EmbedPolicy::Respect)
            .unwrap();
        assert_eq!(
            fonts.use_glyph(key, GlyphId::new(400), "").unwrap_err().code(),
            "glyph-out-of-range"
        );
    }

    #[test]
    fn running_out_of_codes_is_encoding_full() {
        // More glyphs outside WinAnsiEncoding than it has undefined codes.
        let font = Standard14::Helvetica;
        let text: String = font
            .glyph_names()
            .iter()
            .filter(|n| Encoding::WinAnsi.code(n).is_none())
            .filter_map(|n| agl::to_unicode(n))
            .collect();
        assert_eq!(encode(font, &text).unwrap_err().code(), "encoding-full");
    }

    #[test]
    fn standard_fonts_do_not_require_embedding() {
        assert!(
            Standard14::ALL
                .into_iter()
                .all(|f| !Font::standard(f).requires_embedding())
        );
    }

    #[test]
    fn loaded_fonts_require_embedding() {
        assert!(common::font("NotoSans-Regular.ttf").requires_embedding());
    }

    #[test]
    fn standard_fonts_have_no_axes() {
        let font = Font::standard(Standard14::Helvetica);
        assert_eq!(
            font.instance(&[(lamina_text::Tag::WEIGHT, 700.0)]).unwrap_err().code(),
            "unknown-axis"
        );
    }
}

mod written {
    use super::*;

    /// The font dictionaries of the first page of `pdf`, by `/BaseFont`.
    fn fonts(pdf: &[u8]) -> BTreeMap<String, (lamina_cos::Dict, lamina_cos::Dict)> {
        let doc = Document::open(pdf.to_vec(), OpenOptions::default()).unwrap();
        let page = doc.pages().unwrap().remove(0);
        let resources = page.resources_dict().unwrap().clone();
        let fonts = doc.deref(resources.get("Font").unwrap()).unwrap();
        fonts
            .as_dict()
            .unwrap()
            .iter()
            .map(|(_, r)| {
                let dict = doc.deref(r).unwrap().as_dict().unwrap().clone();
                let descriptor = doc
                    .deref(dict.get("FontDescriptor").unwrap())
                    .unwrap()
                    .as_dict()
                    .unwrap()
                    .clone();
                (common::name_of(&dict, "BaseFont"), (dict, descriptor))
            })
            .collect()
    }

    fn golden() -> BTreeMap<String, (lamina_cos::Dict, lamina_cos::Dict)> {
        fonts(&common::fixture("golden/text-std14.pdf"))
    }

    #[test]
    fn every_font_is_a_non_embedded_type1() {
        let fonts = golden();
        assert_eq!(fonts.len(), 14);
        for (name, (dict, descriptor)) in &fonts {
            assert_eq!(common::name_of(dict, "Subtype"), "Type1", "{name}");
            assert!(
                ["FontFile", "FontFile2", "FontFile3"]
                    .iter()
                    .all(|k| descriptor.get(k).is_none()),
                "{name}"
            );
        }
    }

    #[test]
    fn widths_are_the_afm_widths_of_each_code() {
        for (name, (dict, _)) in golden() {
            let font = Standard14::from_base_font(&name).unwrap();
            let first = u8::try_from(dict.get("FirstChar").unwrap().as_int().unwrap()).unwrap();
            let widths = dict.get("Widths").unwrap().as_array().unwrap();
            let encoding = font.text_encoding();
            for (code, width) in (first..).zip(widths) {
                // Codes /Differences assigns are checked by `differences_name_the_extra_glyphs`.
                let expected = encoding.glyph_name(code).and_then(|n| font.width(n));
                if let Some(expected) = expected {
                    assert_eq!(width.as_int(), Some(i64::from(expected)), "{name} {code}");
                }
            }
        }
    }

    #[test]
    fn latin_fonts_use_win_ansi_and_symbolic_ones_their_own() {
        let fonts = golden();
        let encoding = |name: &str| fonts[name].0.get("Encoding").cloned();
        assert_eq!(
            (encoding("Courier"), encoding("Symbol"), encoding("ZapfDingbats")),
            (Some(Object::name("WinAnsiEncoding")), None, None)
        );
    }

    #[test]
    fn differences_name_the_extra_glyphs() {
        let fonts = golden();
        let (dict, _) = &fonts["Helvetica"];
        let encoding = dict.get("Encoding").unwrap().as_dict().unwrap();
        let differences = encoding.get("Differences").unwrap().as_array().unwrap();
        let text = String::from_utf8(Object::Array(differences.to_vec()).to_bytes()).unwrap();
        assert_eq!(text, "[129 /Lslash 141 /zacute 143 /minus /Scedilla 157 /scedilla]");
    }

    #[test]
    fn descriptors_carry_the_afm_flags() {
        let fonts = golden();
        let flags = |name: &str| fonts[name].1.get("Flags").unwrap().as_int().unwrap();
        // FixedPitch 1, Serif 2, Symbolic 4, Nonsymbolic 32, Italic 64.
        assert_eq!(
            [
                flags("Courier-Oblique"),
                flags("Times-Bold"),
                flags("Helvetica"),
                flags("Symbol")
            ],
            [1 | 32 | 64, 2 | 32, 32, 4]
        );
    }

    #[test]
    fn a_font_drawn_with_nothing_still_has_a_valid_range() {
        let mut pdf = TestPdf::new();
        let key = pdf.add_font(Font::standard(Standard14::Courier)).unwrap();
        pdf.line(key, 12.0, "").unwrap();
        let (bytes, embedded, _) = pdf.finish().unwrap();
        let (dict, _) = &fonts(&bytes)["Courier"];
        assert_eq!(
            (dict.get("FirstChar"), dict.get("LastChar"), embedded[0].program_len),
            (Some(&Object::Integer(32)), Some(&Object::Integer(32)), 0)
        );
    }
}
