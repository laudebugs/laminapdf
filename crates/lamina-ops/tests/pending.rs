//! `PendingChanges` (ADR 0007 §2): the op classes the save planner checks against signatures.
//! They err on the side of reporting, and only an empty overlay or a rebase clears them.

mod common;

use std::collections::BTreeSet;

use common::{corpus, page_ids, rotate, set_title};
use lamina_cos::{ObjRef, Object};
use lamina_model::Rotation;
use lamina_ops::{Document, Error, ModificationClass, Origin, PeerId};

fn classes(doc: &Document) -> Vec<ModificationClass> {
    doc.pending_changes().classes().iter().copied().collect()
}

#[test]
fn a_committed_op_adds_its_class() {
    let mut doc = corpus("hello.pdf");
    set_title(&mut doc, Origin::Local, Some("A"));
    let page = page_ids(&doc).remove(0);
    rotate(&mut doc, &page, Rotation::Half);
    assert_eq!(
        (classes(&doc), doc.pending_changes().requires_rewrite()),
        (
            vec![ModificationClass::Metadata, ModificationClass::PageStructure],
            false
        )
    );
}

#[test]
fn a_remote_change_counts_too() {
    let mut doc = corpus("hello.pdf");
    set_title(&mut doc, Origin::Remote(PeerId::new("peer")), Some("A"));
    assert_eq!(classes(&doc), [ModificationClass::Metadata]);
}

#[test]
fn a_rolled_back_transaction_adds_nothing() {
    let mut doc = corpus("hello.pdf");
    let page = page_ids(&doc).remove(0);
    doc.begin(Origin::Local).unwrap();
    doc.apply(lamina_ops::RotatePage::new(page, Rotation::Half)).unwrap();
    doc.rollback().unwrap();
    assert!(doc.pending_changes().is_empty());
}

#[test]
fn a_failed_savepoint_adds_nothing() {
    let mut doc = corpus("hello.pdf");
    let page = page_ids(&doc).remove(0);
    doc.transact(Origin::Local, |tx| {
        let _ = tx.savepoint(|tx| {
            tx.apply(lamina_ops::RotatePage::new(page.clone(), Rotation::Half))?;
            Err::<(), _>(Error::NoTransaction)
        });
        tx.apply(lamina_ops::SetInfo::new(
            lamina_ops::InfoPatch::new().title(Some("A")),
            None,
        ))?;
        Ok(())
    })
    .unwrap();
    assert_eq!(classes(&doc), [ModificationClass::Metadata]);
}

#[test]
fn undoing_to_an_empty_overlay_clears_them_and_redo_brings_them_back() {
    let mut doc = corpus("hello.pdf");
    set_title(&mut doc, Origin::Local, Some("A"));
    doc.undo().unwrap();
    let after_undo = doc.pending_changes().is_empty();
    doc.redo().unwrap();
    assert_eq!((after_undo, classes(&doc)), (true, vec![ModificationClass::Metadata]));
}

#[test]
fn an_edit_outside_history_adds_no_class_and_rolls_back() {
    let mut doc = corpus("hello.pdf");
    let mark = doc
        .edit_outside_history(|cos| cos.set(ObjRef::new(1, 0), Object::Null))
        .unwrap();
    let dirty = doc.is_dirty();
    doc.rollback_outside_history(mark).unwrap();
    assert_eq!(
        (
            dirty,
            doc.pending_changes().classes().clone(),
            doc.is_dirty(),
            doc.history().can_undo()
        ),
        (true, BTreeSet::new(), false, false)
    );
}

#[test]
fn an_edit_outside_history_is_refused_in_a_transaction() {
    let mut doc = corpus("hello.pdf");
    doc.begin(Origin::Local).unwrap();
    assert_eq!(
        doc.edit_outside_history(|_| Ok(())).err(),
        Some(Error::NestedTransaction)
    );
}

#[test]
fn a_failing_edit_outside_history_leaves_nothing() {
    let mut doc = corpus("hello.pdf");
    let result = doc.edit_outside_history(|cos| {
        cos.set(ObjRef::new(1, 0), Object::Null)?;
        Err(lamina_cos::Error::RepairedBase)
    });
    assert_eq!(
        (result.err(), doc.is_dirty()),
        (Some(Error::Cos(lamina_cos::Error::RepairedBase)), false)
    );
}
