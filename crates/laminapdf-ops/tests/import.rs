//! `InsertPagesFrom`, `ReplacePages` and `extract_pages` (E2 plan §2.1, §2.2) over
//! `fixtures/pages/` and `fixtures/signed/`: what follows a page copied in from another document
//! (annotations, links and the named destinations they use, form fields named or linked, tags
//! dropped), what a replacement does to references to the replaced page, undo, and new
//! documents.
#![allow(
    clippy::unwrap_used,
    clippy::indexing_slicing,
    reason = "tests: a panic is a test failure"
)]

mod common;

use common::{fixture, open, page_ids, snapshot};
use laminapdf_cos::{ObjRef, Object};
use laminapdf_model::{FieldId, PageId, PagePosition};
use laminapdf_ops::{
    Carry, Document, Error, FieldClash, FieldOutcome, InsertPagesFrom, Origin, ReferenceOutcome, ReplacePages, SourceId,
};

fn pages(name: &str) -> Document {
    open(&fixture(&format!("pages/{name}")))
}

/// `doc` with `name` from `fixtures/` added as a source, and the source's page IDs.
fn with_source(doc: &mut Document, name: &str) -> (SourceId, Vec<PageId>) {
    let id = doc.add_source(fixture(name), None).unwrap();
    let ids = page_ids(doc.source_document(&id).unwrap());
    (id, ids)
}

fn obj(id: &PageId) -> ObjRef {
    id.as_lamina_id().obj_ref().unwrap()
}

fn dict(doc: &laminapdf_model::Document, r: ObjRef) -> laminapdf_cos::Dict {
    doc.resolve(r).unwrap().as_dict().unwrap().clone()
}

/// The dictionaries of page `page`'s annotations.
fn annots(doc: &laminapdf_model::Document, page: &PageId) -> Vec<(ObjRef, laminapdf_cos::Dict)> {
    let list = dict(doc, obj(page)).get("Annots").cloned();
    let items = match list {
        Some(Object::Array(items)) => items,
        _ => Vec::new(),
    };
    items
        .iter()
        .filter_map(Object::as_ref)
        .map(|r| (r, dict(doc, r)))
        .collect()
}

fn subtype(d: &laminapdf_cos::Dict) -> &[u8] {
    d.get_name("Subtype").unwrap_or_default()
}

fn apply(doc: &mut Document, op: impl Into<laminapdf_ops::Op>) -> laminapdf_ops::Applied {
    doc.apply_all(Origin::Local, [op.into()]).unwrap()
}

#[test]
fn copies_go_where_asked_as_new_objects_with_their_annotations_pointing_at_them() {
    let mut doc = pages("forms.pdf");
    let before = page_ids(&doc);
    let (src, ids) = with_source(&mut doc, "pages/references.pdf");
    let applied = apply(
        &mut doc,
        InsertPagesFrom::new(
            src,
            vec![ids[2].clone(), ids[3].clone()],
            PagePosition::After(before[0].clone()),
        ),
    );
    let after = page_ids(&doc);
    assert_eq!(after.len(), before.len() + 2);
    assert_eq!((&after[0], &after[3..]), (&before[0], &before[1..]));
    let copies = &after[1..3];
    assert_eq!(applied.page_order.as_deref(), Some(&after[..]));
    for copy in copies {
        assert!(!ids.contains(copy), "a copy has a new ID");
        for (_, annot) in annots(&doc, copy) {
            assert_eq!(annot.get("P").and_then(Object::as_ref), Some(obj(copy)));
            assert!(annot.get("StructParent").is_none());
        }
    }
    // Page 3's four widgets, and page 4's link to "chap3" (page 3, which came along) and widget.
    let kinds: Vec<Vec<Vec<u8>>> = copies
        .iter()
        .map(|c| annots(&doc, c).iter().map(|(_, d)| subtype(d).to_vec()).collect())
        .collect();
    assert_eq!(kinds[0].len(), 4);
    assert_eq!(kinds[1], [b"Link".to_vec(), b"Widget".to_vec()]);
    assert_eq!(applied.links_removed, Vec::<PageId>::new());
}

#[test]
fn a_link_to_a_page_left_behind_is_removed_and_reported() {
    let mut doc = pages("forms.pdf");
    let (src, ids) = with_source(&mut doc, "pages/references.pdf");
    // Page 2's link is a GoTo to page 3, left behind; page 5's names page 2, which comes too.
    let applied = apply(
        &mut doc,
        InsertPagesFrom::new(src, vec![ids[1].clone(), ids[4].clone()], PagePosition::End),
    );
    let after = page_ids(&doc);
    let (two, five) = (&after[after.len() - 2], &after[after.len() - 1]);
    assert_eq!(applied.links_removed, vec![two.clone()]);
    assert_eq!(annots(&doc, two).len(), 0);
    let (_, link) = annots(&doc, five).into_iter().next().unwrap();
    let dest = link.get("Dest").unwrap().as_array().unwrap().to_vec();
    assert_eq!(dest[0].as_ref(), Some(obj(two)));
}

#[test]
fn a_link_kept_between_pages_copied_together_names_the_copy() {
    let mut doc = pages("forms.pdf");
    let (src, ids) = with_source(&mut doc, "pages/references.pdf");
    // Page 2's GoTo names page 3, which comes too.
    apply(
        &mut doc,
        InsertPagesFrom::new(src, vec![ids[1].clone(), ids[2].clone()], PagePosition::End),
    );
    let after = page_ids(&doc);
    let (two, three) = (&after[after.len() - 2], &after[after.len() - 1]);
    let (_, link) = annots(&doc, two).into_iter().next().unwrap();
    let action = match link.get("A").unwrap() {
        Object::Ref(r) => dict(&doc, *r),
        Object::Dict(d) => d.clone(),
        _ => unreachable!(),
    };
    let dest = action.get("D").unwrap().as_array().unwrap().to_vec();
    assert_eq!(dest[0].as_ref(), Some(obj(three)));
}

#[test]
fn a_named_destination_a_copied_link_uses_comes_along_renamed_on_a_clash() {
    // The target is the source's own file, so "chap3" is taken.
    let mut doc = pages("references.pdf");
    let (src, ids) = with_source(&mut doc, "pages/references.pdf");
    apply(
        &mut doc,
        InsertPagesFrom::new(src, vec![ids[2].clone(), ids[3].clone()], PagePosition::End),
    );
    let after = page_ids(&doc);
    let (three, four) = (&after[after.len() - 2], &after[after.len() - 1]);
    let link = annots(&doc, four)
        .into_iter()
        .find(|(_, d)| subtype(d) == b"Link")
        .unwrap()
        .1;
    let Some(Object::String(name)) = link.get("Dest") else {
        panic!("a named destination");
    };
    assert_eq!(name.bytes, b"chap3#2");
    let dest = doc.outline().unwrap().named_destination(b"chap3#2").unwrap().unwrap();
    assert_eq!(dest.page.as_ref(), Some(three));
    // The original keeps naming the original page.
    let old = doc.outline().unwrap().named_destination(b"chap3").unwrap().unwrap();
    assert_eq!(old.page.as_ref(), Some(&after[2]));
}

#[test]
fn fields_keep_their_names_unless_taken_then_take_the_next_free_number() {
    let mut doc = pages("forms.pdf");
    let (src, ids) = with_source(&mut doc, "pages/references.pdf");
    let applied = apply(
        &mut doc,
        InsertPagesFrom::new(src, vec![ids[2].clone()], PagePosition::End),
    );
    let renamed: Vec<_> = applied
        .fields
        .iter()
        .map(|f| match &f.outcome {
            FieldOutcome::Renamed { name, .. } => name.as_str(),
            other => panic!("{other:?}"),
        })
        .collect();
    assert_eq!(renamed, ["name#2"]);
    let names = doc.field_names().unwrap();
    for name in ["name", "name#2", "agree", "city", "addr", "addr.street"] {
        assert!(names.contains(name), "{name}");
    }
}

#[test]
fn a_field_can_join_the_targets_field_of_the_same_name_and_type() {
    let mut doc = pages("references.pdf");
    let city = doc.field_by_name("city").unwrap().unwrap();
    let (src, ids) = with_source(&mut doc, "pages/references.pdf");
    let carry = Carry::default().with_fields(FieldClash::Link);
    let applied = apply(
        &mut doc,
        InsertPagesFrom::new(src, vec![ids[2].clone()], PagePosition::End).with_carry(carry),
    );
    let outcomes: Vec<_> = applied.fields.iter().map(|f| f.outcome.clone()).collect();
    // "city" has its widget as a kid: linked. "name" is merged with its widget: renamed. "addr"
    // isn't terminal: renamed.
    assert!(outcomes.contains(&FieldOutcome::Linked {
        to: FieldId::object(city)
    }));
    let kids = dict(&doc, city).get("Kids").unwrap().as_array().unwrap().len();
    assert_eq!(kids, 2);
    // The joined field's /Kids changed: reported for the planner's field-lock checks.
    assert!(applied.widgets.iter().any(|w| w.field == FieldId::object(city)));
}

#[test]
fn a_signature_widget_is_left_off_and_reported() {
    let mut doc = pages("forms.pdf");
    let (src, ids) = with_source(&mut doc, "signed/approval.pdf");
    let applied = apply(
        &mut doc,
        InsertPagesFrom::new(src, vec![ids[0].clone()], PagePosition::End),
    );
    assert!(applied.fields.iter().any(|f| f.outcome == FieldOutcome::NotCopied));
    let copy = page_ids(&doc).last().unwrap().clone();
    assert!(
        annots(&doc, &copy)
            .iter()
            .all(|(_, d)| d.get_name("FT") != Some(b"Sig"))
    );
}

#[test]
fn tagged_pages_keep_their_tags_in_a_tagged_file() {
    let mut doc = pages("tagged.pdf");
    let (src, ids) = with_source(&mut doc, "pages/references.pdf");
    let applied = apply(
        &mut doc,
        InsertPagesFrom::new(src, vec![ids[2].clone()], PagePosition::End),
    );
    let copy = page_ids(&doc).last().unwrap().clone();
    assert_eq!(applied.tags_dropped, []);
    let key = dict(&doc, obj(&copy)).get_int("StructParents").unwrap();
    let tree = doc.structure().unwrap().unwrap();
    let on_copy = tree
        .elements()
        .iter()
        .filter(|e| e.page.as_ref() == Some(&copy))
        .count();
    assert!(on_copy > 0, "no element is on the copy");
    assert!(tree.parent_tree_next_key.unwrap() > key);
}

#[test]
fn tagged_pages_lose_their_tags_in_an_untagged_file() {
    let mut doc = pages("forms.pdf");
    let (src, ids) = with_source(&mut doc, "pages/references.pdf");
    let applied = apply(
        &mut doc,
        InsertPagesFrom::new(src, vec![ids[2].clone()], PagePosition::End),
    );
    let copy = page_ids(&doc).last().unwrap().clone();
    assert_eq!(applied.tags_dropped, vec![copy.clone()]);
    assert!(dict(&doc, obj(&copy)).get("StructParents").is_none());
    assert!(doc.structure().unwrap().is_none());
}

#[test]
fn undo_leaves_the_overlay_as_it_was_and_redo_puts_the_copies_back() {
    let mut doc = pages("forms.pdf");
    let (src, ids) = with_source(&mut doc, "pages/references.pdf");
    let empty = snapshot(&doc);
    apply(&mut doc, InsertPagesFrom::new(src, ids, PagePosition::Start));
    let applied = snapshot(&doc);
    doc.undo().unwrap();
    assert_eq!(snapshot(&doc), empty);
    doc.redo().unwrap();
    assert_eq!(snapshot(&doc), applied);
}

#[test]
fn an_unknown_source_or_source_page_is_refused_with_nothing_changed() {
    let mut doc = pages("forms.pdf");
    let missing: SourceId = "f534f6f1a920e560dfd6103ae24af60e318d129a177a038366135afc943d058d"
        .parse()
        .unwrap();
    let first = page_ids(&doc)[0].clone();
    let err = doc
        .apply_all(
            Origin::Local,
            [InsertPagesFrom::new(missing, vec![first], PagePosition::End).into()],
        )
        .unwrap_err();
    assert_eq!(err, Error::UnknownSource(missing));
    let (src, _) = with_source(&mut doc, "pages/references.pdf");
    let stranger = PageId::object(ObjRef::new(9999, 0));
    let err = doc
        .apply_all(
            Origin::Local,
            [InsertPagesFrom::new(src, vec![stranger.clone()], PagePosition::End).into()],
        )
        .unwrap_err();
    assert_eq!(err, Error::UnknownPage(stranger));
    assert!(!doc.is_dirty());
}

#[test]
fn a_replaced_page_hands_what_pointed_at_it_to_the_page_taking_its_place() {
    let mut doc = pages("references.pdf");
    let before = page_ids(&doc);
    let (src, ids) = with_source(&mut doc, "pages/forms.pdf");
    let applied = apply(
        &mut doc,
        ReplacePages::new(vec![before[2].clone()], src, vec![ids[0].clone()]),
    );
    let after = page_ids(&doc);
    assert_eq!(after.len(), before.len());
    let copy = &after[2];
    assert!(!before.contains(copy));
    let outline = doc.outline().unwrap();
    let first = outline.roots().next().unwrap();
    assert_eq!(first.target().unwrap().and_then(|d| d.page).as_ref(), Some(copy));
    assert_eq!(
        outline.named_destination(b"chap3").unwrap().unwrap().page.as_ref(),
        Some(copy)
    );
    assert!(applied.references.iter().all(|r| match &r.outcome {
        ReferenceOutcome::Retargeted { to } => to == copy,
        ReferenceOutcome::Removed => true,
        _ => false,
    }));
    assert!(applied.still_in_earlier_revisions);
}

#[test]
fn targets_left_over_are_deleted_and_source_pages_left_over_follow_the_last_copy() {
    let mut doc = pages("references.pdf");
    let before = page_ids(&doc);
    let (src, ids) = with_source(&mut doc, "pages/references.pdf");
    // Two targets, one page: page 2 goes.
    apply(
        &mut doc,
        ReplacePages::new(vec![before[0].clone(), before[1].clone()], src, vec![ids[4].clone()]),
    );
    assert_eq!(page_ids(&doc).len(), 4);
    // One target, three pages: two extra after the first.
    let now = page_ids(&doc);
    apply(
        &mut doc,
        ReplacePages::new(vec![now[0].clone()], src, ids[..3].to_vec()),
    );
    let after = page_ids(&doc);
    assert_eq!(after.len(), 6);
    assert_eq!(&after[3..], &now[1..]);
}

#[test]
fn replacing_every_page_with_nothing_is_refused() {
    let mut doc = pages("forms.pdf");
    let all = page_ids(&doc);
    let (src, _) = with_source(&mut doc, "pages/references.pdf");
    let err = doc
        .apply_all(Origin::Local, [ReplacePages::new(all, src, Vec::new()).into()])
        .unwrap_err();
    assert_eq!(err, Error::NoPagesLeft);
}

#[test]
fn an_extracted_document_holds_the_pages_their_labels_and_unsigned_signature_fields() {
    let doc = pages("labels.pdf");
    let ids = page_ids(&doc);
    let labels = doc.page_labels().unwrap();
    let out = doc.extract_pages(&[ids[1].clone(), ids[4].clone()]).unwrap();
    assert_eq!(out.page_count().unwrap(), 2);
    let new_labels = out.page_labels().unwrap();
    assert_eq!(
        (new_labels.label(0), new_labels.label(1)),
        (labels.label(1), labels.label(4))
    );
    let signed = open(&fixture("signed/approval.pdf"));
    let first = page_ids(&signed)[0].clone();
    let out = signed.extract_pages(&[first]).unwrap();
    let page = page_ids(&out)[0].clone();
    let sig = annots(&out, &page)
        .into_iter()
        .find(|(_, d)| d.get_name("FT") == Some(b"Sig") || d.get("Parent").is_some())
        .map(|(r, _)| r);
    if let Some(widget) = sig {
        let field = dict(&out, widget)
            .get("Parent")
            .and_then(Object::as_ref)
            .unwrap_or(widget);
        assert!(dict(&out, field).get("V").is_none(), "unsigned");
    }
}

#[test]
fn export_ranges_makes_one_document_per_range() {
    let doc = pages("references.pdf");
    let ids = page_ids(&doc);
    let parts = doc.export_ranges(&[ids[..2].to_vec(), ids[2..].to_vec()]).unwrap();
    let counts: Vec<usize> = parts.iter().map(|d| d.page_count().unwrap()).collect();
    assert_eq!(counts, [2, 3]);
}

#[test]
fn undoing_a_replacement_brings_the_old_page_and_what_pointed_at_it_back() {
    let mut doc = pages("references.pdf");
    let before = page_ids(&doc);
    let (src, ids) = with_source(&mut doc, "pages/forms.pdf");
    let empty = snapshot(&doc);
    apply(
        &mut doc,
        ReplacePages::new(vec![before[2].clone()], src, vec![ids[0].clone()]),
    );
    let replaced = snapshot(&doc);
    doc.undo().unwrap();
    assert_eq!((snapshot(&doc), page_ids(&doc)), (empty, before));
    doc.redo().unwrap();
    assert_eq!(snapshot(&doc), replaced);
}

#[test]
fn pages_from_an_encrypted_source_are_written_as_plain_objects() {
    use laminapdf_save::{Save as _, SaveOptions};
    let mut doc = pages("forms.pdf");
    let id = doc
        .add_source(fixture("encrypted/r4-aes-128-user.pdf"), Some("user-pw"))
        .unwrap();
    let source = doc.source_document(&id).unwrap();
    let first = page_ids(source)[0].clone();
    let source_cos: &laminapdf_cos::Document = source;
    let want = source_cos.page_content(&source_cos.pages().unwrap()[0]).unwrap();
    apply(&mut doc, InsertPagesFrom::new(id, vec![first], PagePosition::End));
    let mut out = Vec::new();
    doc.save(&mut out, &SaveOptions::new()).unwrap();
    // No password: the copy was written as plain objects into an unencrypted file.
    let reopened = laminapdf_model::Document::open(out, laminapdf_cos::OpenOptions::default()).unwrap();
    assert!(!reopened.is_encrypted());
    let pages = reopened.pages().unwrap();
    let cos: &laminapdf_cos::Document = &reopened;
    let copy = cos.pages().unwrap();
    assert_eq!(cos.page_content(copy.last().unwrap()).unwrap(), want);
    assert_eq!(pages.len(), 3);
}
