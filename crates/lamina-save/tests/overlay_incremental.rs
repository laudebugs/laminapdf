//! The incremental writer over the overlay (ADR 0007 §5, §7.5.6): what the base keeps, what
//! is appended, and how the appended section matches the base's.
#![allow(
    clippy::unwrap_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "tests: a panic is a test failure"
)]

mod common;

use std::sync::Arc;

use common::fixture;
use common::save::{ID0, Windowed, corpus_doc, editable, facts, ids, reopen, rotate, set_title, two_pages};
use lamina_cos::{ByteSource, Document, ObjRef, OpenOptions};
use lamina_model::Rotation;
use lamina_save::{IncrementalOutput, IncrementalWriter, Save, SaveMode, SaveOptions};

fn save(doc: &mut lamina_ops::Document, options: SaveOptions) -> Vec<u8> {
    let mut out = Vec::new();
    doc.save_incremental(&mut out, &options).unwrap();
    out
}

#[test]
fn an_empty_overlay_writes_exactly_the_base_bytes() {
    let base = fixture("corpus/hello.pdf");
    assert_eq!(save(&mut editable(&base), SaveOptions::new()), base);
}

#[test]
fn an_empty_overlay_with_appended_only_writes_nothing() {
    let out = save(
        &mut editable(&fixture("corpus/hello.pdf")),
        SaveOptions::new().with_output(IncrementalOutput::AppendedOnly),
    );
    assert_eq!(out, Vec::<u8>::new());
}

#[test]
fn set_info_round_trips_with_one_more_revision_and_the_base_as_prefix() {
    let base = two_pages();
    let mut doc = editable(&base);
    set_title(&mut doc, "Updated");
    let out = save(&mut doc, SaveOptions::new());
    let reopened = reopen(&out);
    assert_eq!(
        (
            facts(&reopened).title.as_deref(),
            reopened.revisions(),
            out.starts_with(&base),
            ids(&reopened).0,
        ),
        (Some("Updated"), 2, true, hex(ID0))
    );
}

#[test]
fn set_info_on_a_document_without_info_creates_it() {
    let mut doc = corpus_doc("hello.pdf");
    set_title(&mut doc, "New");
    let out = save(&mut doc, SaveOptions::new());
    assert_eq!(facts(&reopen(&out)).title.as_deref(), Some("New"));
}

#[test]
fn the_report_gives_the_append_offset_and_byte_count() {
    let base = two_pages();
    let mut doc = editable(&base);
    set_title(&mut doc, "Updated");
    let mut out = Vec::new();
    let report = doc.save_incremental(&mut out, &SaveOptions::new()).unwrap();
    assert_eq!(
        (
            report.mode(),
            report.append_offset(),
            report.bytes_written(),
            report.renumbered()
        ),
        (SaveMode::Incremental, Some(base.len() as u64), out.len() as u64, None)
    );
}

#[test]
fn appended_only_is_the_whole_output_without_the_base() {
    let base = two_pages();
    let mut whole = editable(&base);
    let mut appended = editable(&base);
    for doc in [&mut whole, &mut appended] {
        rotate(doc, 0, Rotation::Half);
    }
    let whole = save(&mut whole, SaveOptions::new());
    let appended = save(
        &mut appended,
        SaveOptions::new().with_output(IncrementalOutput::AppendedOnly),
    );
    assert_eq!(&whole[base.len()..], &appended[..]);
}

#[test]
fn a_classic_base_gets_a_classic_section_with_prev() {
    let mut doc = editable(&two_pages());
    rotate(&mut doc, 1, Rotation::None);
    let out = save(&mut doc, SaveOptions::new());
    let reopened = reopen(&out);
    assert_eq!(
        (reopened.has_xref_stream(), reopened.trailer().get_int("Prev").is_some()),
        (false, true)
    );
}

#[test]
fn an_xref_stream_base_gets_an_xref_stream_section() {
    let mut doc = corpus_doc("hello-xrefstm.pdf");
    set_title(&mut doc, "Streamed");
    let out = save(&mut doc, SaveOptions::new());
    let reopened = reopen(&out);
    assert_eq!(
        (reopened.newest_section_is_stream(), facts(&reopened).title.as_deref()),
        (true, Some("Streamed"))
    );
}

#[test]
fn the_trailer_drops_xref_stream_keys() {
    let mut doc = corpus_doc("hello-xrefstm.pdf");
    set_title(&mut doc, "Streamed");
    let out = save(&mut doc, SaveOptions::new());
    let reopened = reopen(&out);
    // The newest trailer is the appended xref stream's dictionary: its own /W, no copies.
    let trailer = reopened.trailer();
    assert_eq!(
        (trailer.get_name("Type"), trailer.contains_key("DecodeParms")),
        (Some(&b"XRef"[..]), false)
    );
}

#[test]
fn a_deleted_object_becomes_a_free_entry_with_the_next_generation() {
    let base = two_pages();
    let mut cos = Document::open(base, OpenOptions::default()).unwrap();
    cos.edit().delete(ObjRef::new(6, 0)).unwrap();
    let mut out = Vec::new();
    IncrementalWriter::new(&cos, SaveOptions::new())
        .write(&mut out)
        .unwrap();
    let reopened = reopen(&out);
    let text = String::from_utf8_lossy(&out);
    assert_eq!(
        (reopened.base_generation(6), text.contains("00001 f\r\n")),
        (None, true)
    );
}

#[test]
fn size_never_shrinks_and_covers_reserved_numbers() {
    let mut cos = Document::open(two_pages(), OpenOptions::default()).unwrap();
    let r = cos.edit().create(lamina_cos::Object::Integer(1)).unwrap();
    cos.edit().delete(r).unwrap(); // number 7 stays reserved, unwritten
    cos.edit()
        .set(ObjRef::new(6, 0), lamina_cos::Object::Integer(2))
        .unwrap();
    let mut out = Vec::new();
    IncrementalWriter::new(&cos, SaveOptions::new())
        .write(&mut out)
        .unwrap();
    assert_eq!(reopen(&out).trailer().get_int("Size"), Some(8));
}

#[test]
fn new_objects_are_written_with_generation_zero_after_the_base() {
    let mut doc = corpus_doc("hello.pdf");
    set_title(&mut doc, "New");
    let out = save(&mut doc, SaveOptions::new());
    let reopened = reopen(&out);
    let info = reopened
        .trailer()
        .get("Info")
        .and_then(lamina_cos::Object::as_ref)
        .unwrap();
    assert_eq!((info.gen_nr, info.num >= 6), (0, true));
}

#[test]
fn a_windowed_base_gives_the_same_bytes_as_in_memory() {
    let base = two_pages();
    let windowed = Windowed::new(base.clone());
    let mut a =
        lamina_ops::Document::open(Arc::clone(&windowed) as Arc<dyn ByteSource>, OpenOptions::default()).unwrap();
    let mut b = editable(&base);
    set_title(&mut a, "Same");
    set_title(&mut b, "Same");
    assert_eq!(save(&mut a, SaveOptions::new()), save(&mut b, SaveOptions::new()));
}

#[test]
fn a_windowed_base_rebases_onto_base_and_revision() {
    let windowed = Windowed::new(two_pages());
    let mut doc = lamina_ops::Document::open(windowed as Arc<dyn ByteSource>, OpenOptions::default()).unwrap();
    set_title(&mut doc, "Rebased");
    save(&mut doc, SaveOptions::new());
    assert_eq!(
        (doc.data(), facts(&doc).title.as_deref(), doc.revisions()),
        (None, Some("Rebased"), 2)
    );
}

#[test]
fn a_pending_base_fails_before_writing_and_works_once_fetched() {
    let source = Windowed::new(two_pages());
    let mut doc =
        lamina_ops::Document::open(Arc::clone(&source) as Arc<dyn ByteSource>, OpenOptions::default()).unwrap();
    set_title(&mut doc, "Later");
    // The bytes are gone again (an evicted range cache): the save must not start writing.
    let pending = Arc::new(Windowed {
        data: source.data.clone(),
        missing_from: 100,
        fetched: true.into(),
    });
    let mut cos = Document::open(Arc::clone(&pending) as Arc<dyn ByteSource>, OpenOptions::default()).unwrap();
    cos.edit()
        .set(ObjRef::new(6, 0), lamina_cos::Object::Integer(3))
        .unwrap();
    pending.fetched.store(false, std::sync::atomic::Ordering::Relaxed);
    let mut out = Vec::new();
    let refused = IncrementalWriter::new(&cos, SaveOptions::new())
        .write(&mut out)
        .map(|_| ())
        .map_err(|e| e.code());
    let empty = out.is_empty();
    pending.fetch();
    let retried = IncrementalWriter::new(&cos, SaveOptions::new()).write(&mut out).is_ok();
    assert_eq!((refused, empty, retried), (Err("data-pending"), true, true));
}

#[test]
fn saving_twice_gives_the_same_bytes() {
    let run = || {
        let mut doc = editable(&two_pages());
        set_title(&mut doc, "Twice");
        rotate(&mut doc, 0, Rotation::Clockwise270);
        save(&mut doc, SaveOptions::new())
    };
    assert_eq!(run(), run());
}

fn hex(s: &str) -> Vec<u8> {
    (0..s.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&s[i..i + 2], 16).unwrap())
        .collect()
}
