//! A content IR recorded through `TextExtractor::page_with` (ADR 0022 §1): the extractor
//! measures the glyphs, so text objects get boxes as wide as their text.
#![allow(clippy::expect_used, reason = "tests: a panic is a test failure")]

use laminapdf_content::{ContentObject, ContentObjectKind, ContentOps, ContentOwner};
use laminapdf_cos::OpenOptions;
use laminapdf_model::PointPt;
use laminapdf_text::TextExtractor;

mod common;

use common::extract::{helvetica, page};

#[test]
fn every_glyph_should_hit_the_text_object_that_shows_it() {
    let content = b"BT /F1 10 Tf 72 700 Td (Hello) Tj ET 0 0 10 10 re f BT /F1 14 Tf 300 400 Td (World) Tj ET";
    let doc = page(content, vec![helvetica()], OpenOptions::default());
    let pages = doc.pages().expect("pages");
    let page = pages.get(0).expect("a page");
    let owner = ContentOwner::Page(page.id().clone());
    let mut ops = ContentOps::read_page(&doc, page.cos(), owner).expect("an IR");
    let frame = page.frame().expect("a frame");
    let text = TextExtractor::new()
        .page_with(&doc, &page, &mut ops.recorder(frame))
        .expect("extracts");
    let texts: Vec<_> = ops
        .objects()
        .iter()
        .filter(|o| o.kind() == ContentObjectKind::Text)
        .map(ContentObject::id)
        .collect();
    let hit: Vec<_> = text
        .glyphs
        .iter()
        .map(|g| {
            let centre = |i: fn(&PointPt) -> f64| g.quad.iter().map(i).sum::<f64>() / 4.0;
            let point = PointPt::from_points(centre(|p| p.x.get()), centre(|p| p.y.get()));
            ops.hit_test(point).into_iter().find(|id| texts.contains(id))
        })
        .collect();
    let expected: Vec<_> = [texts[0]; 5].into_iter().chain([texts[1]; 5]).map(Some).collect();
    assert_eq!(hit, expected);
}
