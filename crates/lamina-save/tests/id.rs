//! `/ID` (§14.4, ADR 0007 §5): `/ID[0]` is kept by every save, `/ID[1]` comes from a hash of
//! the content written, and both are derived when the document has none.
#![allow(
    clippy::unwrap_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "tests: a panic is a test failure"
)]

mod common;

use common::save::{ID0, corpus_doc, editable, ids, reopen, set_title, two_pages};
use lamina_save::{Renumber, Save, SaveOptions, XrefKind};

#[derive(Clone, Copy, Debug)]
enum How {
    Incremental,
    Full(SaveOptions),
}

const HOWS: [How; 4] = [
    How::Incremental,
    How::Full(SaveOptions::new()),
    How::Full(SaveOptions::new().with_xref(XrefKind::Classic)),
    How::Full(SaveOptions::new().with_renumber(Renumber::Compact)),
];

fn saved(mut doc: lamina_ops::Document, how: How, title: &str) -> Vec<u8> {
    set_title(&mut doc, title);
    let mut out = Vec::new();
    match how {
        How::Incremental => doc.save_incremental(&mut out, &SaveOptions::new()).map(|_| ()),
        How::Full(options) => doc.save_full(&mut out, &options).map(|_| ()),
    }
    .unwrap();
    out
}

fn id0() -> Vec<u8> {
    (0..16u8).map(|i| i * 0x11).collect()
}

#[test]
fn every_save_keeps_id0() {
    assert_eq!(ID0.len(), 32);
    for how in HOWS {
        let out = saved(editable(&two_pages()), how, "T");
        assert_eq!(ids(&reopen(&out)).0, id0(), "{how:?}");
    }
}

#[test]
fn id1_changes_with_the_content() {
    for how in HOWS {
        let a = ids(&reopen(&saved(editable(&two_pages()), how, "A"))).1;
        let b = ids(&reopen(&saved(editable(&two_pages()), how, "B"))).1;
        assert!(a != b && a != id0(), "{how:?}");
    }
}

#[test]
fn a_document_without_id_gets_both_from_the_hash() {
    for how in HOWS {
        let (first, second) = ids(&reopen(&saved(corpus_doc("hello.pdf"), how, "T")));
        assert_eq!((first.len(), first == second), (16, true), "{how:?}");
    }
}

#[test]
fn the_id_is_deterministic() {
    for how in HOWS {
        let run = || ids(&reopen(&saved(corpus_doc("hello.pdf"), how, "T")));
        assert_eq!(run(), run(), "{how:?}");
    }
}

#[test]
fn a_second_save_keeps_the_id0_the_first_derived() {
    let mut doc = corpus_doc("hello.pdf");
    set_title(&mut doc, "One");
    doc.save_incremental(&mut Vec::new(), &SaveOptions::new()).unwrap();
    let derived = ids(&doc).0;
    set_title(&mut doc, "Two");
    let mut out = Vec::new();
    doc.save_full(&mut out, &SaveOptions::new()).unwrap();
    let (first, second) = ids(&reopen(&out));
    assert_eq!((first == derived, second == derived), (true, false));
}
