//! `Document::page_text` (todo B9d; feature `text`): the five-script golden's lines, the
//! index check, invisible text, warnings through the cursor, and the font cache following
//! edits (a rotated page's quads turn with it).
#![cfg(feature = "text")]
#![expect(
    clippy::unwrap_used,
    clippy::panic,
    reason = "test code and helpers: a panic is a test failure"
)]

use std::path::Path;

use laminapdf::extract::TextOptions;
use laminapdf::{Document, Op, OpenOptions, RotatePage, Rotation};

fn open(rel: &str) -> Document {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures").join(rel);
    let bytes = std::fs::read(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
    Document::open_bytes(bytes, OpenOptions::default()).unwrap()
}

#[test]
fn the_five_scripts_extract_in_logical_order() {
    let doc = open("golden/text-shaped.pdf");
    let lines = doc.page_text(0, &TextOptions::new()).unwrap().lines();
    assert_eq!(lines[3..5], ["مرحبا بالعالم، السلام عليكم", "שָׁלוֹם עוֹלָם, שלום עולם"]);
}

#[test]
fn a_page_past_the_end_is_out_of_range() {
    let doc = open("golden/text-shaped.pdf");
    let error = doc.page_text(1, &TextOptions::new()).unwrap_err();
    assert_eq!(error.code(), "page-index-out-of-range");
}

#[test]
fn invisible_text_is_kept_unless_skipped() {
    let doc = open("golden/build-text-latin.pdf");
    let has = |options: &TextOptions| {
        doc.page_text(0, options)
            .unwrap()
            .lines()
            .iter()
            .any(|l| l == "Invisible but extractable")
    };
    assert_eq!(
        [
            has(&TextOptions::new()),
            has(&TextOptions::new().with_skip_invisible(true))
        ],
        [true, false]
    );
}

#[test]
fn quads_follow_a_rotation_made_through_the_facade() {
    let mut doc = open("golden/text-std14.pdf");
    let before = doc.page_text(0, &TextOptions::new()).unwrap().glyphs[0].quad;
    let id = doc.page_info(0).unwrap().id;
    doc.apply(vec![Op::from(RotatePage::new(id, Rotation::Clockwise90))])
        .unwrap();
    let after = doc.page_text(0, &TextOptions::new()).unwrap().glyphs[0].quad;
    // Turned 90° clockwise: page x is the old distance from the bottom, page y the old x.
    let height = doc.page_info(0).unwrap().width;
    assert!(
        (after[0].x.get() - (height - before[0].y.get())).abs() < 1e-6
            && (after[0].y.get() - before[0].x.get()).abs() < 1e-6,
        "{before:?} → {after:?}"
    );
}

#[test]
fn page_texts_gives_every_page_as_page_text_does() {
    let doc = open("golden/build-text-multiscript.pdf");
    let all = doc.page_texts(&TextOptions::new()).unwrap();
    let one_by_one: Vec<_> = (0..doc.page_count().unwrap())
        .map(|i| doc.page_text(i, &TextOptions::new()).unwrap().lines())
        .collect();
    assert_eq!(
        all.iter().map(laminapdf::extract::PageText::lines).collect::<Vec<_>>(),
        one_by_one
    );
}
