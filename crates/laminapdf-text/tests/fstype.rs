//! `OS/2.fsType`: restricted fonts need an explicit override, "no subsetting" fonts are
//! embedded whole (research 03 §6.3; todo B4a).
#![allow(clippy::unwrap_used, reason = "tests: a panic is a test failure")]

mod common;

use common::{TestPdf, fixture, name_of, read_fonts, with_fs_type};
use laminapdf_text::{EmbedPolicy, Error, Font, FsType};

fn noto_with(fs_type: u16) -> Font {
    Font::from_bytes(with_fs_type(&fixture("fonts/NotoSans-Regular.ttf"), fs_type)).unwrap()
}

#[test]
fn restricted_font_fails_without_override() {
    let mut pdf = TestPdf::new();
    assert_eq!(
        pdf.add_font(noto_with(0x0002)).unwrap_err(),
        Error::FontNotEmbeddable {
            font: "NotoSans-Regular".to_owned(),
            fs_type: FsType::new(0x0002)
        }
    );
}

#[test]
fn restricted_font_error_has_its_code() {
    assert_eq!(
        TestPdf::new().add_font(noto_with(0x0002)).unwrap_err().code(),
        "font-not-embeddable"
    );
}

#[test]
fn restricted_font_embeds_with_override() {
    let mut pdf = TestPdf::new();
    let key = pdf.add_font_with(noto_with(0x0002), EmbedPolicy::Override).unwrap();
    pdf.line(key, 12.0, "licensed").unwrap();
    assert_eq!(read_fonts(&pdf.finish().unwrap().0).len(), 1);
}

#[test]
fn bitmap_only_font_fails_without_override() {
    assert!(matches!(
        TestPdf::new().add_font(noto_with(0x0200)),
        Err(Error::FontNotEmbeddable { .. })
    ));
}

#[test]
fn editable_font_embeds() {
    let mut pdf = TestPdf::new();
    let key = pdf.add_font(noto_with(0x0008)).unwrap();
    pdf.line(key, 12.0, "editable").unwrap();
    assert!(pdf.finish().is_ok());
}

/// A "no subsetting" font, embedded with `text`: (file, the font).
fn whole(text: &str) -> (Vec<u8>, Font) {
    let font = noto_with(0x0100);
    let mut pdf = TestPdf::new();
    let key = pdf.add_font(font.clone()).unwrap();
    pdf.line(key, 12.0, text).unwrap();
    (pdf.finish().unwrap().0, font)
}

#[test]
fn no_subsetting_font_keeps_every_glyph() {
    let (bytes, font) = whole("whole");
    let fonts = read_fonts(&bytes);
    let program = skrifa::FontRef::new(&fonts[0].program).unwrap();
    assert_eq!(
        skrifa::raw::TableProvider::maxp(&program).unwrap().num_glyphs(),
        font.info().glyph_count
    );
}

#[test]
fn no_subsetting_font_has_no_subset_tag() {
    let (bytes, _) = whole("whole");
    assert_eq!(name_of(&read_fonts(&bytes)[0].type0, "BaseFont"), "NotoSans-Regular");
}

#[test]
fn no_subsetting_font_draws_original_glyph_ids() {
    let (bytes, font) = whole("w");
    let (cmap, _) = common::parse_cmap(&read_fonts(&bytes)[0].to_unicode);
    let gid = font.glyph_for('w').unwrap().get();
    assert_eq!(cmap.get(&gid).map(String::as_str), Some("w"));
}
