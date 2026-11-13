//! `AddAnnot` with text markup, notes and carets, and `UpdateAnnot`'s changes that draw them
//! again: what each writes, its appearance, its Popup and group, its place in the structure
//! tree, and its refusals.
#![allow(
    clippy::unwrap_used,
    clippy::indexing_slicing,
    reason = "tests: a panic is a test failure"
)]

mod common;

use common::{fixture, open};
use laminapdf_build::AnnotationAppearances;
use laminapdf_cos::{Dict, ObjRef, Object};
use laminapdf_model::{AnnotId, PageId, UserSpacePoint, UserSpaceRect};
use laminapdf_ops::{
    AddAnnot, AnnotColor, AnnotCommon, AnnotSpec, AnnotSpecPatch, Applied, Caret, CaretPatch, DeleteAnnot, Document,
    MarkupKind, ModificationClass, Note, NoteIcon, Op, Opacity, Origin, TextMarkup, UpdateAnnot,
};

fn doc(name: &str) -> Document {
    open(&fixture(name)).with_appearances(AnnotationAppearances)
}

fn apply(doc: &mut Document, op: impl Into<Op>) -> Applied {
    doc.apply_all(Origin::Local, [op.into()]).unwrap()
}

fn refused(doc: &mut Document, op: impl Into<Op>) -> &'static str {
    doc.apply_all(Origin::Local, [op.into()]).unwrap_err().code()
}

fn page(doc: &Document, index: usize) -> PageId {
    doc.pages().unwrap().get(index).unwrap().id().clone()
}

fn dict(doc: &Document, r: ObjRef) -> Dict {
    doc.resolve(r).unwrap().as_dict().unwrap().clone()
}

fn obj(id: &AnnotId) -> ObjRef {
    id.as_lamina_id().obj_ref().unwrap()
}

fn p(x: f64, y: f64) -> UserSpacePoint {
    UserSpacePoint::new(x, y)
}

/// One line of text, 108 points wide and 14 high.
fn line() -> [UserSpacePoint; 4] {
    [p(72.0, 714.0), p(180.0, 714.0), p(72.0, 700.0), p(180.0, 700.0)]
}

fn markup(kind: MarkupKind) -> AnnotSpec {
    AnnotSpec::TextMarkup(TextMarkup::new(kind, vec![line()]))
}

fn add(doc: &mut Document, spec: AnnotSpec) -> AnnotId {
    let page = page(doc, 0);
    apply(doc, AddAnnot::new(page, spec)).created_annotations.pop().unwrap()
}

/// The annotation's normal appearance.
fn appearance(doc: &Document, annot: &Dict) -> Dict {
    let ap = annot.get("AP").and_then(Object::as_dict).unwrap();
    dict(doc, ap.get("N").and_then(Object::as_ref).unwrap())
}

/// The appearance's ExtGState `/GS0`.
fn extgstate(doc: &Document, annot: &Dict) -> Option<Dict> {
    let form = appearance(doc, annot);
    let resources = form.get("Resources").and_then(Object::as_dict)?;
    resources.get("ExtGState")?.as_dict()?.get("GS0")?.as_dict().cloned()
}

/// The appearance's content stream, decoded.
fn content(doc: &Document, annot: &Dict) -> String {
    let ap = annot.get("AP").and_then(Object::as_dict).unwrap();
    let form = doc.resolve(ap.get("N").and_then(Object::as_ref).unwrap()).unwrap();
    let Object::Stream(stream) = &*form else {
        panic!("the appearance is a stream")
    };
    String::from_utf8(doc.cos().decode_stream(stream).unwrap()).unwrap()
}

fn numbers(value: Option<&Object>) -> Vec<f64> {
    value
        .and_then(Object::as_array)
        .unwrap()
        .iter()
        .map(|v| v.as_f64().unwrap())
        .collect()
}

mod add {
    use super::*;

    #[test]
    fn a_highlight_is_printed_over_its_quads_and_multiplies() {
        let mut doc = doc("golden/examples/hello.pdf");
        let id = add(&mut doc, markup(MarkupKind::Highlight));
        let annot = dict(&doc, obj(&id));
        assert_eq!(
            (
                annot.get_name("Subtype"),
                annot.get_int("F"),
                numbers(annot.get("Rect")),
                numbers(annot.get("QuadPoints")),
                extgstate(&doc, &annot).and_then(|gs| gs.get_name("BM").map(<[u8]>::to_vec)),
            ),
            (
                Some(b"Highlight".as_slice()),
                Some(4),
                vec![72.0, 700.0, 180.0, 714.0],
                vec![72.0, 714.0, 180.0, 714.0, 72.0, 700.0, 180.0, 700.0],
                Some(b"Multiply".to_vec()),
            )
        );
    }

    #[test]
    fn quads_given_in_the_standards_order_are_written_in_acrobats() {
        let mut doc = doc("golden/examples/hello.pdf");
        let standard = [p(72.0, 700.0), p(180.0, 700.0), p(180.0, 714.0), p(72.0, 714.0)];
        let id = add(
            &mut doc,
            AnnotSpec::TextMarkup(TextMarkup::new(MarkupKind::Underline, vec![standard])),
        );
        assert_eq!(
            numbers(dict(&doc, obj(&id)).get("QuadPoints")),
            vec![72.0, 714.0, 180.0, 714.0, 72.0, 700.0, 180.0, 700.0]
        );
    }

    #[test]
    fn the_appearance_maps_onto_the_rect() {
        let mut doc = doc("golden/examples/hello.pdf");
        let id = add(&mut doc, markup(MarkupKind::StrikeOut));
        let annot = dict(&doc, obj(&id));
        let form = appearance(&doc, &annot);
        assert_eq!(
            (numbers(form.get("BBox")), form.get("Matrix")),
            (numbers(annot.get("Rect")), None)
        );
    }

    #[test]
    fn a_translucent_mark_writes_ca_and_its_appearance_applies_it() {
        let mut doc = doc("golden/examples/hello.pdf");
        let mut common = AnnotCommon::default();
        common.opacity = Opacity::new(0.5).unwrap();
        common.color = Some(AnnotColor::Rgb(0.0, 0.0, 1.0));
        let page = page(&doc, 0);
        let id = apply(
            &mut doc,
            AddAnnot::new(page, markup(MarkupKind::Underline)).with_common(common),
        )
        .created_annotations
        .pop()
        .unwrap();
        let annot = dict(&doc, obj(&id));
        assert_eq!(
            (
                annot.get("CA").and_then(Object::as_f64),
                numbers(annot.get("C")),
                extgstate(&doc, &annot).and_then(|gs| gs.get("ca").and_then(Object::as_f64)),
            ),
            (Some(0.5), vec![0.0, 0.0, 1.0], Some(0.5))
        );
    }

    #[test]
    fn a_note_gets_its_popup_beside_it() {
        let mut doc = doc("golden/examples/hello.pdf");
        let id = add(
            &mut doc,
            AnnotSpec::Note(Note::new(p(100.0, 700.0), NoteIcon::Comment).with_open(true)),
        );
        let note = dict(&doc, obj(&id));
        let popup = note.get("Popup").and_then(Object::as_ref).unwrap();
        let popup_dict = dict(&doc, popup);
        let listed = doc.annotation_links(&doc.pages().unwrap().get(0).unwrap()).unwrap();
        assert_eq!(
            (
                note.get_name("Name"),
                note.get_int("F"),
                numbers(note.get("Rect")),
                popup_dict.get("Parent").and_then(Object::as_ref),
                popup_dict.get("Open"),
                listed.iter().any(|a| a.id == AnnotId::object(popup)),
            ),
            (
                Some(b"Comment".as_slice()),
                Some(28),
                vec![100.0, 680.0, 120.0, 700.0],
                Some(obj(&id)),
                Some(&Object::Bool(true)),
                true,
            )
        );
    }

    #[test]
    fn a_caret_grouped_with_a_strike_out_is_deleted_with_it() {
        let mut doc = doc("golden/examples/hello.pdf");
        let strike = add(&mut doc, markup(MarkupKind::StrikeOut));
        let mut common = AnnotCommon::default();
        common.group_with = Some(strike.clone());
        common.contents = Some("replacement".into());
        let page = page(&doc, 0);
        let caret = AnnotSpec::Caret(Caret::new(UserSpaceRect::from_corners(178.0, 696.0, 186.0, 706.0)));
        let caret = apply(&mut doc, AddAnnot::new(page, caret).with_common(common))
            .created_annotations
            .pop()
            .unwrap();
        let caret_dict = dict(&doc, obj(&caret));
        let deleted = apply(&mut doc, DeleteAnnot::new(caret.clone())).also_deleted;
        assert_eq!(
            (
                caret_dict.get("IRT").and_then(Object::as_ref),
                caret_dict.get_name("RT"),
                deleted,
            ),
            (Some(obj(&strike)), Some(b"Group".as_slice()), vec![strike])
        );
    }

    #[test]
    fn is_an_annotation_change_that_needs_no_rewrite() {
        let op = Op::from(AddAnnot::new("p2.0".parse().unwrap(), markup(MarkupKind::Highlight)));
        assert_eq!(
            (op.class(), op.requires_rewrite()),
            (ModificationClass::Annotation, false)
        );
    }

    #[test]
    fn undo_leaves_the_document_as_it_was() {
        let mut doc = doc("golden/examples/hello.pdf");
        add(&mut doc, AnnotSpec::Note(Note::new(p(100.0, 700.0), NoteIcon::Note)));
        doc.undo().unwrap();
        assert!(!doc.is_dirty());
    }
}

mod tagged {
    use super::*;

    #[test]
    fn a_new_annotation_joins_the_structure_tree_as_an_annot_element() {
        let mut doc = doc("golden/examples/pdf_ua_report.pdf");
        let id = add(&mut doc, markup(MarkupKind::Highlight));
        let annot = dict(&doc, obj(&id));
        let key = annot.get_int("StructParent").unwrap();
        let tree = doc.catalog().unwrap();
        let root = dict(&doc, tree.get("StructTreeRoot").and_then(Object::as_ref).unwrap());
        let entries = doc
            .number_tree(root.get("ParentTree").cloned().unwrap())
            .entries()
            .unwrap();
        let elem = entries
            .iter()
            .find(|(k, _)| *k == key)
            .and_then(|(_, e)| e.as_ref())
            .unwrap();
        let elem = dict(&doc, elem);
        let page = dict(&doc, page(&doc, 0).as_lamina_id().obj_ref().unwrap());
        assert_eq!(
            (
                elem.get_name("S"),
                elem.get("K")
                    .and_then(Object::as_dict)
                    .and_then(|k| k.get("Obj"))
                    .and_then(Object::as_ref),
                page.get_name("Tabs"),
            ),
            (Some(b"Annot".as_slice()), Some(obj(&id)), Some(b"S".as_slice()))
        );
    }
}

mod pdfa {
    use super::*;

    #[test]
    fn an_appearance_in_a_pdfa_file_paints_in_its_intents_space() {
        let mut doc = doc("golden/examples/pdf_a_archive-2b.pdf");
        let mut common = AnnotCommon::default();
        common.color = Some(AnnotColor::Gray(0.5));
        let page = page(&doc, 0);
        let id = apply(
            &mut doc,
            AddAnnot::new(page, markup(MarkupKind::Underline)).with_common(common),
        )
        .created_annotations
        .pop()
        .unwrap();
        let text = content(&doc, &dict(&doc, obj(&id)));
        assert!(text.contains("0.5 0.5 0.5 RG"), "{text}");
    }
}

mod update {
    use super::*;

    #[test]
    fn a_new_colour_draws_the_appearance_again() {
        let mut doc = doc("golden/examples/hello.pdf");
        let id = add(&mut doc, markup(MarkupKind::Highlight));
        let before = dict(&doc, obj(&id)).get("AP").cloned();
        let mut update = UpdateAnnot::new(id.clone());
        update.color = Some(Some(AnnotColor::Rgb(0.0, 1.0, 0.0)));
        apply(&mut doc, update);
        let annot = dict(&doc, obj(&id));
        assert_eq!(
            (annot.get("AP") != before.as_ref(), numbers(annot.get("C"))),
            (true, vec![0.0, 1.0, 0.0])
        );
    }

    #[test]
    fn a_caret_patch_moves_the_caret() {
        let mut doc = doc("golden/examples/hello.pdf");
        let id = add(
            &mut doc,
            AnnotSpec::Caret(Caret::new(UserSpaceRect::from_corners(10.0, 10.0, 18.0, 20.0))),
        );
        let mut update = UpdateAnnot::new(id.clone());
        let mut patch = CaretPatch::default();
        patch.rect = Some(UserSpaceRect::from_corners(30.0, 10.0, 38.0, 20.0));
        update.spec = Some(AnnotSpecPatch::Caret(patch));
        apply(&mut doc, update);
        assert_eq!(numbers(dict(&doc, obj(&id)).get("Rect")), vec![30.0, 10.0, 38.0, 20.0]);
    }

    #[test]
    fn a_patch_for_another_kind_of_mark_is_refused() {
        let mut doc = doc("golden/examples/hello.pdf");
        let id = add(&mut doc, markup(MarkupKind::Highlight));
        let mut update = UpdateAnnot::new(id);
        update.spec = Some(AnnotSpecPatch::Caret(CaretPatch::default()));
        assert_eq!(refused(&mut doc, update), "invalid-op");
    }
}

mod refusals {
    use super::*;

    #[test]
    fn without_a_generator_no_annotation_is_made() {
        let mut doc = open(&fixture("golden/examples/hello.pdf"));
        let page = page(&doc, 0);
        assert_eq!(
            refused(&mut doc, AddAnnot::new(page, markup(MarkupKind::Highlight))),
            "appearance-unavailable"
        );
    }

    #[test]
    fn markup_without_quads_is_invalid() {
        let mut doc = doc("golden/examples/hello.pdf");
        let page = page(&doc, 0);
        let empty = AnnotSpec::TextMarkup(TextMarkup::new(MarkupKind::Highlight, Vec::new()));
        assert_eq!(refused(&mut doc, AddAnnot::new(page, empty)), "invalid-op");
    }

    #[test]
    fn a_point_that_isnt_finite_is_invalid() {
        let mut doc = doc("golden/examples/hello.pdf");
        let page = page(&doc, 0);
        let note = AnnotSpec::Note(Note::new(p(f64::NAN, 700.0), NoteIcon::Note));
        assert_eq!(refused(&mut doc, AddAnnot::new(page, note)), "invalid-op");
    }
}
