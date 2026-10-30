//! Outlines (todo B5b): the golden's outline read back through `lamina-model`'s view (B2c):
//! titles, nesting, open state, colour/style flags, `/Count`, destinations to pages, and the
//! IDs `Builder::outline` hands out.
#![allow(
    clippy::unwrap_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "tests: a panic is a test failure"
)]

mod common;

use common::{fixture, reopen, save};
use lamina_build::{BuildOptions, Builder, LinkTarget, OutlineEntry, PageSize};

#[test]
fn the_golden_outline_reads_back_as_built() {
    let doc = reopen(&fixture("golden/links-and-outlines.pdf"));
    let outline = doc.outline().unwrap();
    let items: Vec<(String, usize, bool, Option<usize>)> = outline
        .iter()
        .map(|i| {
            (
                i.title().unwrap(),
                i.depth(),
                i.is_open().unwrap(),
                i.target().unwrap().and_then(|d| d.page_index),
            )
        })
        .collect();
    let s = |t: &str| t.to_owned();
    assert_eq!(
        items,
        [
            (s("Title page"), 1, false, Some(0)),
            (s("Contents"), 1, false, Some(1)),
            (s("Chapters"), 1, true, None),
            (s("Chapter 1"), 2, false, Some(2)),
            (s("Chapter 2"), 2, false, Some(3)),
            (s("Appendix"), 3, false, Some(3)),
            (s("Website"), 1, false, None),
        ]
    );
    assert_eq!(doc.warnings(), []);
}

#[test]
fn counts_flags_and_colours_are_written() {
    let doc = reopen(&fixture("golden/links-and-outlines.pdf"));
    let catalog = doc.catalog().unwrap();
    let root = doc.resolve(catalog.get("Outlines").unwrap().as_ref().unwrap()).unwrap();
    // Four top-level items plus the open "Chapters"' two children.
    assert_eq!(root.as_dict().unwrap().get_int("Count"), Some(6));
    let outline = doc.outline().unwrap();
    let chapters = outline.iter().nth(2).unwrap();
    let chapter_2 = outline.iter().nth(4).unwrap();
    assert_eq!(
        (chapters.count().unwrap(), chapter_2.count().unwrap()),
        (Some(2), Some(-1))
    );
    let dict = |i: usize| {
        let id = outline.iter().nth(i).unwrap().id().unwrap().clone();
        let r = id.as_lamina_id().obj_ref().unwrap();
        doc.resolve(r).unwrap().as_dict().unwrap().clone()
    };
    assert_eq!((dict(2).get_int("F"), dict(5).get_int("F")), (Some(2), Some(1)));
    assert!(dict(1).contains_key("C"));
}

#[test]
fn outline_ids_are_the_items_ids() {
    let mut b = Builder::new(BuildOptions::default());
    b.page(PageSize::A4).finish().unwrap();
    let ids = b
        .outline([OutlineEntry::new("A", LinkTarget::page(0)).with_child(OutlineEntry::heading("B"))])
        .unwrap();
    let more = b.outline([OutlineEntry::heading("C")]).unwrap();
    let doc = reopen(&save(b.finish().unwrap()));
    let outline = doc.outline().unwrap();
    let read: Vec<_> = outline.iter().map(|i| i.id().unwrap().clone()).collect();
    assert_eq!(read, [ids, more].concat());
}
