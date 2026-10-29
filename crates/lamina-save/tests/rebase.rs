//! The save cycle (ADR 0005 §3): `Clean | Dirty | SavedNotRebased`, rebasing, and what
//! happens to history and numbers afterwards.
#![allow(
    clippy::unwrap_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "tests: a panic is a test failure"
)]

mod common;

use common::save::{editable, facts, rotate, set_title, two_pages};
use lamina_cos::Object;
use lamina_model::Rotation;
use lamina_ops::{Error, Origin, SaveState};
use lamina_save::{IoWrite, Renumber, Save, SaveError, SaveOptions};

#[test]
fn a_document_is_clean_then_dirty_then_clean_after_an_incremental_save() {
    let mut doc = editable(&two_pages());
    let opened = doc.save_state();
    set_title(&mut doc, "A");
    let edited = doc.save_state();
    doc.save_incremental(&mut Vec::new(), &SaveOptions::new()).unwrap();
    assert_eq!(
        (opened, edited, doc.save_state(), doc.has_unsaved_changes()),
        (SaveState::Clean, SaveState::Dirty, SaveState::Clean, false)
    );
}

#[test]
fn after_an_incremental_save_the_base_is_the_output() {
    let mut doc = editable(&two_pages());
    set_title(&mut doc, "A");
    let mut out = Vec::new();
    doc.save_incremental(&mut out, &SaveOptions::new()).unwrap();
    assert_eq!((doc.data(), doc.revisions()), (Some(&out[..]), 2));
}

#[test]
fn a_full_rewrite_to_a_readable_sink_rebases() {
    let mut doc = editable(&two_pages());
    rotate(&mut doc, 0, Rotation::Half);
    doc.save_full_and_rebase(Vec::new(), &SaveOptions::new()).unwrap();
    assert_eq!(
        (doc.save_state(), doc.base_generation(6), facts(&doc).rotations[0]),
        (SaveState::Clean, None, Rotation::Half)
    );
}

#[test]
fn a_full_rewrite_to_a_write_only_sink_leaves_saved_not_rebased() {
    let mut doc = editable(&two_pages());
    set_title(&mut doc, "A");
    let mut sink = IoWrite::new(Vec::new());
    doc.save_full(&mut sink, &SaveOptions::new()).unwrap();
    assert_eq!(
        (doc.save_state(), doc.has_unsaved_changes(), doc.is_dirty()),
        (SaveState::SavedNotRebased, false, true)
    );
}

#[test]
fn saved_not_rebased_refuses_ops_undo_redo_and_saves_until_rebase() {
    let mut doc = editable(&two_pages());
    set_title(&mut doc, "A");
    let mut sink = IoWrite::new(Vec::new());
    doc.save_full(&mut sink, &SaveOptions::new()).unwrap();
    let op = doc.transact(Origin::Local, |_| Ok(())).map(|_| ()).err();
    let undo = doc.undo().err();
    let redo = doc.redo().err();
    let save = doc.save_incremental(&mut Vec::new(), &SaveOptions::new()).err();
    let full = doc.save_full(&mut Vec::new(), &SaveOptions::new()).err();
    let reads = facts(&doc).title;
    doc.rebase(sink.into_inner()).unwrap();
    assert_eq!(
        (op, undo, redo, save, full, reads, doc.save_state(), facts(&doc).title),
        (
            Some(Error::SavedNotRebased),
            Some(Error::SavedNotRebased),
            Some(Error::SavedNotRebased),
            Some(SaveError::Ops(Error::SavedNotRebased)),
            Some(SaveError::Ops(Error::SavedNotRebased)),
            Some("A".to_owned()),
            SaveState::Clean,
            Some("A".to_owned())
        )
    );
}

#[test]
fn rebase_refuses_changes_no_save_wrote() {
    let mut doc = editable(&two_pages());
    set_title(&mut doc, "Unsaved");
    assert_eq!(doc.rebase(two_pages()), Err(Error::UnsavedChanges));
}

#[test]
fn saving_inside_a_transaction_is_refused() {
    let mut doc = editable(&two_pages());
    doc.begin(Origin::Local).unwrap();
    assert_eq!(
        doc.save_incremental(&mut Vec::new(), &SaveOptions::new()).err(),
        Some(SaveError::Ops(Error::NestedTransaction))
    );
}

#[test]
fn undo_after_a_save_restores_the_saved_value_and_saves_again() {
    let mut doc = editable(&two_pages());
    set_title(&mut doc, "First");
    doc.save_incremental(&mut Vec::new(), &SaveOptions::new()).unwrap();
    set_title(&mut doc, "Second");
    doc.save_incremental(&mut Vec::new(), &SaveOptions::new()).unwrap();
    doc.undo().unwrap();
    let mut out = Vec::new();
    doc.save_incremental(&mut out, &SaveOptions::new()).unwrap();
    let reopened = common::save::reopen(&out);
    assert_eq!(
        (facts(&reopened).title.as_deref(), reopened.revisions()),
        (Some("First"), 4)
    );
}

#[test]
fn undo_is_unsaved_after_a_save() {
    let mut doc = editable(&two_pages());
    set_title(&mut doc, "A");
    doc.save_incremental(&mut Vec::new(), &SaveOptions::new()).unwrap();
    doc.undo().unwrap();
    assert_eq!((doc.has_unsaved_changes(), doc.save_state()), (true, SaveState::Dirty));
}

#[test]
fn compact_clears_the_history() {
    let mut doc = editable(&two_pages());
    set_title(&mut doc, "A");
    doc.save_full_and_rebase(Vec::new(), &SaveOptions::new().with_renumber(Renumber::Compact))
        .unwrap();
    assert_eq!((doc.history().can_undo(), doc.has_unsaved_changes()), (false, false));
}

#[test]
fn keep_keeps_the_history() {
    let mut doc = editable(&two_pages());
    set_title(&mut doc, "A");
    doc.save_full_and_rebase(Vec::new(), &SaveOptions::new()).unwrap();
    doc.undo().unwrap();
    assert_eq!(facts(&doc).title.as_deref(), Some("Base"));
}

#[test]
fn numbers_reserved_before_a_save_are_not_handed_out_after_it() {
    let mut cos = lamina_cos::Document::open(two_pages(), lamina_cos::OpenOptions::default()).unwrap();
    let r = cos.edit().create(Object::Integer(1)).unwrap();
    cos.edit().delete(r).unwrap();
    cos.edit()
        .set(lamina_cos::ObjRef::new(6, 0), Object::Integer(2))
        .unwrap();
    let mut out = Vec::new();
    lamina_save::IncrementalWriter::new(&cos, SaveOptions::new())
        .write(&mut out)
        .unwrap();
    cos.rebase(out).unwrap();
    let next = cos.edit().create(Object::Integer(3)).unwrap();
    assert!(next.num > r.num, "{next:?} reuses {r:?}");
}

#[test]
fn redo_of_a_creation_after_a_save_recreates_its_number() {
    let mut doc = common::save::corpus_doc("hello.pdf");
    set_title(&mut doc, "Created"); // hello.pdf has no Info: the op creates one
    let info = doc.trailer().get("Info").and_then(Object::as_ref).unwrap();
    doc.undo().unwrap();
    // A remote change keeps the redo stack and gives the save something to write.
    let page = common::save::page_ids(&doc).remove(0);
    doc.apply_all(
        Origin::Remote(lamina_ops::PeerId::new("peer")),
        [lamina_ops::RotatePage::new(page, Rotation::Half).into()],
    )
    .unwrap();
    doc.save_incremental(&mut Vec::new(), &SaveOptions::new()).unwrap();
    doc.redo().unwrap();
    let mut out = Vec::new();
    doc.save_incremental(&mut out, &SaveOptions::new()).unwrap();
    let reopened = common::save::reopen(&out);
    assert_eq!(
        (
            reopened.trailer().get("Info").and_then(Object::as_ref),
            facts(&reopened).title.as_deref()
        ),
        (Some(info), Some("Created"))
    );
}
