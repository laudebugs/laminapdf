//! The incremental writer over the overlay (ADR 0007 §5, §7.5.6): what the base keeps, what
//! is appended, and how the appended section matches the base's.
#![allow(
    clippy::unwrap_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "tests: a panic is a test failure"
)]

mod common;

use common::save::ModeSaves as _;

use std::sync::Arc;
use std::sync::atomic::Ordering;

use common::fixture;
use common::save::{
    ID0, Windowed, corpus_doc, editable, facts, free_list_base, ids, reopen, rotate, set_title, two_pages,
    xref_stream_base,
};
use laminapdf_cos::{ByteSource, Document, ObjRef, Object, OpenOptions};
use laminapdf_model::Rotation;
use laminapdf_ops::SaveState;
use laminapdf_save::{IncrementalOutput, IncrementalWriter, SaveError, SaveMode, SaveOptions, SinkError};

fn save(doc: &mut laminapdf_ops::Document, options: SaveOptions) -> Vec<u8> {
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
fn the_appended_xref_stream_has_its_own_section_keys_only() {
    let base = xref_stream_base();
    let startxref = Document::open(base.clone(), OpenOptions::default())
        .unwrap()
        .startxref()
        .unwrap();
    let mut doc = editable(&base);
    set_title(&mut doc, "Streamed");
    let out = save(&mut doc, SaveOptions::new());
    let reopened = reopen(&out);
    // The newest trailer is the appended xref stream's dictionary.
    let trailer = reopened.trailer();
    let keys = ["DL", "DecodeParms", "F", "FFilter", "FDecodeParms"].map(|k| trailer.contains_key(k));
    assert_eq!(
        (
            keys,
            trailer.get("W").cloned(),
            trailer.get("Index").cloned(),
            trailer.get_int("Prev"),
            trailer.get_int("Size"),
        ),
        (
            [false; 5],
            Some(Object::Array(vec![1.into(), 4.into(), 2.into()])),
            // The new Info object (6) and the xref stream (7) are one run.
            Some(Object::Array(vec![6.into(), 2.into()])),
            Some(i64::try_from(startxref).unwrap()),
            Some(8),
        )
    );
}

#[test]
fn deleting_links_the_free_entry_in_front_of_the_bases_free_list() {
    let mut cos = Document::open(free_list_base(), OpenOptions::default()).unwrap();
    cos.edit().delete(ObjRef::new(5, 0)).unwrap();
    let mut out = Vec::new();
    let saved = IncrementalWriter::new(&cos, SaveOptions::new())
        .write(&mut out)
        .unwrap();
    let appended = String::from_utf8_lossy(saved.appended()).into_owned();
    let reopened = reopen(&out);
    assert_eq!(
        (
            appended.contains("xref\n0 1\n0000000005 65535 f\r\n5 1\n0000000003 00001 f\r\n"),
            reopened.base_generation(5),
            reopened.free_list_head(),
        ),
        (true, None, 5),
        "{appended}"
    );
}

#[test]
fn a_classic_section_keeps_the_bases_free_list_head_when_nothing_is_freed() {
    let mut doc = editable(&free_list_base());
    set_title(&mut doc, "Kept");
    let out = save(&mut doc, SaveOptions::new());
    assert_eq!(reopen(&out).free_list_head(), 3);
}

#[test]
fn size_never_shrinks_and_covers_reserved_numbers() {
    let mut cos = Document::open(two_pages(), OpenOptions::default()).unwrap();
    let r = cos.edit().create(Object::Integer(1)).unwrap();
    cos.edit().delete(r).unwrap(); // number 7 stays reserved, unwritten
    cos.edit().set(ObjRef::new(6, 0), Object::Integer(2)).unwrap();
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
    let info = reopened.trailer().get("Info").and_then(Object::as_ref).unwrap();
    assert_eq!((info.gen_nr, info.num >= 6), (0, true));
}

#[test]
fn a_windowed_base_gives_the_same_bytes_as_in_memory() {
    let base = two_pages();
    let windowed = Windowed::new(base.clone());
    let mut a =
        laminapdf_ops::Document::open(Arc::clone(&windowed) as Arc<dyn ByteSource>, OpenOptions::default()).unwrap();
    let mut b = editable(&base);
    set_title(&mut a, "Same");
    set_title(&mut b, "Same");
    assert_eq!(save(&mut a, SaveOptions::new()), save(&mut b, SaveOptions::new()));
}

#[test]
fn a_windowed_base_rebases_onto_base_and_revision() {
    let windowed = Windowed::new(two_pages());
    let mut doc = laminapdf_ops::Document::open(windowed as Arc<dyn ByteSource>, OpenOptions::default()).unwrap();
    set_title(&mut doc, "Rebased");
    save(&mut doc, SaveOptions::new());
    assert_eq!(
        (doc.data(), facts(&doc).title.as_deref(), doc.revisions()),
        (None, Some("Rebased"), 2)
    );
}

/// A windowed copy of [`two_pages`] whose bytes from 100 on can be made to answer `Pending`.
fn pending_two_pages() -> Arc<Windowed> {
    Arc::new(Windowed {
        data: two_pages(),
        missing_from: 100,
        fetched: true.into(),
    })
}

#[test]
fn a_pending_base_fails_before_writing_and_works_once_fetched() {
    let pending = pending_two_pages();
    let mut cos = Document::open(Arc::clone(&pending) as Arc<dyn ByteSource>, OpenOptions::default()).unwrap();
    cos.edit().set(ObjRef::new(6, 0), Object::Integer(3)).unwrap();
    pending.fetched.store(false, Ordering::Relaxed);
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
fn a_pending_base_through_save_lists_every_missing_range_and_stays_dirty() {
    let pending = pending_two_pages();
    let len = pending.data.len() as u64;
    let mut doc =
        laminapdf_ops::Document::open(Arc::clone(&pending) as Arc<dyn ByteSource>, OpenOptions::default()).unwrap();
    set_title(&mut doc, "Later");
    pending.fetched.store(false, Ordering::Relaxed);
    let mut out = Vec::new();
    let result = doc.save_incremental(&mut out, &SaveOptions::new());
    assert_eq!(
        (result.err(), out.len(), doc.save_state(), facts(&doc).title.as_deref()),
        (
            Some(SaveError::Cos(laminapdf_cos::Error::DataPending {
                ranges: vec![(0, len)]
            })),
            0,
            SaveState::Dirty,
            Some("Later")
        )
    );
}

#[test]
fn the_writer_refuses_a_repaired_base_when_there_is_something_to_append() {
    // The planner refuses first (`RequiresRewrite`, tests/planner.rs); the writer checks too.
    let mut doc = corpus_doc("hello-broken.pdf");
    rotate(&mut doc, 0, Rotation::Half);
    let mut out = Vec::new();
    let result = IncrementalWriter::new(doc.cos(), SaveOptions::new()).write(&mut out);
    assert_eq!(
        (result.err(), out.len()),
        (Some(SaveError::Cos(laminapdf_cos::Error::RepairedBase)), 0)
    );
}

#[test]
fn the_writer_copies_a_clean_repaired_base_unchanged() {
    let base = fixture("corpus/hello-broken.pdf");
    let doc = editable(&base);
    let mut out = Vec::new();
    IncrementalWriter::new(doc.cos(), SaveOptions::new())
        .write(&mut out)
        .unwrap();
    assert_eq!(out, base);
}

#[test]
fn size_covers_objects_dropped_by_max_object_number() {
    // Objects 1–3 and 9, `/Size 4`; opened with max_object_number 8, so 9 is dropped.
    let mut bytes = b"%PDF-1.7\n".to_vec();
    let mut offsets = Vec::new();
    for (num, body) in [
        (1, "<< /Type /Catalog /Pages 2 0 R >>"),
        (2, "<< /Type /Pages /Kids [3 0 R] /Count 1 >>"),
        (3, "<< /Type /Page /Parent 2 0 R >>"),
        (9, "42"),
    ] {
        offsets.push(bytes.len());
        bytes.extend_from_slice(format!("{num} 0 obj\n{body}\nendobj\n").as_bytes());
    }
    let xref = bytes.len();
    bytes.extend_from_slice(b"xref\n0 4\n0000000000 65535 f\r\n");
    for off in &offsets[..3] {
        bytes.extend_from_slice(format!("{off:010} 00000 n\r\n").as_bytes());
    }
    bytes.extend_from_slice(format!("9 1\n{:010} 00000 n\r\n", offsets[3]).as_bytes());
    bytes.extend_from_slice(format!("trailer\n<< /Size 4 /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF\n").as_bytes());
    let limits = laminapdf_cos::Limits::default().with_max_object_number(8);
    let mut cos = Document::open(bytes, OpenOptions::default().with_limits(limits)).unwrap();
    cos.edit().set(ObjRef::new(3, 0), Object::Null).unwrap();
    let mut out = Vec::new();
    IncrementalWriter::new(&cos, SaveOptions::new())
        .write(&mut out)
        .unwrap();
    assert_eq!(
        Document::open(out, OpenOptions::default())
            .unwrap()
            .trailer()
            .get_int("Size"),
        Some(10)
    );
}

#[test]
fn an_xref_stream_number_past_max_object_number_is_refused_before_writing() {
    let base = xref_stream_base();
    // Objects 1–5; the xref stream would be 6.
    let limits = laminapdf_cos::Limits::default().with_max_object_number(5);
    let mut cos = Document::open(base, OpenOptions::default().with_limits(limits)).unwrap();
    cos.edit().set(ObjRef::new(3, 0), Object::Null).unwrap();
    let mut out = Vec::new();
    let result = IncrementalWriter::new(&cos, SaveOptions::new()).write(&mut out);
    assert_eq!(
        (result.map(|_| ()).map_err(|e| e.code()), out.len()),
        (Err("invalid-structure"), 0)
    );
}

#[test]
fn a_sink_that_already_holds_bytes_is_refused() {
    let mut doc = editable(&two_pages());
    set_title(&mut doc, "A");
    let mut out = b"junk".to_vec();
    assert_eq!(
        (doc.save_incremental(&mut out, &SaveOptions::new()).err(), out.len()),
        (Some(SaveError::Sink(SinkError::NotEmpty { held: 4 })), 4)
    );
}

#[test]
fn appended_only_into_a_sink_holding_the_base_makes_the_whole_file() {
    let base = two_pages();
    let mut doc = editable(&base);
    set_title(&mut doc, "Appended");
    let mut out = base;
    doc.save_incremental(
        &mut out,
        &SaveOptions::new().with_output(IncrementalOutput::AppendedOnly),
    )
    .unwrap();
    assert_eq!(facts(&reopen(&out)).title.as_deref(), Some("Appended"));
}

#[test]
fn id1_depends_on_the_base_content_not_only_its_length() {
    let other = String::from_utf8(two_pages())
        .unwrap()
        .replace("(Base)", "(Bxse)")
        .into_bytes();
    let id1 = |base: &[u8]| {
        let mut doc = editable(base);
        set_title(&mut doc, "Same");
        ids(&reopen(&save(&mut doc, SaveOptions::new()))).1
    };
    assert_eq!(two_pages().len(), other.len());
    assert_ne!(id1(&two_pages()), id1(&other));
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
