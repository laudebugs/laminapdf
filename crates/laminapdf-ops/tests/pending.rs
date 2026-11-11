//! `PendingChanges` (ADR 0007 §2): the op classes the save planner checks against signatures.
//! They err on the side of reporting, and only an empty overlay or a rebase clears them.
#![expect(
    deprecated,
    reason = "these tests check that SetInfo, deprecated but still on the wire, keeps working"
)]

mod common;

use common::{corpus, page_ids, rotate, set_title};
use laminapdf_cos::{ObjRef, Object};
use laminapdf_model::Rotation;
use laminapdf_ops::{Document, Error, ModificationClass, Origin, PeerId};

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
    doc.apply(laminapdf_ops::RotatePage::new(page, Rotation::Half)).unwrap();
    doc.rollback().unwrap();
    assert!(doc.pending_changes().is_empty());
}

#[test]
fn a_failed_savepoint_adds_nothing() {
    let mut doc = corpus("hello.pdf");
    let page = page_ids(&doc).remove(0);
    doc.transact(Origin::Local, |tx| {
        let _ = tx.savepoint(|tx| {
            tx.apply(laminapdf_ops::RotatePage::new(page.clone(), Rotation::Half))?;
            Err::<(), _>(Error::NoTransaction)
        });
        tx.apply(laminapdf_ops::SetInfo::patch(
            laminapdf_ops::InfoPatch::new().title(Some("A")),
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
fn a_model_document_wrapped_while_dirty_has_unknown_changes() {
    let mut model = laminapdf_model::Document::open(
        common::fixture("corpus/hello.pdf"),
        laminapdf_cos::OpenOptions::default(),
    )
    .unwrap();
    model.cos_mut().edit().set(ObjRef::new(1, 0), Object::Null).unwrap();
    let doc = Document::from(model);
    let pending = doc.pending_changes();
    assert_eq!(
        (
            pending.is_unknown(),
            pending.requires_rewrite(),
            pending.classes().len()
        ),
        (true, true, 7)
    );
}

#[test]
fn a_clean_model_document_has_no_pending_changes() {
    let model = laminapdf_model::Document::open(
        common::fixture("corpus/hello.pdf"),
        laminapdf_cos::OpenOptions::default(),
    )
    .unwrap();
    assert!(Document::from(model).pending_changes().is_empty());
}
