//! Regression tests for the first `/code-review` round of PR #48 (todo B4a). Each test names
//! the finding it covers.
#![allow(
    clippy::unwrap_used,
    clippy::indexing_slicing,
    clippy::cast_possible_truncation,
    reason = "tests: a panic is a test failure; values are small font units"
)]

mod common;

use common::{TestPdf, fixture, font, name_of, patch_table, read_fonts, with_fs_type};
use lamina_text::{EmbedPolicy, Error, Font, FontRegistry, Tag};
use skrifa::MetadataProvider as _;
use skrifa::instance::{LocationRef, Size};

fn variable_bytes() -> Vec<u8> {
    fixture("fonts/NotoSans-VF.ttf")
}

/// The VF fixture with its first `fvar` axis's minimum raised above its maximum.
fn inverted_axis() -> Vec<u8> {
    let bytes = variable_bytes();
    let fvar = common::table_offset(&bytes, *b"fvar");
    let axes = usize::from(u16::from_be_bytes([bytes[fvar + 4], bytes[fvar + 5]]));
    // Axis record: tag, minValue (Fixed 16.16), defaultValue, maxValue, flags, nameID.
    let min = 2000i32 << 16;
    patch_table(&bytes, *b"fvar", axes + 4, &min.to_be_bytes())
}

// A1
#[test]
fn malformed_axis_range_is_an_error_not_a_panic() {
    let font = Font::from_bytes(inverted_axis()).unwrap();
    let tag = font.info().axes[0].tag;
    assert_eq!(font.instance(&[(tag, 500.0)]).unwrap_err().code(), "invalid-font");
}

// A2
#[test]
fn non_finite_axis_value_is_refused() {
    let font = font("NotoSans-VF.ttf");
    assert_eq!(
        font.instance(&[(Tag::WEIGHT, f32::NAN)]).unwrap_err().code(),
        "invalid-axis-value"
    );
}

// A2
#[test]
fn infinite_axis_value_is_refused() {
    let font = font("NotoSans-VF.ttf");
    assert!(font.instance(&[(Tag::WEIGHT, f32::INFINITY)]).is_err());
}

// A3
#[test]
fn instance_bbox_is_the_union_of_the_instanced_glyph_bounds() {
    let heavy = font("NotoSans-VF.ttf").instance(&[(Tag::WEIGHT, 900.0)]).unwrap();
    let mut pdf = TestPdf::new();
    let key = pdf.add_font(heavy).unwrap();
    pdf.line(key, 12.0, "WMgj").unwrap();
    let f = &read_fonts(&pdf.finish().unwrap().0)[0];
    let program = skrifa::FontRef::new(&f.program).unwrap();
    let upem = f32::from(skrifa::raw::TableProvider::head(&program).unwrap().units_per_em());
    let metrics = program.glyph_metrics(Size::unscaled(), LocationRef::default());
    let mut bbox = [f32::MAX, f32::MAX, f32::MIN, f32::MIN];
    for gid in 1..u32::from(skrifa::raw::TableProvider::maxp(&program).unwrap().num_glyphs()) {
        if let Some(b) = metrics.bounds(skrifa::GlyphId::new(gid)) {
            bbox = [
                bbox[0].min(b.x_min),
                bbox[1].min(b.y_min),
                bbox[2].max(b.x_max),
                bbox[3].max(b.y_max),
            ];
        }
    }
    let expected: Vec<i64> = bbox.iter().map(|v| (v * 1000.0 / upem).round() as i64).collect();
    let written: Vec<i64> = f
        .descriptor
        .get("FontBBox")
        .unwrap()
        .as_array()
        .unwrap()
        .iter()
        .map(|o| o.as_int().unwrap())
        .collect();
    assert_eq!(written, expected);
}

// A4
#[test]
fn instance_at_the_defaults_is_the_default_instance() {
    let regular = font("NotoSans-VF.ttf");
    let same = regular.instance(&[(Tag::WEIGHT, 400.0), (Tag::WIDTH, 100.0)]).unwrap();
    assert_eq!(
        (same.coordinates().len(), same.info().postscript_name.as_str()),
        (0, regular.info().postscript_name.as_str())
    );
}

// A6
#[test]
fn unreadable_collection_reports_the_requested_index() {
    assert_eq!(
        Font::from_collection(b"garbage".to_vec(), 3).unwrap_err(),
        Error::InvalidFont { index: 3 }
    );
}

// A9
#[test]
fn finishing_into_another_document_is_refused() {
    let base = lamina_cos::Document::open(fixture("corpus/hello.pdf"), lamina_cos::OpenOptions::default()).unwrap();
    let mut fonts = FontRegistry::new();
    let key = fonts.add(font("NotoSans-Regular.ttf"), EmbedPolicy::Respect).unwrap();
    // Reserve in a document with few objects, then finish into one where that number exists.
    let empty = common::empty_document();
    let reserved = fonts.reference(key, &empty).unwrap();
    fonts.encode(key, "x").unwrap();
    let mut other = base;
    assert!(other.resolve(reserved).is_ok());
    assert_eq!(
        fonts.finish_fonts(&mut other.edit()).unwrap_err().code(),
        "wrong-document"
    );
}

// A10
#[test]
fn no_subsetting_truetype_font_is_embedded_byte_for_byte() {
    let bytes = with_fs_type(&fixture("fonts/NotoSans-Regular.ttf"), 0x0100);
    let mut pdf = TestPdf::new();
    let key = pdf.add_font(Font::from_bytes(bytes.clone()).unwrap()).unwrap();
    pdf.line(key, 12.0, "whole").unwrap();
    let embedded = read_fonts(&pdf.finish().unwrap().0).remove(0).program;
    assert!(embedded == bytes, "the embedded program differs from the font file");
}

// A10
#[test]
fn no_subsetting_cff_font_is_embedded_as_opentype() {
    let bytes = with_fs_type(&fixture("fonts/NotoSans-Regular.otf"), 0x0100);
    let mut pdf = TestPdf::new();
    let key = pdf.add_font(Font::from_bytes(bytes.clone()).unwrap()).unwrap();
    pdf.line(key, 12.0, "whole").unwrap();
    let f = &read_fonts(&pdf.finish().unwrap().0)[0];
    assert_eq!(
        (name_of(&f.program_dict, "Subtype"), f.program == bytes),
        ("OpenType".to_owned(), true)
    );
}

// A10
#[test]
fn no_subsetting_collection_member_is_embedded_whole() {
    let ttc = fixture("fonts/NotoSans-RegularBold.ttc");
    let member = Font::from_collection(ttc, 1).unwrap();
    let count = member.info().glyph_count;
    let patched = {
        // fsType lives in the member's own OS/2 table; patch through the member's directory.
        let bytes = member.bytes().to_vec();
        let dir = u32::from_be_bytes(bytes[16..20].try_into().unwrap()) as usize;
        let tables = usize::from(u16::from_be_bytes([bytes[dir + 4], bytes[dir + 5]]));
        let mut out = bytes.clone();
        for i in 0..tables {
            let r = dir + 12 + 16 * i;
            if &bytes[r..r + 4] == b"OS/2" {
                let off = u32::from_be_bytes(bytes[r + 8..r + 12].try_into().unwrap()) as usize;
                out[off + 8..off + 10].copy_from_slice(&0x0100u16.to_be_bytes());
            }
        }
        Font::from_collection(out, 1).unwrap()
    };
    let mut pdf = TestPdf::new();
    let key = pdf.add_font(patched).unwrap();
    pdf.line(key, 12.0, "ttc").unwrap();
    let f = &read_fonts(&pdf.finish().unwrap().0)[0];
    let program = skrifa::FontRef::new(&f.program).unwrap();
    assert_eq!(
        (
            skrifa::raw::TableProvider::maxp(&program).unwrap().num_glyphs(),
            program.table_data(skrifa::Tag::new(b"cmap")).is_some()
        ),
        (count, true)
    );
}

// A11
#[test]
fn encode_records_nothing_when_a_glyph_is_out_of_range() {
    // maxp.numGlyphs lowered to 4 while the cmap still maps letters past it.
    let bytes = patch_table(&fixture("fonts/NotoSans-Regular.ttf"), *b"maxp", 4, &4u16.to_be_bytes());
    let Ok(font) = Font::from_bytes(bytes) else {
        return; // a reader that refuses such a font is fine too
    };
    let space = font.glyph_for(' ');
    let mut fonts = FontRegistry::new();
    let key = fonts.add(font, EmbedPolicy::Respect).unwrap();
    let result = fonts.encode(key, " A");
    assert_eq!(
        (
            result.is_err(),
            fonts.usage(key).unwrap().embedded().unwrap().glyphs().len(),
            space.is_some()
        ),
        (true, 1, true)
    );
}

// A13
#[test]
fn cff_type0_base_font_appends_the_cmap_name() {
    let fonts = read_fonts(&common::samples::cff().bytes);
    let f = fonts
        .iter()
        .find(|f| name_of(&f.cid_font, "BaseFont").ends_with("+NotoSans-Regular"))
        .unwrap();
    assert_eq!(
        name_of(&f.type0, "BaseFont"),
        format!("{}-Identity-H", name_of(&f.cid_font, "BaseFont"))
    );
}

// A13
#[test]
fn truetype_type0_base_font_equals_the_cidfont_name() {
    let f = &read_fonts(&common::samples::latin().bytes)[0];
    assert_eq!(name_of(&f.type0, "BaseFont"), name_of(&f.cid_font, "BaseFont"));
}

// A14
#[test]
fn key_from_a_non_empty_foreign_registry_is_unknown() {
    let mut mine = FontRegistry::new();
    mine.add(font("NotoSansHebrew-Regular.ttf"), EmbedPolicy::Respect)
        .unwrap();
    let mut other = FontRegistry::new();
    let key = other.add(font("NotoSans-Regular.ttf"), EmbedPolicy::Respect).unwrap();
    assert_eq!(mine.encode(key, "a").unwrap_err().code(), "unknown-font");
}

// A10
#[test]
fn no_subsetting_instance_is_rebuilt_whole_with_a_warning() {
    let bytes = with_fs_type(&variable_bytes(), 0x0100);
    let bold = Font::from_bytes(bytes)
        .unwrap()
        .instance(&[(Tag::WEIGHT, 700.0)])
        .unwrap();
    let count = bold.info().glyph_count;
    let mut pdf = TestPdf::new();
    let key = pdf.add_font(bold).unwrap();
    pdf.line(key, 12.0, "whole").unwrap();
    let (bytes, embedded, _) = pdf.finish().unwrap();
    let program = read_fonts(&bytes).remove(0).program;
    let parsed = skrifa::FontRef::new(&program).unwrap();
    assert_eq!(
        (
            embedded[0]
                .warnings
                .iter()
                .map(lamina_text::Warning::code)
                .collect::<Vec<_>>(),
            skrifa::raw::TableProvider::maxp(&parsed).unwrap().num_glyphs(),
        ),
        (vec!["whole-font-rebuilt"], count)
    );
}
