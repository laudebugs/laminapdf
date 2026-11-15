//! Saving replies and review states (E5 plan §5): `AddReply`'s and `SetReviewState`'s verdict on
//! the signed fixtures (class `Annotation`: approval signatures, a FieldMDP lock and a
//! certification that allows annotations (P3) allow them, P1 and P2 forbid them), and both saved in
//! each mode, reopened as one thread, `qpdf --check` clean.
#![allow(
    clippy::unwrap_used,
    clippy::indexing_slicing,
    reason = "tests: a panic is a test failure"
)]

mod common;

use std::process::Command;

use common::fixture;
use common::save::editable;
use laminapdf_cos::OpenOptions;
use laminapdf_model::{AnnotId, StateModel};
use laminapdf_ops::{AddReply, Op, Origin, ReviewState, SetReviewState};
use laminapdf_save::{ConstraintKind, DocMdpPermission, Restriction, Save as _, SaveMode, SaveOptions, SavePlanner};

fn id(text: &str) -> AnnotId {
    text.parse().unwrap()
}

/// A reply to the highlight of `review.pdf` and a state set on it; the planner judges an op by
/// its class, so the same ops serve the signed fixtures.
fn review_ops() -> Vec<Op> {
    let mut reply = AddReply::new(id("a10.0"), "Fixed in the next draft");
    reply.author = Some("Ada".to_owned());
    vec![
        reply.into(),
        SetReviewState::new(id("a10.0"), ReviewState::Completed).into(),
    ]
}

mod would_violate {
    use super::*;

    fn verdicts(name: &str) -> (ConstraintKind, Vec<Vec<Restriction>>) {
        let doc = editable(&fixture(&format!("signed/{name}")));
        let planner = SavePlanner::new(&doc).unwrap();
        let restrictions = review_ops()
            .iter()
            .map(|op| planner.would_violate(op).iter().map(|v| v.restriction).collect())
            .collect();
        (planner.constraint().kind(), restrictions)
    }

    fn none() -> (ConstraintKind, Vec<Vec<Restriction>>) {
        (ConstraintKind::AppendOnly, vec![Vec::new(); 2])
    }

    #[test]
    fn approval_signatures_and_a_certification_that_allows_annotations_allow_them() {
        assert_eq!(
            [
                verdicts("approval.pdf"),
                verdicts("two-signatures.pdf"),
                verdicts("certified-p3.pdf")
            ],
            [none(), none(), none()]
        );
    }

    #[test]
    fn a_fieldmdp_lock_allows_them_since_they_change_no_field() {
        assert_eq!(verdicts("locked.pdf"), none());
    }

    #[test]
    fn a_certification_without_annotations_forbids_them() {
        let all = |p| (ConstraintKind::AppendOnly, vec![vec![Restriction::DocMdp(p)]; 2]);
        assert_eq!(
            [verdicts("certified-p1.pdf"), verdicts("certified-p2.pdf")],
            [all(DocMdpPermission::NoChanges), all(DocMdpPermission::FormFill)]
        );
    }

    #[test]
    fn an_unsigned_file_has_nothing_to_forbid() {
        let doc = editable(&fixture("annotations/review.pdf"));
        let planner = SavePlanner::new(&doc).unwrap();
        let violations: usize = review_ops().iter().map(|op| planner.would_violate(op).len()).sum();
        assert_eq!((planner.constraint().kind(), violations), (ConstraintKind::Free, 0));
    }
}

fn qpdf_accepts(bytes: &[u8]) -> bool {
    let dir = std::env::temp_dir().join(format!("lamina-review-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("out.pdf");
    std::fs::write(&path, bytes).unwrap();
    // No qpdf on the machine: the nightly round trip runs it.
    Command::new("qpdf")
        .arg("--check")
        .arg(&path)
        .output()
        .map_or(true, |o| o.status.success())
}

#[test]
fn a_reply_and_a_state_save_in_both_modes_and_reopen_in_their_thread() {
    for mode in [SaveMode::Incremental, SaveMode::FullRewrite] {
        let mut doc = editable(&fixture("annotations/review.pdf"));
        doc.apply_all(Origin::Local, review_ops()).unwrap();
        let mut bytes = Vec::new();
        doc.save(&mut bytes, &SaveOptions::new().with_mode(mode)).unwrap();
        let reopened = laminapdf_model::Document::open(bytes.clone(), OpenOptions::default()).unwrap();
        let threads = reopened.comment_threads().unwrap();
        let thread = threads
            .iter()
            .find(|t| t.comment.contents.as_deref() == Some("Check this"))
            .unwrap();
        let replies: Vec<&str> = thread.replies.iter().filter_map(|a| a.contents.as_deref()).collect();
        let states = thread.states.len();
        assert_eq!(
            (
                replies.contains(&"Fixed in the next draft"),
                states,
                thread.broken,
                qpdf_accepts(&bytes)
            ),
            (true, 4, None, true),
            "{mode:?}"
        );
        // A state without a date counts as later than every state before it in the document.
        let latest = thread.latest_state(&StateModel::Review).unwrap();
        assert_eq!(latest.state.as_ref().unwrap().state, "Completed", "{mode:?}");
    }
}

#[test]
fn undoing_each_op_writes_the_base_and_redoing_it_writes_the_same_bytes_again() {
    let incremental = SaveOptions::new().with_mode(SaveMode::Incremental);
    for op in review_ops() {
        let mut once = editable(&fixture("annotations/review.pdf"));
        once.apply_all(Origin::Local, [op.clone()]).unwrap();
        let mut applied = Vec::new();
        once.save(&mut applied, &incremental).unwrap();

        let mut doc = editable(&fixture("annotations/review.pdf"));
        let base = doc.data().unwrap().to_vec();
        doc.apply_all(Origin::Local, [op.clone()]).unwrap();
        doc.undo().unwrap();
        let empty = doc.overlay().is_empty();
        let mut undone = Vec::new();
        doc.save(&mut undone, &incremental).unwrap();
        let mut redo = editable(&fixture("annotations/review.pdf"));
        redo.apply_all(Origin::Local, [op.clone()]).unwrap();
        redo.undo().unwrap();
        redo.redo().unwrap();
        let mut redone = Vec::new();
        redo.save(&mut redone, &incremental).unwrap();
        assert_eq!((empty, undone == base, redone == applied), (true, true, true), "{op:?}");
    }
}
