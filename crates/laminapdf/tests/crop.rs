//! Cropping only hides: text outside a page's new crop box stays in the file, so extraction
//! still reads it, and the edit says which pages that is true of.
#![allow(clippy::unwrap_used, reason = "tests: a panic is a test failure")]

use laminapdf::extract::TextOptions;
use laminapdf::{CropPages, Document, Margins, OpenOptions, SaveOptions};
use laminapdf_test_fixtures::fixture;

fn text(doc: &Document) -> Vec<String> {
    doc.page_texts(&TextOptions::default())
        .unwrap()
        .iter()
        .map(|page| page.lines().join("\n"))
        .collect()
}

#[test]
fn text_a_crop_hides_is_still_extracted_and_the_edit_says_so() {
    let mut doc = Document::open_bytes(fixture("pages/boxes.pdf"), OpenOptions::new()).unwrap();
    let before = text(&doc);
    let pages: Vec<_> = doc.page_infos().unwrap().into_iter().map(|p| p.id).collect();
    // A narrow strip in the middle of each page: "Page n", near the top, is cut off.
    let edited = doc
        .apply(vec![
            CropPages::new(pages.clone(), Margins::new(250.0, 100.0, 250.0, 100.0)).into(),
        ])
        .unwrap();
    let saved = doc.save(&SaveOptions::new()).unwrap().bytes;
    let reopened = Document::open_bytes(saved, OpenOptions::new()).unwrap();
    assert_eq!(
        (
            edited.cropped_content_remains,
            text(&reopened),
            before.first().map(String::as_str)
        ),
        (pages, before.clone(), Some("Page 1"))
    );
}
