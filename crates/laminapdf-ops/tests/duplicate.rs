//! `DuplicatePages` (E2 plan §2.2) over `fixtures/pages/`: what a copy shares with its source
//! and what it gets anew (annotations, fields named `<partial>#<n>`, structure keys and cloned
//! elements), the labels, the PDF/UA warning, and undo.
#![allow(
    clippy::unwrap_used,
    clippy::indexing_slicing,
    reason = "tests: a panic is a test failure"
)]

mod common;

use common::{fixture, open, page_ids, rotations, snapshot};
use laminapdf_cos::{ObjRef, Object};
use laminapdf_model::{FieldId, PageId, PagePosition, Rotation};
use laminapdf_ops::{
    ConformanceWarning, Document, DuplicatePages, Error, FieldOutcome, Origin, pending_conformance_warnings,
};

fn pages(name: &str) -> Document {
    open(&fixture(&format!("pages/{name}")))
}

fn duplicate(doc: &mut Document, pages: &[PageId], at: PagePosition) -> laminapdf_ops::Applied {
    doc.apply_all(Origin::Local, [DuplicatePages::new(pages.to_vec(), at).into()])
        .unwrap()
}

fn obj(id: &PageId) -> ObjRef {
    id.as_lamina_id().obj_ref().unwrap()
}

/// Object `r`'s dictionary.
fn dict(doc: &Document, r: ObjRef) -> laminapdf_cos::Dict {
    doc.resolve(r).unwrap().as_dict().unwrap().clone()
}

/// The value at `key` of object `r`, one reference followed.
fn entry(doc: &Document, r: ObjRef, key: &str) -> Option<Object> {
    let value = dict(doc, r).get(key)?.clone();
    Some(match value {
        Object::Ref(v) => (*doc.resolve(v).unwrap()).clone(),
        other => other,
    })
}

fn refs(value: Option<Object>) -> Vec<ObjRef> {
    value
        .and_then(|v| v.as_array().map(|a| a.iter().filter_map(Object::as_ref).collect()))
        .unwrap_or_default()
}

/// Every field's fully qualified name, sorted.
fn names(doc: &Document) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for page in page_ids(doc) {
        for w in doc.page_widgets(&page).unwrap() {
            let name = doc.field_name(w.field).unwrap().unwrap();
            if !out.contains(&name) {
                out.push(name);
            }
        }
    }
    out.sort();
    out
}

#[test]
fn a_copy_lands_after_the_anchor_and_shares_its_sources_content() {
    let mut doc = pages("inherited.pdf");
    let before = page_ids(&doc);
    let applied = duplicate(&mut doc, &before[..2], PagePosition::After(before[2].clone()));
    let after = page_ids(&doc);
    let copies = &after[3..5];
    assert_eq!(
        (
            after.len(),
            &after[..3],
            &after[5..],
            applied.page_order.as_deref(),
            applied.touched_pages.iter().cloned().collect::<Vec<_>>()
        ),
        (
            before.len() + 2,
            &before[..3],
            &before[3..],
            Some(after.as_slice()),
            copies.to_vec()
        )
    );
    for (source, copy) in before.iter().zip(copies) {
        assert_eq!(
            entry(&doc, obj(copy), "Contents"),
            entry(&doc, obj(source), "Contents"),
            "the copy shares its source's content"
        );
    }
}

#[test]
fn a_copy_shows_what_its_source_inherited_wherever_it_lands() {
    let mut doc = pages("inherited.pdf");
    let before = page_ids(&doc);
    let shown = |doc: &Document, id: &PageId| {
        let pages = doc.pages().unwrap();
        let page = pages.by_id(id).unwrap();
        (
            page.media_box().unwrap(),
            page.crop_box().unwrap(),
            page.rotate().unwrap(),
        )
    };
    let wanted: Vec<_> = before.iter().map(|id| shown(&doc, id)).collect();
    duplicate(&mut doc, &before, PagePosition::Start);
    let after = page_ids(&doc);
    let got: Vec<_> = after[..before.len()].iter().map(|id| shown(&doc, id)).collect();
    assert_eq!(got, wanted);
}

/// Every page keeps its label, and a copy continues the range it lands in (E2j's remap), so a
/// copy of page "1" placed after it is "2", like the page that follows.
#[test]
fn copies_continue_the_label_range_they_land_in() {
    let mut doc = pages("labels.pdf");
    let before = page_ids(&doc);
    duplicate(&mut doc, &before[3..4], PagePosition::After(before[3].clone()));
    let labels = doc.page_labels().unwrap();
    let got: Vec<_> = (0..7).map(|i| labels.label(i)).collect();
    let want: Vec<_> = ["i", "ii", "iii", "1", "2", "2", "3"]
        .map(|s| Some(s.to_owned()))
        .to_vec();
    assert_eq!(got, want);
}

#[test]
fn a_merged_field_and_a_kid_widget_become_new_fields_with_their_own_names_and_values() {
    let mut doc = pages("forms.pdf");
    let before = page_ids(&doc);
    let applied = duplicate(&mut doc, &before[..1], PagePosition::End);
    let copy = page_ids(&doc)[2].clone();
    assert_eq!(
        names(&doc),
        ["agree", "agree#2", "name", "name#2", "signature"].map(str::to_owned)
    );
    let renamed: Vec<_> = applied
        .fields
        .iter()
        .map(|c| match &c.outcome {
            FieldOutcome::Renamed { name, .. } => (c.page.clone(), name.clone()),
            _ => panic!("{c:?}"),
        })
        .collect();
    assert_eq!(
        renamed,
        [
            (copy.clone(), "name#2".to_owned()),
            (copy.clone(), "agree#2".to_owned())
        ]
    );
    // Each copied widget belongs to its new field, and the value came with it.
    let widgets = doc.page_widgets(&copy).unwrap();
    let values: Vec<_> = widgets
        .iter()
        .map(|w| (doc.field_name(w.field).unwrap().unwrap(), entry(&doc, w.field, "V")))
        .collect();
    assert_eq!(
        values,
        [
            (
                "name#2".to_owned(),
                Some(Object::String(laminapdf_cos::PdfString::literal("Ada")))
            ),
            ("agree#2".to_owned(), Some(Object::name("Off"))),
        ]
    );
}

#[test]
fn every_new_field_is_listed_in_the_form() {
    let mut doc = pages("forms.pdf");
    let before = page_ids(&doc);
    let applied = duplicate(&mut doc, &before, PagePosition::End);
    let catalog = doc.catalog().unwrap();
    let form = match catalog.get("AcroForm").unwrap() {
        Object::Dict(d) => d.clone(),
        Object::Ref(r) => dict(&doc, *r),
        _ => panic!("no form"),
    };
    let listed: Vec<FieldId> = form
        .get("Fields")
        .and_then(Object::as_array)
        .unwrap()
        .iter()
        .filter_map(Object::as_ref)
        .map(FieldId::object)
        .collect();
    for change in &applied.fields {
        if let FieldOutcome::Renamed { to, .. } = &change.outcome {
            assert!(listed.contains(to), "{to} is not in /Fields");
        }
    }
}

#[test]
fn a_signature_widget_is_not_copied_and_is_reported() {
    let mut doc = pages("forms.pdf");
    let before = page_ids(&doc);
    let applied = duplicate(&mut doc, &before[1..2], PagePosition::End);
    let copy = page_ids(&doc)[2].clone();
    let not_copied: Vec<_> = applied
        .fields
        .iter()
        .filter(|c| c.outcome == FieldOutcome::NotCopied)
        .map(|c| {
            doc.field_name(c.field.as_lamina_id().obj_ref().unwrap())
                .unwrap()
                .unwrap()
        })
        .collect();
    let on_copy: Vec<_> = doc
        .page_widgets(&copy)
        .unwrap()
        .iter()
        .map(|w| doc.field_name(w.field).unwrap().unwrap())
        .collect();
    assert_eq!(
        (not_copied, on_copy),
        (vec!["signature".to_owned()], vec!["agree#2".to_owned()])
    );
}

#[test]
fn a_copied_annotation_is_a_new_object_on_the_copy() {
    let mut doc = pages("references.pdf");
    let before = page_ids(&doc);
    duplicate(&mut doc, &before[3..4], PagePosition::End);
    let copy = page_ids(&doc)[5].clone();
    let source_annots = refs(entry(&doc, obj(&before[3]), "Annots"));
    let copy_annots = refs(entry(&doc, obj(&copy), "Annots"));
    assert_eq!(copy_annots.len(), source_annots.len());
    for a in &copy_annots {
        assert!(!source_annots.contains(a));
        assert_eq!(entry(&doc, *a, "P"), Some((*doc.resolve(obj(&copy)).unwrap()).clone()));
    }
}

#[test]
fn a_tagged_copy_gets_its_own_key_and_clones_of_the_elements_on_its_page_alone() {
    let mut doc = pages("tagged.pdf");
    let before = page_ids(&doc);
    let applied = duplicate(&mut doc, &before[..1], PagePosition::End);
    let copy = page_ids(&doc)[2].clone();
    let key = dict(&doc, obj(&copy)).get_int("StructParents").unwrap();
    let root = doc
        .catalog()
        .unwrap()
        .get("StructTreeRoot")
        .and_then(Object::as_ref)
        .unwrap();
    let tree = dict(&doc, root).get("ParentTree").cloned().unwrap();
    let entries: std::collections::BTreeMap<i64, Object> =
        doc.number_tree(tree).entries().unwrap().into_iter().collect();
    let owners = entries[&key].as_array().unwrap().to_vec();
    let kinds: Vec<Option<String>> = owners
        .iter()
        .map(|o| {
            o.as_ref().map(|e| {
                let d = dict(&doc, e);
                assert_eq!(d.get("Pg").and_then(Object::as_ref), Some(obj(&copy)));
                String::from_utf8(d.get_name("S").unwrap().to_vec()).unwrap()
            })
        })
        .collect();
    assert_eq!(
        (key, kinds, dict(&doc, root).get_int("ParentTreeNextKey")),
        (
            4,
            vec![Some("H1".into()), Some("P".into()), None, Some("Link".into())],
            Some(7)
        )
    );
    // The paragraph that runs on to page 2 can't be split: the copy is reported.
    assert_eq!(applied.tags_incomplete, vec![copy]);
}

#[test]
fn a_tagged_annotation_on_the_copy_points_at_the_clone_of_its_element() {
    let mut doc = pages("tagged.pdf");
    let before = page_ids(&doc);
    duplicate(&mut doc, &before[..1], PagePosition::End);
    let copy = page_ids(&doc)[2].clone();
    let root = doc
        .catalog()
        .unwrap()
        .get("StructTreeRoot")
        .and_then(Object::as_ref)
        .unwrap();
    let tree = dict(&doc, root).get("ParentTree").cloned().unwrap();
    let entries: std::collections::BTreeMap<i64, Object> =
        doc.number_tree(tree).entries().unwrap().into_iter().collect();
    for annot in refs(entry(&doc, obj(&copy), "Annots")) {
        let key = dict(&doc, annot).get_int("StructParent").unwrap();
        let elem = entries[&key].as_ref().unwrap();
        let kids = dict(&doc, elem).get("K").cloned().unwrap();
        let objr: Vec<ObjRef> = match kids {
            Object::Array(items) => items,
            single => vec![single],
        }
        .into_iter()
        .filter_map(|k| k.as_dict().and_then(|d| d.get("Obj")).and_then(Object::as_ref))
        .collect();
        assert_eq!(objr, vec![annot], "the clone's OBJR is the copied annotation");
    }
}

#[test]
fn incomplete_tags_on_a_pdf_ua_file_raise_a_conformance_warning() {
    let mut doc = pages("tagged.pdf");
    let before = page_ids(&doc);
    let clean = pending_conformance_warnings(&doc, doc.pending_changes()).unwrap();
    duplicate(&mut doc, &before[..1], PagePosition::End);
    let after = pending_conformance_warnings(&doc, doc.pending_changes()).unwrap();
    assert_eq!(
        (clean, after),
        (Vec::new(), vec![ConformanceWarning::DuplicateTagsIncomplete])
    );
}

#[test]
fn a_page_whose_elements_end_on_it_is_tagged_whole() {
    let mut doc = pages("tagged.pdf");
    let before = page_ids(&doc);
    let applied = duplicate(&mut doc, &before[1..2], PagePosition::End);
    // Page 2's only content belongs to the paragraph that starts on page 1.
    assert_eq!(applied.tags_incomplete.len(), 1);
    let mut doc = pages("references.pdf");
    let before = page_ids(&doc);
    let applied = duplicate(&mut doc, &before[..1], PagePosition::End);
    assert!(applied.tags_incomplete.is_empty(), "page 1 holds no tagged content");
}

#[test]
fn undoing_a_duplication_empties_the_overlay() {
    let mut doc = pages("references.pdf");
    let before = page_ids(&doc);
    let clean = snapshot(&doc);
    duplicate(&mut doc, &before[2..4], PagePosition::Start);
    doc.undo().unwrap().unwrap();
    assert_eq!((snapshot(&doc), page_ids(&doc)), (clean, before));
}

#[test]
fn an_unknown_page_is_refused_and_changes_nothing() {
    let mut doc = pages("forms.pdf");
    let clean = snapshot(&doc);
    let err = doc
        .apply_all(
            Origin::Local,
            [DuplicatePages::new(vec!["p999.0".parse().unwrap()], PagePosition::End).into()],
        )
        .unwrap_err();
    assert!(matches!(err, Error::UnknownPage(_)), "{err:?}");
    assert_eq!(snapshot(&doc), clean);
}

#[test]
fn rotation_comes_with_the_copy() {
    let mut doc = pages("boxes.pdf");
    let before = page_ids(&doc);
    duplicate(&mut doc, &before, PagePosition::End);
    let r = rotations(&doc);
    assert_eq!(r[4..], r[..4]);
    assert_eq!(r[1], Rotation::Clockwise90);
}
