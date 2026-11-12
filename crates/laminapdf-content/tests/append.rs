//! The append API (todo E2e) over `fixtures/content/append/` (`tools/fixtures/content_append.py`):
//! what a wrap writes, which `Q` tokens it blanks, the resource names it picks, and what the
//! interpreter sees before and after (the same paths, in the same place and colour).
#![allow(
    clippy::unwrap_used,
    clippy::indexing_slicing,
    clippy::panic,
    clippy::cast_possible_truncation,
    reason = "tests: a panic is a failure; coordinates are rounded to hundredths of a point"
)]

mod common;

use common::append_fuzz_entry::content_append_scan;
use laminapdf_content::{
    Balance, PageContentWriter, PathPaint, StreamOffset, Visitor, Wrapped, interpret_page, scan_balance,
};
use laminapdf_cos::{Dict, ObjRef, Object};
use laminapdf_model::{Document, OpenOptions};
use laminapdf_save::{FullWriter, SaveOptions};
use laminapdf_test_fixtures::fixture;

const FIXTURES: [&str; 9] = [
    "balanced",
    "open-q",
    "extra-Q",
    "stray-Q",
    "split",
    "inline-image",
    "shared",
    "in-text",
    "unterminated",
];

fn open(name: &str) -> Document {
    Document::open(fixture(&format!("content/append/{name}.pdf")), OpenOptions::default()).unwrap()
}

fn writer(doc: &mut Document, index: usize) -> PageContentWriter<'_> {
    let page = doc.pages().unwrap().get(index).unwrap().cos().clone();
    PageContentWriter::new(doc.cos_mut().edit(), &page).unwrap()
}

fn wrap(doc: &mut Document, index: usize) -> laminapdf_content::Result<Wrapped> {
    writer(doc, index).wrap_existing()
}

/// The page's `/Contents` entries as stored.
fn contents(doc: &Document, index: usize) -> Vec<Object> {
    let pages = doc.pages().unwrap();
    let page = pages.get(index).unwrap();
    match &*doc.deref(page.cos().dict.get("Contents").unwrap()).unwrap() {
        Object::Array(items) => items.clone(),
        _ => vec![page.cos().dict.get("Contents").unwrap().clone()],
    }
}

fn decoded(doc: &Document, entry: &Object) -> Vec<u8> {
    match &*doc.deref(entry).unwrap() {
        Object::Stream(s) => doc.decode_stream(s).unwrap(),
        other => panic!("not a stream: {other:?}"),
    }
}

fn streams(doc: &Document, index: usize) -> Vec<Vec<u8>> {
    contents(doc, index).iter().map(|e| decoded(doc, e)).collect()
}

fn balance(doc: &Document, index: usize) -> Balance {
    let streams = streams(doc, index);
    let views: Vec<&[u8]> = streams.iter().map(Vec::as_slice).collect();
    scan_balance(&views, &laminapdf_content::Limits::default()).unwrap()
}

fn now(closing: usize, blanked: &[(usize, usize)], ended_text: bool) -> Wrapped {
    Wrapped::Now {
        closing,
        blanked: blanked.iter().map(|&(s, o)| StreamOffset::new(s, o)).collect(),
        ended_text,
    }
}

// What a wrap writes.

#[test]
fn a_balanced_page_keeps_its_stream_and_gets_one_q_and_one_closing_q() {
    let mut doc = open("balanced");
    let before = contents(&doc, 0);
    assert_eq!(wrap(&mut doc, 0).unwrap(), now(1, &[], false));
    let after = contents(&doc, 0);
    assert_eq!((after.len(), &after[1]), (3, &before[0]));
    assert_eq!(
        (decoded(&doc, &after[0]), decoded(&doc, &after[2])),
        (b"q\n".to_vec(), b"\nQ\n".to_vec())
    );
}

#[test]
fn open_qs_are_closed_by_one_q_each_plus_the_wraps_own() {
    let mut doc = open("open-q");
    let e = balance(&doc, 0).depth;
    assert_eq!(e, 2);
    assert_eq!(wrap(&mut doc, 0).unwrap(), now(e + 1, &[], false));
    assert_eq!(decoded(&doc, contents(&doc, 0).last().unwrap()), b"\nQ\nQ\nQ\n");
}

#[test]
fn streams_without_an_unmatched_q_are_byte_identical_after_wrapping() {
    for name in ["balanced", "open-q", "in-text"] {
        let mut doc = open(name);
        let before = streams(&doc, 0);
        wrap(&mut doc, 0).unwrap();
        assert_eq!(streams(&doc, 0)[1..2], before[..], "{name}");
    }
}

#[test]
fn the_unmatched_q_after_a_state_change_is_blanked_and_nothing_else() {
    let mut doc = open("stray-Q");
    let before = streams(&doc, 0);
    let old = contents(&doc, 0)[0].clone();
    assert_eq!(wrap(&mut doc, 0).unwrap(), now(1, &[(0, 15)], false));
    let mut expected = before[0].clone();
    expected[15] = b' ';
    assert_eq!(streams(&doc, 0)[1], expected);
    assert_ne!(contents(&doc, 0)[1], old, "the copy is a new object");
    assert_eq!(decoded(&doc, &old), before[0], "the original stream is unchanged");
}

#[test]
fn only_the_second_of_two_qs_is_blanked() {
    let mut doc = open("extra-Q");
    let content = &streams(&doc, 0)[0];
    let second = content.windows(3).position(|w| w == b"Q Q").unwrap() + 2;
    assert_eq!(wrap(&mut doc, 0).unwrap(), now(1, &[(0, second)], false));
}

#[test]
fn unmatched_qs_in_two_streams_are_blanked_in_each_and_the_depth_carries_across() {
    let mut doc = open("split");
    // Stream 0 leaves one q open, closed by stream 1's first Q; its second Q is unmatched.
    assert_eq!(wrap(&mut doc, 0).unwrap(), now(2, &[(0, 15), (1, 2)], false));
    assert_eq!(contents(&doc, 0).len(), 4);
    assert!(balance(&doc, 0).wrapped);
}

#[test]
fn q_bytes_in_an_inline_image_are_left_alone() {
    let mut doc = open("inline-image");
    let content = streams(&doc, 0)[0].clone();
    let stray = content.windows(6).position(|w| w == b"EI Q Q").unwrap() + 5;
    assert_eq!(wrap(&mut doc, 0).unwrap(), now(1, &[(0, stray)], false));
    let after = &streams(&doc, 0)[1];
    let data = content.windows(5).position(|w| w == b"ID QQ").unwrap() + 3;
    assert_eq!(&after[data..data + 2], b"QQ");
}

#[test]
fn content_ending_in_a_text_object_is_closed_with_et_first() {
    let mut doc = open("in-text");
    assert_eq!(wrap(&mut doc, 0).unwrap(), now(1, &[], true));
    assert_eq!(decoded(&doc, contents(&doc, 0).last().unwrap()), b"\nET\nQ\n");
}

#[test]
fn content_ending_inside_a_string_is_refused_and_nothing_is_written() {
    let mut doc = open("unterminated");
    assert_eq!(wrap(&mut doc, 0).unwrap_err().code(), "invalid-structure");
    assert!(!doc.cos().is_dirty());
}

#[test]
fn a_wrapped_page_is_not_wrapped_again_even_after_appending() {
    let mut doc = open("split");
    wrap(&mut doc, 0).unwrap();
    let mut w = writer(&mut doc, 0);
    w.append(b"q 1 0 0 rg 0 0 5 5 re f Q").unwrap();
    assert_eq!(w.wrap_existing().unwrap(), Wrapped::Already);
    assert_eq!(wrap(&mut doc, 0).unwrap(), Wrapped::Already);
}

#[test]
fn added_content_that_leaks_state_is_refused() {
    let mut doc = open("balanced");
    let mut w = writer(&mut doc, 0);
    for bad in [&b"q 1 0 0 rg"[..], b"Q", b"BT /F1 12 Tf"] {
        assert_eq!(w.append(bad).unwrap_err().code(), "invalid-structure");
    }
}

// Resources.

#[test]
fn new_names_avoid_every_inherited_name_and_the_shared_dictionaries_stay_as_they_were() {
    let mut doc = open("shared");
    let tree = doc.cos().resolve(ObjRef::new(2, 0)).unwrap();
    let form = ObjRef::new(5, 0);
    let mut w = writer(&mut doc, 0);
    w.wrap_existing().unwrap();
    let new_form = w
        .cos()
        .create(Object::Stream(laminapdf_cos::Stream::new(
            form_dict(),
            b"0 0 5 5 re f".to_vec(),
        )))
        .unwrap();
    assert_eq!(w.add_xobject(new_form).unwrap().as_bytes(), b"Fm2");
    assert_eq!(
        w.add_xobject(form).unwrap().as_bytes(),
        b"Fm1",
        "a form already there keeps its name"
    );
    assert_eq!(w.add_extgstate(Object::Dict(Dict::new())).unwrap().as_bytes(), b"Gs2");
    assert_eq!(w.add_font(ObjRef::new(6, 0)).unwrap().as_bytes(), b"Ft1");
    drop(w);
    assert_eq!(
        doc.cos().resolve(ObjRef::new(2, 0)).unwrap(),
        tree,
        "the page tree's resources"
    );
    let pages = doc.pages().unwrap();
    let second = pages.get(1).unwrap();
    assert_eq!(
        second.cos().dict.get("Contents"),
        Some(&Object::Ref(ObjRef::new(10, 0)))
    );
    assert!(second.cos().dict.get("Resources").is_none());
}

#[test]
fn a_shared_stream_is_copied_for_the_wrapped_page_and_kept_for_the_other() {
    let mut doc = open("shared");
    let shared = decoded(&doc, &Object::Ref(ObjRef::new(10, 0)));
    wrap(&mut doc, 0).unwrap();
    assert_eq!(decoded(&doc, &Object::Ref(ObjRef::new(10, 0))), shared);
    assert_eq!(streams(&doc, 1), [shared]);
}

fn form_dict() -> Dict {
    let mut d = Dict::new();
    d.insert("Type", Object::name("XObject"));
    d.insert("Subtype", Object::name("Form"));
    d.insert("BBox", vec![0.into(), 0.into(), 5.into(), 5.into()]);
    d
}

// The journal.

#[test]
fn writes_rolled_back_leave_the_overlay_empty_and_the_next_save_unchanged() {
    for name in FIXTURES {
        let mut doc = open(name);
        let saved = full_save(&doc);
        for page in 0..doc.pages().unwrap().len() {
            let mark = doc.cos().journal_mark();
            let mut w = writer(&mut doc, page);
            let _ = w.wrap_existing();
            let _ = w.add_extgstate(Object::Dict(Dict::new()));
            let _ = w.append(b"q 1 0 0 rg 0 0 10 10 re f Q");
            let _ = w.prepend(b"");
            drop(w);
            doc.cos_mut().rollback_to(mark).unwrap();
            assert!(doc.cos().overlay().is_empty(), "{name} page {page}");
        }
        assert_eq!(full_save(&doc), saved, "{name}");
    }
}

fn full_save(doc: &Document) -> Vec<u8> {
    let mut out = Vec::new();
    FullWriter::new(doc.cos(), SaveOptions::default())
        .write(&mut out)
        .unwrap();
    out
}

// What the interpreter sees.

/// Every filled path: its box in default user space and its fill colour.
#[derive(Default)]
struct Fills(Vec<([i64; 4], Option<[i64; 3]>)>);

impl Visitor for Fills {
    fn paint_path(&mut self, paint: &PathPaint<'_>) {
        let round = |v: f64| (v * 100.0).round() as i64;
        let b = paint
            .bounds
            .map_or([0; 4], |r| [round(r.x0), round(r.y0), round(r.x1), round(r.y1)]);
        let colour = paint.state.fill.approx_rgb().map(|c| c.map(round));
        self.0.push((b, colour));
    }
}

fn fills(doc: &Document, index: usize) -> Vec<([i64; 4], Option<[i64; 3]>)> {
    let pages = doc.pages().unwrap();
    let mut fills = Fills::default();
    interpret_page(doc, &pages.get(index).unwrap(), &mut fills).unwrap();
    fills.0
}

#[test]
fn every_page_paints_the_same_paths_after_wrapping() {
    for name in FIXTURES.iter().filter(|&&n| n != "unterminated") {
        let mut doc = open(name);
        for page in 0..doc.pages().unwrap().len() {
            let before = fills(&doc, page);
            wrap(&mut doc, page).unwrap();
            assert_eq!(fills(&doc, page), before, "{name} page {page}");
        }
    }
}

#[test]
fn a_rectangle_appended_after_a_stray_q_lands_where_it_does_on_a_balanced_page() {
    let appended = |name: &str| {
        let mut doc = open(name);
        let mut w = writer(&mut doc, 0);
        w.wrap_existing().unwrap();
        w.append(b"q 1 0 0 rg 150 150 20 20 re f Q").unwrap();
        drop(w);
        *fills(&doc, 0).last().unwrap()
    };
    let expected = appended("balanced");
    assert_eq!(expected, ([15000, 15000, 17000, 17000], Some([100, 0, 0])));
    for name in ["open-q", "extra-Q", "stray-Q", "split", "inline-image", "in-text"] {
        assert_eq!(appended(name), expected, "{name}");
    }
}

// The fuzz target's entry.

#[test]
fn every_saved_fuzz_input_replays_without_a_panic() {
    let Ok(files) = std::fs::read_dir(laminapdf_test_fixtures::fixtures_dir().join("regressions/content_append_scan"))
    else {
        return;
    };
    for file in files {
        content_append_scan(&std::fs::read(file.unwrap().path()).unwrap());
    }
}

#[test]
fn the_fuzz_entry_holds_on_the_fixtures_content() {
    for input in [
        &b"q 2 0 0 2 0 0 cm Q Q 0 0 1 rg\x002 0 0 2 0 0 cm Q 0 0 10 10 re f"[..],
        b"q\x00q 1 0 0 rg\x00\nQ\nQ\n",
        b"BI /W 1 /H 1 /BPC 8 /CS /G ID Q EI Q",
        b"BT (unterminated",
        b"Q\x00Q\x00Q",
        b"",
    ] {
        content_append_scan(input);
    }
}
