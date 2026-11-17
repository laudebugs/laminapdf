//! `Page::fonts` finds the fonts a page draws with through a tiling pattern and a soft mask's
//! group, not only through its own resources and forms.
#![allow(clippy::unwrap_used, reason = "tests: a panic is a test failure")]

use laminapdf_cos::OpenOptions;
use laminapdf_model::Document;
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
