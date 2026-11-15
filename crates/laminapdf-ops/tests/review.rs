//! `AddReply`, `SetReviewState` and `delete_annotations` over `fixtures/annotations/`: what each
//! writes, what reads back as a thread, what they refuse, undo, and the frozen samples.
#![expect(clippy::unwrap_used, clippy::panic, reason = "tests: a panic is a test failure")]

mod common;

use common::{fixture, open, snapshot};
use laminapdf_cos::{Dict, Object};
use laminapdf_model::{AnnotFlags, AnnotId, PdfDate, StateModel};
use laminapdf_ops::{
    AddReply, DeleteAnnot, Document, Error, ModificationClass, Op, Origin, ReviewState, SetReviewState,
    delete_annotations,
};

fn doc(name: &str) -> Document {
    open(&fixture(&format!("annotations/{name}")))
}

fn id(text: &str) -> AnnotId {
    text.parse().unwrap()
}

fn apply(doc: &mut Document, op: impl Into<Op>) -> Result<laminapdf_ops::Applied, Error> {
    doc.apply_all(Origin::Local, [op.into()])
}

fn dict(doc: &Document, id: &AnnotId) -> Dict {
    let r = id.as_lamina_id().obj_ref().unwrap();
    laminapdf_cos::tolerant(doc.resolve(r))
        .unwrap()
        .as_deref()
        .and_then(Object::as_dict)
        .cloned()
        .unwrap()
}

fn text(dict: &Dict, key: &str) -> Option<String> {
    match dict.get(key) {
        Some(Object::String(s)) => Some(laminapdf_syntax::text::decode_text_string(&s.bytes)),
        _ => None,
    }
}

fn reply(d: &mut Document, parent: &str, contents: &str) -> AnnotId {
    let mut op = AddReply::new(id(parent), contents);
    op.author = Some("Ben".to_owned());
    op.created = Some("D:20261113120000Z".parse::<PdfDate>().unwrap());
    apply(d, op).unwrap().created_annotations.remove(0)
}

fn state(d: &mut Document, target: &str, state: ReviewState, at: &str) -> AnnotId {
    let mut op = SetReviewState::new(id(target), state);
    op.author = Some("Ben".to_owned());
    op.created = Some(at.parse::<PdfDate>().unwrap());
    apply(d, op).unwrap().created_annotations.remove(0)
}

mod add_reply {
    use super::*;

    #[test]
    fn writes_a_text_annotation_that_replies_to_its_parent() {
        let mut d = doc("threads.pdf");
        let new = reply(&mut d, "a10.0", "Looks good");
        let written = dict(&d, &new);
        assert_eq!(
            (
                written.get_name("Subtype"),
                written.get("IRT").and_then(Object::as_ref).map(|r| r.num),
                written.get_name("RT"),
                text(&written, "Contents").as_deref(),
                text(&written, "T").as_deref(),
            ),
            (
                Some(b"Text".as_slice()),
                Some(10),
                Some(b"R".as_slice()),
                Some("Looks good"),
                Some("Ben")
            )
        );
    }

    #[test]
    fn takes_its_parent_s_rect_and_draws_nothing_over_it() {
        let mut d = doc("threads.pdf");
        let new = reply(&mut d, "a10.0", "Looks good");
        let written = dict(&d, &new);
        let normal = written
            .get("AP")
            .and_then(Object::as_dict)
            .and_then(|ap| ap.get("N"))
            .and_then(Object::as_ref)
            .unwrap();
        let stream = laminapdf_cos::tolerant(d.resolve(normal)).unwrap().unwrap();
        let Object::Stream(stream) = &*stream else {
            panic!("not a stream")
        };
        assert_eq!(
            (written.get("Rect"), stream.data.is_empty()),
            (dict(&d, &id("a10.0")).get("Rect"), true)
        );
    }

    #[test]
    fn leaves_every_hiding_flag_clear_and_prints() {
        let mut d = doc("threads.pdf");
        let new = reply(&mut d, "a10.0", "Looks good");
        let flags = AnnotFlags(u32::try_from(dict(&d, &new).get_int("F").unwrap()).unwrap());
        let hiding = [AnnotFlags::INVISIBLE, AnnotFlags::HIDDEN, AnnotFlags::NO_VIEW];
        assert_eq!(
            (flags.has(AnnotFlags::PRINT), hiding.iter().any(|f| flags.has(*f))),
            (true, false)
        );
    }

    #[test]
    fn is_listed_on_its_parent_s_page_and_reads_back_in_its_thread() {
        let mut d = doc("threads.pdf");
        let new = reply(&mut d, "a12.0", "And me");
        let threads = d.comment_threads().unwrap();
        let thread = threads.iter().find(|t| t.comment.id == id("a10.0")).unwrap();
        assert!(thread.replies.iter().any(|a| a.id == new));
    }

    #[test]
    fn writes_the_date_to_both_date_entries() {
        let mut d = doc("threads.pdf");
        let new = reply(&mut d, "a10.0", "Looks good");
        let written = dict(&d, &new);
        assert_eq!(
            (text(&written, "CreationDate"), text(&written, "M")),
            (
                Some("D:20261113120000Z".to_owned()),
                Some("D:20261113120000Z".to_owned())
            )
        );
    }

    #[test]
    fn refuses_a_parent_that_is_not_markup() {
        let mut d = doc("threads.pdf");
        let refused = apply(&mut d, AddReply::new(id("a11.0"), "To a Popup")).map_err(|e| e.code());
        assert_eq!(refused.err(), Some("invalid-op"));
    }

    #[test]
    fn refuses_an_unknown_parent() {
        let mut d = doc("threads.pdf");
        let refused = apply(&mut d, AddReply::new(id("a99.0"), "Nobody")).map_err(|e| e.code());
        assert_eq!(refused.err(), Some("unknown-annotation"));
    }
}

mod set_review_state {
    use super::*;

    #[test]
    fn writes_the_model_and_the_state_as_text() {
        let mut d = doc("threads.pdf");
        let new = state(&mut d, "a10.0", ReviewState::Rejected, "D:20261113120000Z");
        let written = dict(&d, &new);
        assert_eq!(
            (
                text(&written, "StateModel"),
                text(&written, "State"),
                written.get("RT"),
                text(&written, "Contents")
            ),
            (
                Some("Review".to_owned()),
                Some("Rejected".to_owned()),
                None,
                Some("Rejected set by Ben".to_owned())
            )
        );
    }

    #[test]
    fn a_check_mark_is_in_the_marked_model() {
        let mut d = doc("threads.pdf");
        let new = state(&mut d, "a10.0", ReviewState::Marked, "D:20261113120000Z");
        assert_eq!(text(&dict(&d, &new), "StateModel").as_deref(), Some("Marked"));
    }

    #[test]
    fn the_newest_state_is_the_comment_s_status() {
        let mut d = doc("threads.pdf");
        state(&mut d, "a10.0", ReviewState::Accepted, "D:20261113120000Z");
        state(&mut d, "a10.0", ReviewState::Completed, "D:20261113130000Z");
        let threads = d.comment_threads().unwrap();
        let thread = threads.iter().find(|t| t.comment.id == id("a10.0")).unwrap();
        let latest = thread.latest_state(&StateModel::Review).unwrap();
        assert_eq!(
            (thread.states.len(), latest.state.as_ref().unwrap().state.as_str()),
            (2, "Completed")
        );
    }

    #[test]
    fn without_an_author_the_contents_name_the_state() {
        let mut d = doc("threads.pdf");
        let new = apply(&mut d, SetReviewState::new(id("a10.0"), ReviewState::None))
            .unwrap()
            .created_annotations
            .remove(0);
        assert_eq!(text(&dict(&d, &new), "Contents").as_deref(), Some("None"));
    }
}

mod delete_many {
    use super::*;

    fn ids(ops: &[Op]) -> Vec<String> {
        ops.iter()
            .map(|op| match op {
                Op::DeleteAnnot(op) => op.id.to_string(),
                other => panic!("not a delete: {other:?}"),
            })
            .collect()
    }

    #[test]
    fn a_comment_and_its_reply_are_one_delete() {
        let d = doc("threads.pdf");
        let ops = delete_annotations(&d, &[id("a12.0"), id("a10.0")]).unwrap();
        assert_eq!(ids(&ops), ["a10.0"]);
    }

    #[test]
    fn deleting_a_comment_with_its_reply_deletes_both_in_one_step() {
        let mut d = doc("threads.pdf");
        let ops = delete_annotations(&d, &[id("a12.0"), id("a10.0")]).unwrap();
        d.apply_all(Origin::Local, ops).unwrap();
        let left: Vec<AnnotId> = d.comment_threads().unwrap().into_iter().map(|t| t.comment.id).collect();
        d.undo().unwrap();
        assert_eq!((left.contains(&id("a10.0")), d.overlay().is_empty()), (false, true));
    }

    #[test]
    fn unrelated_annotations_each_get_their_delete_once() {
        let d = doc("threads.pdf");
        let ops = delete_annotations(&d, &[id("a16.0"), id("a10.0"), id("a10.0")]).unwrap();
        assert_eq!(ids(&ops), ["a16.0", "a10.0"]);
    }

    #[test]
    fn a_ring_s_members_are_one_delete() {
        let d = doc("threads.pdf");
        let ops = delete_annotations(&d, &[id("a14.0"), id("a15.0")]).unwrap();
        assert_eq!(ops.len(), 1);
    }

    #[test]
    fn both_members_of_a_group_are_one_delete() {
        let d = doc("threads.pdf");
        let ops = delete_annotations(&d, &[id("a17.0"), id("a16.0")]).unwrap();
        assert_eq!(ids(&ops), ["a17.0"]);
    }

    #[test]
    fn an_unknown_id_is_refused() {
        let d = doc("threads.pdf");
        let refused = delete_annotations(&d, &[id("a99.0")]).map_err(|e| e.code());
        assert_eq!(refused.err(), Some("unknown-annotation"));
    }
}

#[test]
fn both_ops_are_annotation_changes_saved_incrementally() {
    let ops: [Op; 2] = [
        AddReply::new(id("a1.0"), "x").into(),
        SetReviewState::new(id("a1.0"), ReviewState::Accepted).into(),
    ];
    assert_eq!(
        ops.iter()
            .map(|op| (op.class(), op.requires_rewrite(), op.page_changes().is_empty()))
            .collect::<Vec<_>>(),
        [(ModificationClass::Annotation, false, true); 2]
    );
}

#[test]
fn undo_leaves_the_overlay_empty_and_redo_writes_the_same_again() {
    let mut d = doc("review.pdf");
    let before = snapshot(&d);
    reply(&mut d, "a10.0", "One more");
    state(&mut d, "a16.0", ReviewState::Cancelled, "D:20261113120000Z");
    let after = snapshot(&d);
    d.undo().unwrap();
    d.undo().unwrap();
    let undone = snapshot(&d);
    d.redo().unwrap();
    d.redo().unwrap();
    assert_eq!((undone, snapshot(&d)), (before, after));
}

#[test]
fn a_reply_in_a_tagged_file_gets_an_annot_element() {
    let mut d = doc("tagged.pdf");
    let new = reply(&mut d, "a11.0", "Tagged reply");
    assert!(dict(&d, &new).get_int("StructParent").is_some());
}

#[test]
fn the_frozen_samples_apply() {
    let mut d = doc("threads.pdf");
    for name in ["add-reply", "set-review-state"] {
        let op: Op = serde_json::from_slice(&fixture(&format!("ops/v1/{name}.json"))).unwrap();
        assert_eq!(apply(&mut d, op).unwrap().created_annotations.len(), 1, "{name}");
    }
}

#[test]
fn the_wire_rejects_unknown_fields_and_states() {
    for json in [
        r#"{ "op": "addReply", "args": { "parent": "a10.0", "contents": "x", "rich": "y" } }"#,
        r#"{ "op": "setReviewState", "args": { "target": "a10.0", "state": "maybe" } }"#,
        r#"{ "op": "setReviewState", "args": { "target": "a10.0" } }"#,
    ] {
        assert!(serde_json::from_str::<Op>(json).is_err(), "{json}");
    }
}

#[test]
fn deleting_a_comment_takes_its_replies_and_states_with_it() {
    let mut d = doc("threads.pdf");
    let new_reply = reply(&mut d, "a10.0", "Bye");
    let new_state = state(&mut d, "a10.0", ReviewState::Accepted, "D:20261113120000Z");
    let applied = apply(&mut d, DeleteAnnot::new(id("a10.0"))).unwrap();
    assert!(applied.also_deleted.contains(&new_reply) && applied.also_deleted.contains(&new_state));
}
