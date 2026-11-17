//! `Page::fonts` finds the fonts a page draws with through a tiling pattern and a soft mask's
//! group, not only through its own resources and forms, and reads each font's name, encoding
//! and ToUnicode CMap as the file declares them.
#![allow(clippy::unwrap_used, reason = "tests: a panic is a test failure")]

use laminapdf_cos::OpenOptions;
use laminapdf_model::{Document, FontEncoding, FontResource};
use laminapdf_test_fixtures::{classic_with_root, stream_body};

/// A page that draws text only through a tiling pattern and a soft mask's group, each with
/// its own font.
fn hidden_fonts() -> Vec<u8> {
    let font = |name: &str| format!("<< /Type /Font /Subtype /Type1 /BaseFont /{name} >>");
    let pattern = stream_body(
        &format!(
            "/PatternType 1 /PaintType 1 /TilingType 1 /BBox [0 0 10 10] /XStep 10 /YStep 10 \
             /Resources << /Font << /F1 {} >> >>",
            font("Helvetica")
        ),
        b"BT /F1 8 Tf (P) Tj ET",
    );
    let group = stream_body(
        &format!(
            "/Type /XObject /Subtype /Form /BBox [0 0 10 10] /Group << /S /Transparency >> \
             /Resources << /Font << /F1 {} >> >>",
            font("Courier")
        ),
        b"BT /F1 8 Tf (M) Tj ET",
    );
    let page = "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 100] /Resources << \
                /Pattern << /P1 4 0 R >> /ExtGState << /G1 << /SMask << /S /Luminosity /G 5 0 R >> >> >> >> >>";
    let objects = [
        "<< /Type /Catalog /Pages 2 0 R >>".to_owned(),
        "<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_owned(),
        page.to_owned(),
        String::from_utf8(pattern).unwrap(),
        String::from_utf8(group).unwrap(),
    ];
    let refs: Vec<&str> = objects.iter().map(String::as_str).collect();
    classic_with_root(&refs, "")
}

#[test]
fn fonts_in_a_pattern_and_a_soft_masks_group_are_listed() {
    let doc = Document::open(hidden_fonts(), OpenOptions::default()).unwrap();
    let pages = doc.pages().unwrap();
    let names: Vec<Option<String>> = pages
        .get(0)
        .unwrap()
        .fonts()
        .unwrap()
        .into_iter()
        .map(|f| f.name)
        .collect();
    assert_eq!(names, [Some("Helvetica".to_owned()), Some("Courier".to_owned())]);
}

/// The fonts of a one-page document whose page resources are `/Font << fonts >>`, followed by
/// `objects` (numbered from 4).
fn fonts_of(fonts: &str, objects: &[&str]) -> Vec<FontResource> {
    let page = format!("<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 100] /Resources << /Font << {fonts} >> >> >>");
    let mut all = vec![
        "<< /Type /Catalog /Pages 2 0 R >>",
        "<< /Type /Pages /Kids [3 0 R] /Count 1 >>",
        &page,
    ];
    all.extend_from_slice(objects);
    let doc = Document::open(classic_with_root(&all, ""), OpenOptions::default()).unwrap();
    doc.pages().unwrap().get(0).unwrap().fonts().unwrap()
}

/// A Type 3 font's dictionary with `extra` entries.
fn type3(extra: &str) -> String {
    format!(
        "<< /Type /Font /Subtype /Type3 /FontBBox [0 0 1 1] /FontMatrix [1 0 0 1 0 0] \
         /CharProcs << >> /Encoding << /Differences [] >> {extra} >>"
    )
}

#[test]
fn an_indirect_base_font_should_name_the_font() {
    let fonts = fonts_of(
        "/F1 4 0 R",
        &[
            "<< /Type /Font /Subtype /Type1 /BaseFont 5 0 R >>",
            "/ABCDEF+Times-Roman",
        ],
    );
    let font = fonts.first().unwrap();
    assert_eq!((font.name.as_deref(), font.subset), (Some("Times-Roman"), true));
}

#[test]
fn a_type3_font_should_take_its_descriptors_font_name() {
    let font = type3("/FontDescriptor << /Type /FontDescriptor /FontName /AAAAAA+Toy#20Sans /Flags 4 >> /Name /F9");
    let fonts = fonts_of(&format!("/F1 {font}"), &[]);
    let font = fonts.first().unwrap();
    assert_eq!((font.name.as_deref(), font.subset), (Some("Toy Sans"), true));
}

#[test]
fn a_type3_font_without_a_descriptor_should_take_its_name_entry() {
    let fonts = fonts_of(&format!("/F1 {}", type3("/Name /T5")), &[]);
    assert_eq!(fonts.first().unwrap().name.as_deref(), Some("T5"));
}

#[test]
fn a_simple_font_should_not_take_its_name_entry() {
    let fonts = fonts_of("/F1 << /Type /Font /Subtype /Type1 /Name /F1 >>", &[]);
    assert_eq!(fonts.first().unwrap().name, None);
}

#[test]
fn a_tounicode_name_should_not_count_as_a_cmap() {
    let fonts = fonts_of(
        "/F1 << /Type /Font /Subtype /Type1 /BaseFont /Times-Roman /ToUnicode /Identity-H >>",
        &[],
    );
    assert!(!fonts.first().unwrap().to_unicode);
}

#[test]
fn a_tounicode_stream_should_count_as_a_cmap() {
    let cmap = String::from_utf8(stream_body("", b"1 beginbfchar <41> <0041> endbfchar")).unwrap();
    let fonts = fonts_of(
        "/F1 << /Type /Font /Subtype /Type1 /BaseFont /Times-Roman /ToUnicode 4 0 R >>",
        &[&cmap],
    );
    assert!(fonts.first().unwrap().to_unicode);
}

#[test]
fn an_encoding_dictionary_without_differences_should_be_its_base_encoding() {
    let fonts = fonts_of(
        "/F1 << /Type /Font /Subtype /Type1 /BaseFont /Times-Roman /Encoding 4 0 R >>",
        &["<< /Type /Encoding /BaseEncoding /WinAnsiEncoding >>"],
    );
    assert_eq!(
        fonts.first().unwrap().encoding,
        Some(FontEncoding::Named("WinAnsiEncoding".to_owned()))
    );
}

#[test]
fn an_encoding_dictionary_with_differences_should_be_custom() {
    let fonts = fonts_of(
        "/F1 << /Type /Font /Subtype /Type1 /BaseFont /Times-Roman \
         /Encoding << /BaseEncoding /WinAnsiEncoding /Differences [65 /B] >> >>",
        &[],
    );
    assert_eq!(fonts.first().unwrap().encoding, Some(FontEncoding::Custom));
}

#[test]
fn a_simple_fonts_encoding_stream_should_declare_nothing() {
    let stream = String::from_utf8(stream_body("", b"junk")).unwrap();
    let fonts = fonts_of(
        "/F1 << /Type /Font /Subtype /TrueType /BaseFont /Arial /Encoding 4 0 R >>",
        &[&stream],
    );
    assert_eq!(fonts.first().unwrap().encoding, None);
}

#[test]
fn a_type0_fonts_cmap_stream_should_be_custom() {
    let cmap = String::from_utf8(stream_body("/Type /CMap /CMapName /Custom", b"")).unwrap();
    let fonts = fonts_of(
        "/F1 << /Type /Font /Subtype /Type0 /BaseFont /Noto /Encoding 4 0 R \
         /DescendantFonts [<< /Type /Font /Subtype /CIDFontType2 /BaseFont /Noto >>] >>",
        &[&cmap],
    );
    assert_eq!(fonts.first().unwrap().encoding, Some(FontEncoding::Custom));
}
