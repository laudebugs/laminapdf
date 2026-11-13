//! The page-structure ops (E2 plan §2.1) over `fixtures/pages/`: what each does to the page
//! list, the labels, the destinations that named deleted pages (E2 plan §11) and the structure
//! tree, and the widgets each reports.
#![allow(
    clippy::unwrap_used,
    clippy::indexing_slicing,
    reason = "tests: a panic is a test failure"
)]

mod common;

use common::{fixture, open, page_ids, rotations};
use laminapdf_cos::{ObjRef, Object};
use laminapdf_model::{PagePosition, PageSize, ReferenceKind, Rotation};
use laminapdf_ops::{
    DeletePages, Document, Error, InsertBlankPages, MovePages, Op, Origin, ReferenceOutcome, ReferencePolicy,
    ReversePages, RotatePages, WidgetChange,
};

fn pages(name: &str) -> Document {
    open(&fixture(&format!("pages/{name}")))
}

fn apply(doc: &mut Document, op: impl Into<Op>) -> laminapdf_ops::Applied {
    doc.apply_all(Origin::Local, [op.into()]).unwrap()
}

fn labels(doc: &Document) -> Vec<Option<String>> {
    let labels = doc.page_labels().unwrap();
    (0..page_ids(doc).len()).map(|i| labels.label(i)).collect()
}

/// The value at `key` of object `r` (one reference followed).
fn entry(doc: &Document, r: ObjRef, key: &str) -> Option<Object> {
    let object = doc.resolve(r).unwrap();
    let value = object.as_dict()?.get(key)?.clone();
    Some(match value {
        Object::Ref(v) => (*doc.resolve(v).unwrap()).clone(),
        other => other,
    })
}

/// `[page /Fit]` for the page at `index` before the edit.
fn fit(ids: &[laminapdf_model::PageId], index: usize) -> Object {
    let r = ids[index].as_lamina_id().obj_ref().unwrap();
    Object::Array(vec![Object::Ref(r), Object::name("Fit")])
}

#[test]
fn inserting_blank_pages_adds_new_pages_after_the_anchor() {
    let mut doc = pages("inherited.pdf");
    let before = page_ids(&doc);
    let applied = apply(
        &mut doc,
        InsertBlankPages::new(PagePosition::After(before[0].clone()), 2, PageSize::A4)
            .with_rotate(Rotation::Clockwise90),
    );
    let after = page_ids(&doc);
    assert_eq!(
        (
            after.len(),
            &after[0],
            &after[3..],
            applied.page_order.as_ref(),
            rotations(&doc)[1..3].to_vec()
        ),
        (
            before.len() + 2,
            &before[0],
            &before[1..],
            Some(&after),
            vec![Rotation::Clockwise90; 2]
        )
    );
}

#[test]
fn inserted_pages_continue_the_label_range_they_land_in() {
    let mut doc = pages("labels.pdf");
    let before = labels(&doc);
    let ids = page_ids(&doc);
    apply(
        &mut doc,
        InsertBlankPages::new(PagePosition::After(ids[4].clone()), 1, PageSize::LETTER),
    );
    let after = labels(&doc);
    assert_eq!(
        (&after[..5], after[5].as_deref(), &after[6..]),
        (&before[..5], Some("3"), &before[5..])
    );
}

#[test]
fn deleting_moving_and_reversing_keep_each_surviving_pages_label() {
    let ids = page_ids(&pages("labels.pdf"));
    let before = labels(&pages("labels.pdf"));
    let label_of = |doc: &Document| -> Vec<(laminapdf_model::PageId, Option<String>)> {
        page_ids(doc).into_iter().zip(labels(doc)).collect()
    };
    let expected = |kept: &[laminapdf_model::PageId]| -> Vec<(laminapdf_model::PageId, Option<String>)> {
        kept.iter()
            .map(|id| (id.clone(), before[ids.iter().position(|i| i == id).unwrap()].clone()))
            .collect()
    };
    let mut deleted = pages("labels.pdf");
    apply(&mut deleted, DeletePages::new(vec![ids[1].clone(), ids[5].clone()]));
    let mut moved = pages("labels.pdf");
    apply(&mut moved, MovePages::new(vec![ids[8].clone()], PagePosition::Start));
    let mut reversed = pages("labels.pdf");
    apply(&mut reversed, ReversePages::new(ids[3..7].to_vec()));
    assert_eq!(
        [label_of(&deleted), label_of(&moved), label_of(&reversed)],
        [
            expected(&page_ids(&deleted)),
            expected(&page_ids(&moved)),
            expected(&page_ids(&reversed))
        ]
    );
}

#[test]
fn moving_pages_keeps_their_ids_and_order() {
    // A page listed twice reads as a cycle until the first edit copies it, so the IDs come
    // from the page-tree editor.
    let mut model =
        laminapdf_model::Document::open(fixture("pages/unbalanced.pdf"), laminapdf_cos::OpenOptions::default())
            .unwrap();
    let ids = model.edit_page_tree().unwrap().page_ids();
    let mut doc = pages("unbalanced.pdf");
    apply(
        &mut doc,
        MovePages::new(vec![ids[5].clone(), ids[1].clone()], PagePosition::End),
    );
    let after = page_ids(&doc);
    assert_eq!(&after[after.len() - 2..], [ids[5].clone(), ids[1].clone()]);
}

#[test]
fn reversing_fills_the_pages_places_in_reverse() {
    let mut doc = pages("inherited.pdf");
    let ids = page_ids(&doc);
    apply(
        &mut doc,
        ReversePages::new(vec![ids[0].clone(), ids[2].clone(), ids[4].clone()]),
    );
    let after = page_ids(&doc);
    assert_eq!(
        [&after[0], &after[1], &after[2], &after[3], &after[4]],
        [&ids[4], &ids[1], &ids[2], &ids[3], &ids[0]]
    );
}

#[test]
fn rotating_sets_or_turns_each_page() {
    let mut set = pages("inherited.pdf");
    let mut turn = pages("inherited.pdf");
    let ids = page_ids(&set);
    let before = rotations(&set);
    apply(&mut set, RotatePages::set(ids[..2].to_vec(), Rotation::Half));
    apply(&mut turn, RotatePages::turn(ids[..2].to_vec(), Rotation::Clockwise90));
    let turned = |r: Rotation| Rotation::from_canonical_degrees((r.degrees() + 90) % 360).unwrap();
    assert_eq!(
        (rotations(&set)[..2].to_vec(), rotations(&turn)[..2].to_vec()),
        (vec![Rotation::Half; 2], vec![turned(before[0]), turned(before[1])])
    );
}

#[test]
fn deleting_every_page_is_refused() {
    let mut doc = pages("inherited.pdf");
    let ids = page_ids(&doc);
    let err = doc
        .apply_all(Origin::Local, [DeletePages::new(ids).into()])
        .unwrap_err();
    assert_eq!((err, doc.is_dirty()), (Error::NoPagesLeft, false));
}

#[test]
fn deleting_a_page_retargets_what_named_it_to_the_next_page() {
    let mut doc = pages("references.pdf");
    let ids = page_ids(&doc);
    let catalog = doc.trailer().get("Root").and_then(Object::as_ref).unwrap();
    let outline = doc.outline().unwrap();
    let items: Vec<ObjRef> = outline
        .iter()
        .map(|i| i.id().unwrap().as_lamina_id().obj_ref().unwrap())
        .collect();
    drop(outline);
    let applied = apply(&mut doc, DeletePages::new(vec![ids[2].clone()]));
    let to_page_4 = fit(&ids, 3);
    let names = doc.name_tree("Dests").unwrap().unwrap().get(b"chap3").unwrap();
    let goto = entry(&doc, items[1], "A").unwrap();
    assert_eq!(
        (
            entry(&doc, items[0], "Dest"),
            goto.as_dict().and_then(|a| a.get("D").cloned()),
            names,
            entry(&doc, catalog, "OpenAction"),
        ),
        (
            Some(to_page_4.clone()),
            Some(to_page_4.clone()),
            Some(to_page_4.clone()),
            Some(to_page_4)
        )
    );
    assert!(
        applied
            .references
            .iter()
            .filter(|c| !matches!(
                c.reference.kind,
                ReferenceKind::StructElem { .. }
                    | ReferenceKind::StructContentRef { .. }
                    | ReferenceKind::Widget { .. }
            ))
            .all(|c| c.outcome == ReferenceOutcome::Retargeted { to: ids[3].clone() }),
        "{:?}",
        applied.references
    );
}

#[test]
fn deleting_with_remove_drops_what_named_the_page() {
    let mut doc = pages("references.pdf");
    let ids = page_ids(&doc);
    let catalog = doc.trailer().get("Root").and_then(Object::as_ref).unwrap();
    let first_item = doc
        .outline()
        .unwrap()
        .iter()
        .next()
        .unwrap()
        .id()
        .unwrap()
        .as_lamina_id()
        .obj_ref()
        .unwrap();
    apply(
        &mut doc,
        DeletePages::new(vec![ids[2].clone()]).with_references(ReferencePolicy::Remove),
    );
    // The link stays on its page (its structure entries stay right), with no destination.
    let link = doc.pages().unwrap().get(0).unwrap().annots().unwrap()[0];
    assert_eq!(
        (
            entry(&doc, first_item, "Dest"),
            doc.name_tree("Dests").unwrap().unwrap().get(b"chap3").unwrap(),
            entry(&doc, catalog, "OpenAction"),
            entry(&doc, link, "Dest"),
        ),
        (None, None, None, None)
    );
}

#[test]
fn deleting_a_page_removes_the_structure_elements_only_it_held() {
    let mut doc = pages("references.pdf");
    let ids = page_ids(&doc);
    let applied = apply(&mut doc, DeletePages::new(vec![ids[2].clone()]));
    let structure = applied
        .references
        .iter()
        .filter(|c| {
            matches!(
                c.reference.kind,
                ReferenceKind::StructElem { .. } | ReferenceKind::StructContentRef { .. }
            )
        })
        .count();
    let tree = doc.structure().unwrap().unwrap();
    let parent_tree = tree
        .parent_tree
        .map(|root| doc.number_tree(root).entries().unwrap().len());
    // Paragraph 3 (all on page 3) goes; paragraph 2 keeps its own content and loses the MCR
    // and OBJR on page 3; the parent tree loses page 3's key and its widget's.
    assert_eq!((structure, parent_tree), (3, Some(1)));
}

#[test]
fn deleting_takes_the_deleted_pages_widgets_out_of_the_form() {
    let mut doc = pages("references.pdf");
    let ids = page_ids(&doc);
    let catalog = doc.trailer().get("Root").and_then(Object::as_ref).unwrap();
    let fields_before = entry(&doc, catalog, "AcroForm")
        .unwrap()
        .as_dict()
        .unwrap()
        .get("Fields")
        .unwrap()
        .as_array()
        .unwrap()
        .len();
    apply(&mut doc, DeletePages::new(vec![ids[2].clone()]));
    let form = entry(&doc, catalog, "AcroForm").unwrap();
    let fields = form.as_dict().unwrap().get("Fields").unwrap().as_array().unwrap().len();
    // "name" (merged, on page 3), "city" (its one widget on page 3) and "addr" (both its
    // widgets on page 3) leave the form; "zip", on page 4, stays.
    assert_eq!((fields_before, fields), (4, 1));
}

#[test]
fn deleting_reports_the_widgets_on_the_deleted_page() {
    let mut doc = pages("references.pdf");
    let ids = page_ids(&doc);
    let applied = apply(&mut doc, DeletePages::new(vec![ids[2].clone()]));
    assert_eq!(
        applied.widgets.iter().map(|w| w.change).collect::<Vec<_>>(),
        vec![WidgetChange::Deleted; 4]
    );
}

#[test]
fn moving_reports_moved_widgets_and_rotating_reshown_ones() {
    let ids = page_ids(&pages("forms.pdf"));
    let mut moved = pages("forms.pdf");
    let mut rotated = pages("forms.pdf");
    let m = apply(&mut moved, MovePages::new(vec![ids[0].clone()], PagePosition::End));
    let r = apply(&mut rotated, RotatePages::set(vec![ids[0].clone()], Rotation::Half));
    let changes = |a: &laminapdf_ops::Applied| a.widgets.iter().map(|w| w.change).collect::<Vec<_>>();
    assert_eq!(
        (changes(&m).first().copied(), changes(&r).first().copied()),
        (Some(WidgetChange::Moved), Some(WidgetChange::Reshown))
    );
}

#[test]
fn page_changes_name_the_pages_before_the_op_applies() {
    let ids = page_ids(&pages("inherited.pdf"));
    let op: Op = DeletePages::new(vec![ids[1].clone()]).into();
    assert_eq!(
        op.page_changes()
            .iter()
            .map(|c| (c.page.clone(), c.change))
            .collect::<Vec<_>>(),
        [(ids[1].clone(), WidgetChange::Deleted)]
    );
}
