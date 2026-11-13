//! `UpdateAnnot` and `DeleteAnnot` over `fixtures/annotations/`: what each writes, what the
//! flags refuse, the cascade through Popups, replies and groups, the structure tree, and undo.
#![expect(clippy::unwrap_used, reason = "tests: a panic is a test failure")]

mod common;

use common::{fixture, open, snapshot};
use laminapdf_cos::{Dict, Object};
use laminapdf_model::{AnnotId, PdfDate};
use laminapdf_ops::{DeleteAnnot, Document, Error, ModificationClass, Offset, Op, Origin, UpdateAnnot};

fn doc(name: &str) -> Document {
    open(&fixture(&format!("annotations/{name}")))
}

fn id(text: &str) -> AnnotId {
    text.parse().unwrap()
}

fn object(doc: &Document, num: u32) -> Option<Dict> {
    let r = laminapdf_cos::ObjRef { num, gen_nr: 0 };
    laminapdf_cos::tolerant(doc.resolve(r))
        .unwrap()
        .as_deref()
        .and_then(Object::as_dict)
        .cloned()
}

fn numbers(dict: &Dict, key: &str) -> Vec<f64> {
    dict.get(key)
        .and_then(Object::as_array)
        .unwrap()
        .iter()
        .map(|o| o.as_f64().unwrap())
        .collect()
}

fn text(dict: &Dict, key: &str) -> Option<String> {
    match dict.get(key) {
        Some(Object::String(s)) => Some(laminapdf_syntax::text::decode_text_string(&s.bytes)),
        _ => None,
    }
}

fn apply(doc: &mut Document, op: impl Into<Op>) -> Result<laminapdf_ops::Applied, Error> {
    doc.apply_all(Origin::Local, [op.into()])
}

/// The object numbers a page's `/Annots` lists now.
fn listed(doc: &Document, index: usize) -> Vec<u32> {
    let pages = doc.pages().unwrap();
    pages
        .get(index)
        .unwrap()
        .annots()
        .unwrap()
        .iter()
        .map(|r| r.num)
        .collect()
}

mod update {
    use super::*;

    #[test]
    fn sets_and_removes_text_entries_and_stamps_the_date() {
        let mut d = doc("every-subtype.pdf");
        let mut op = UpdateAnnot::new(id("a10.0"));
        op.contents = Some(Some("Résumé".to_owned()));
        op.author = Some(None);
        op.modified = Some("D:20261113120000Z".parse::<PdfDate>().unwrap());
        apply(&mut d, op).unwrap();
        let dict = object(&d, 10).unwrap();
        assert_eq!(
            (
                text(&dict, "Contents").as_deref(),
                dict.contains_key("T"),
                text(&dict, "M").as_deref()
            ),
            (Some("Résumé"), false, Some("D:20261113120000Z"))
        );
    }

    #[test]
    fn writes_nothing_when_nothing_changes() {
        let mut d = doc("every-subtype.pdf");
        let mut op = UpdateAnnot::new(id("a10.0"));
        op.author = Some(Some("Ada".to_owned()));
        op.modified = Some("D:20261113120000Z".parse::<PdfDate>().unwrap());
        let applied = apply(&mut d, op).unwrap();
        assert_eq!((applied.touched_pages.len(), d.overlay().len()), (0, 0));
    }

    #[test]
    fn translate_moves_the_rect_and_the_geometry_and_keeps_the_appearance() {
        let mut d = doc("every-subtype.pdf");
        for num in [10, 14] {
            let mut op = UpdateAnnot::new(AnnotId::object(laminapdf_cos::ObjRef { num, gen_nr: 0 }));
            op.translate = Some(Offset::new(5.0, -10.0));
            apply(&mut d, op).unwrap();
        }
        let highlight = object(&d, 10).unwrap();
        let ink = object(&d, 14).unwrap();
        let first_stroke = ink.get("InkList").and_then(Object::as_array).unwrap()[0].clone();
        assert_eq!(
            (
                numbers(&highlight, "Rect"),
                numbers(&highlight, "QuadPoints"),
                highlight.get("AP").cloned(),
                first_stroke
                    .as_array()
                    .unwrap()
                    .iter()
                    .map(|o| o.as_f64().unwrap())
                    .collect::<Vec<_>>(),
            ),
            (
                vec![25.0, 250.0, 125.0, 265.0],
                vec![25.0, 250.0, 125.0, 250.0, 125.0, 265.0, 25.0, 265.0],
                object_before("every-subtype.pdf", 10).get("AP").cloned(),
                vec![25.0, 140.0, 75.0, 190.0, 125.0, 140.0],
            )
        );
    }

    #[test]
    fn translate_moves_the_popup_with_its_parent() {
        let mut d = doc("every-subtype.pdf");
        let mut op = UpdateAnnot::new(id("a12.0"));
        op.translate = Some(Offset::new(-10.0, 0.0));
        apply(&mut d, op).unwrap();
        assert_eq!(numbers(&object(&d, 13).unwrap(), "Rect"), [170.0, 200.0, 270.0, 260.0]);
    }

    #[test]
    fn keeps_keys_it_does_not_own() {
        let mut d = doc("flags.pdf");
        let mut op = UpdateAnnot::new(id("a12.0"));
        op.contents = Some(Some("Note".to_owned()));
        op.translate = Some(Offset::new(1.0, 1.0));
        apply(&mut d, op).unwrap();
        let dict = object(&d, 12).unwrap();
        assert_eq!(
            (dict.get("LaminaTestKey").cloned(), dict.get("AAPL:Private").cloned()),
            (
                object_before("flags.pdf", 12).get("LaminaTestKey").cloned(),
                object_before("flags.pdf", 12).get("AAPL:Private").cloned()
            )
        );
    }

    #[test]
    fn a_locked_annotation_refuses_a_move_but_takes_new_flags_and_contents() {
        let mut d = doc("flags.pdf");
        let mut moved = UpdateAnnot::new(id("a10.0"));
        moved.translate = Some(Offset::new(1.0, 0.0));
        let mut unlocked = UpdateAnnot::new(id("a10.0"));
        unlocked.flags = Some(4);
        let mut contents = UpdateAnnot::new(id("a10.0"));
        contents.contents = Some(Some("New".to_owned()));
        assert_eq!(
            (
                apply(&mut d, moved).map_err(|e| e.code()).err(),
                apply(&mut d, contents).is_ok(),
                apply(&mut d, unlocked).is_ok(),
            ),
            (Some("annotation-locked"), true, true)
        );
    }

    #[test]
    fn locked_contents_refuses_new_contents() {
        let mut d = doc("flags.pdf");
        let mut op = UpdateAnnot::new(id("a11.0"));
        op.contents = Some(Some("Changed".to_owned()));
        assert_eq!(apply(&mut d, op).map_err(|e| e.code()).err(), Some("annotation-locked"));
    }

    #[test]
    fn widgets_direct_dictionaries_and_unknown_ids_are_refused() {
        let mut d = doc("every-subtype.pdf");
        let direct = d
            .annotations(&d.pages().unwrap().get(0).unwrap())
            .unwrap()
            .last()
            .unwrap()
            .id
            .clone();
        let codes: Vec<&str> = [id("a18.0"), direct, id("a99.0")]
            .into_iter()
            .map(|target| apply(&mut d, DeleteAnnot::new(target)).unwrap_err().code())
            .collect();
        assert_eq!(
            codes,
            [
                "annotation-not-editable",
                "annotation-not-editable",
                "unknown-annotation"
            ]
        );
    }
}

fn object_before(name: &str, num: u32) -> Dict {
    object(&doc(name), num).unwrap()
}

mod delete {
    use super::*;

    #[test]
    fn a_note_goes_with_its_popup_and_every_reply_on_any_page() {
        let mut d = doc("threads.pdf");
        let applied = apply(&mut d, DeleteAnnot::new(id("a10.0"))).unwrap();
        let mut also: Vec<String> = applied.also_deleted.iter().map(ToString::to_string).collect();
        also.sort();
        assert_eq!(
            (listed(&d, 0), listed(&d, 1), also, applied.still_in_earlier_revisions),
            (
                vec![14, 15, 16, 17],
                vec![],
                vec![
                    "a11.0".to_owned(),
                    "a12.0".to_owned(),
                    "a13.0".to_owned(),
                    "a20.0".to_owned()
                ],
                true
            )
        );
    }

    #[test]
    fn deleting_either_member_of_a_group_deletes_both() {
        for member in ["a16.0", "a17.0"] {
            let mut d = doc("threads.pdf");
            apply(&mut d, DeleteAnnot::new(id(member))).unwrap();
            assert_eq!(listed(&d, 0), [10, 11, 12, 13, 14, 15], "{member}");
        }
    }

    #[test]
    fn a_reply_cycle_is_followed_once() {
        let mut d = doc("threads.pdf");
        apply(&mut d, DeleteAnnot::new(id("a14.0"))).unwrap();
        assert_eq!(listed(&d, 0), [10, 11, 12, 13, 16, 17]);
    }

    #[test]
    fn deleting_a_popup_alone_unlinks_it_from_its_parent() {
        let mut d = doc("every-subtype.pdf");
        apply(&mut d, DeleteAnnot::new(id("a13.0"))).unwrap();
        assert_eq!(
            (object(&d, 12).unwrap().contains_key("Popup"), object(&d, 13)),
            (false, None)
        );
    }

    #[test]
    fn a_locked_annotation_is_refused() {
        let mut d = doc("flags.pdf");
        assert_eq!(
            apply(&mut d, DeleteAnnot::new(id("a10.0"))).map_err(|e| e.code()).err(),
            Some("annotation-locked")
        );
    }

    #[test]
    fn the_structure_tree_loses_the_annotation_and_an_element_left_empty() {
        let mut d = doc("tagged.pdf");
        apply(&mut d, DeleteAnnot::new(id("a10.0"))).unwrap();
        apply(&mut d, DeleteAnnot::new(id("a11.0"))).unwrap();
        let document = object(&d, 21).unwrap();
        let paragraph = object(&d, 23).unwrap();
        assert_eq!(
            (document.get("K").cloned(), paragraph.get("K").cloned()),
            (
                Some(Object::Array(vec![Object::Ref(laminapdf_cos::ObjRef {
                    num: 23,
                    gen_nr: 0
                })])),
                Some(Object::Array(vec![Object::Integer(0)]))
            )
        );
    }
}

#[test]
fn both_ops_are_annotation_changes_saved_incrementally() {
    let ops: [Op; 2] = [UpdateAnnot::new(id("a1.0")).into(), DeleteAnnot::new(id("a1.0")).into()];
    assert_eq!(
        ops.iter()
            .map(|op| (op.class(), op.requires_rewrite()))
            .collect::<Vec<_>>(),
        [(ModificationClass::Annotation, false); 2]
    );
}

#[test]
fn undo_leaves_the_overlay_as_it_was_and_redo_writes_the_same_again() {
    let mut d = doc("threads.pdf");
    let before = snapshot(&d);
    let mut op = UpdateAnnot::new(id("a12.0"));
    op.translate = Some(Offset::new(3.0, 3.0));
    apply(&mut d, op).unwrap();
    apply(&mut d, DeleteAnnot::new(id("a10.0"))).unwrap();
    let after = snapshot(&d);
    d.undo().unwrap();
    d.undo().unwrap();
    let undone = snapshot(&d);
    d.redo().unwrap();
    d.redo().unwrap();
    assert_eq!((undone, snapshot(&d)), (before, after));
}

#[test]
fn the_frozen_samples_apply() {
    let mut d = doc("threads.pdf");
    for name in ["update-annot", "delete-annot"] {
        let op: Op = serde_json::from_slice(&fixture(&format!("ops/v1/{name}.json"))).unwrap();
        apply(&mut d, op).unwrap();
    }
    assert_eq!(listed(&d, 0), [14, 15, 16, 17]);
}

#[test]
fn the_fuzz_entry_holds_on_every_fixture() {
    for name in ["every-subtype.pdf", "threads.pdf", "flags.pdf", "tagged.pdf"] {
        common::annot_fuzz_entry::annots(&fixture(&format!("annotations/{name}")));
    }
}
