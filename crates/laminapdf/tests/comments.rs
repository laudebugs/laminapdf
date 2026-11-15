//! `Document::comment_threads` and `delete_annotations` over `fixtures/annotations/review.pdf`:
//! threads in page space with their quoted text, latest states and broken flags, a page filter,
//! and a selection deleted as one undo step.
#![expect(clippy::unwrap_used, reason = "tests: a panic is a test failure")]

use laminapdf::{AddReply, BrokenThread, Document, Op, OpenOptions, PageComment, ReviewState, SetReviewState};
use laminapdf_test_fixtures::fixture;

fn open() -> Document {
    Document::open_bytes(fixture("annotations/review.pdf"), OpenOptions::new()).unwrap()
}

fn comment<'a>(threads: &'a [PageComment], id: &str) -> &'a PageComment {
    threads.iter().find(|t| t.comment.id == id).unwrap()
}

#[test]
fn a_highlight_quotes_the_text_under_it() {
    let threads = open().comment_threads(None).unwrap();
    assert_eq!(comment(&threads, "a10.0").quoted.as_deref(), Some("Quoted words"));
}

#[test]
fn a_note_quotes_nothing() {
    let threads = open().comment_threads(None).unwrap();
    assert_eq!(comment(&threads, "a16.0").quoted, None);
}

#[test]
fn the_latest_state_of_each_model_is_given() {
    let threads = open().comment_threads(None).unwrap();
    let c = comment(&threads, "a10.0");
    assert_eq!(
        (c.review.as_deref(), c.marked.as_deref()),
        (Some("Accepted"), Some("Marked"))
    );
}

#[test]
fn every_state_keeps_its_place_in_the_history() {
    let threads = open().comment_threads(None).unwrap();
    let history: Vec<&str> = comment(&threads, "a10.0")
        .states
        .iter()
        .map(|s| s.annotation.state.as_ref().unwrap().state.as_str())
        .collect();
    assert_eq!(history, ["Accepted", "Rejected", "Marked"]);
}

#[test]
fn a_reply_on_another_page_names_its_page() {
    let threads = open().comment_threads(None).unwrap();
    let pages: Vec<u32> = comment(&threads, "a10.0").replies.iter().map(|r| r.page).collect();
    assert_eq!(pages, [0, 0, 1]);
}

#[test]
fn broken_threads_are_kept_and_flagged() {
    let threads = open().comment_threads(None).unwrap();
    assert_eq!(
        (comment(&threads, "a17.0").broken, comment(&threads, "a18.0").broken),
        (Some(BrokenThread::MissingParent), Some(BrokenThread::Cycle))
    );
}

#[test]
fn a_page_filter_keeps_the_threads_whose_comments_are_on_it() {
    let doc = open();
    assert_eq!(
        (
            doc.comment_threads(Some(&[0])).unwrap().len(),
            doc.comment_threads(Some(&[1])).unwrap().len()
        ),
        (6, 0)
    );
}

#[test]
fn a_page_past_the_end_is_refused() {
    let refused = open().comment_threads(Some(&[2])).map_err(|e| e.code().to_owned());
    assert_eq!(refused.err().as_deref(), Some("page-index-out-of-range"));
}

#[test]
fn a_new_reply_and_state_show_in_the_thread() {
    let mut doc = open();
    let parent = "a16.0".parse().unwrap();
    doc.apply(vec![
        Op::from(AddReply::new("a16.0".parse().unwrap(), "On it")),
        Op::from(SetReviewState::new(parent, ReviewState::Cancelled)),
    ])
    .unwrap();
    let threads = doc.comment_threads(None).unwrap();
    let c = comment(&threads, "a16.0");
    assert_eq!(
        (
            c.replies.last().unwrap().annotation.contents.as_deref(),
            c.review.as_deref()
        ),
        (Some("On it"), Some("Cancelled"))
    );
}

#[test]
fn deleting_a_comment_with_its_reply_is_one_undo_step() {
    let mut doc = open();
    let ids = ["a11.0".parse().unwrap(), "a10.0".parse().unwrap()];
    doc.delete_annotations(&ids).unwrap();
    let after = doc.comment_threads(None).unwrap().len();
    doc.undo().unwrap();
    assert_eq!((after, doc.comment_threads(None).unwrap().len()), (4, 6));
}

/// pdf.js's view (`fixtures/annotations/pdfjs-replies.json`, from
/// `tools/fixtures/pdfjs_replies.mjs`) of the review example's output: each reply and state
/// Lamina PDF wrote names its comment, a state with its model and name.
#[test]
fn pdfjs_reads_lamina_pdf_s_replies_and_states() {
    let facts: serde_json::Value = serde_json::from_slice(&fixture("annotations/pdfjs-replies.json")).unwrap();
    let doc = Document::open_bytes(fixture("golden/examples/review_comments.pdf"), OpenOptions::new()).unwrap();
    let ours: Vec<(String, String, Option<String>)> = doc
        .page_annotations(0)
        .unwrap()
        .into_iter()
        .filter(|a| a.in_reply_to.is_some() && !a.grouped)
        .map(|a| {
            let object = |id: &str| format!("{}R", id.trim_start_matches('a').trim_end_matches(".0"));
            (
                object(&a.id),
                object(a.in_reply_to.as_deref().unwrap()),
                a.state.map(|s| s.state),
            )
        })
        .collect();
    let theirs: Vec<(String, String, Option<String>)> = facts["files"]["review_comments.pdf"][0]["replies"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|r| r["replyType"] == "R")
        .map(|r| {
            (
                r["id"].as_str().unwrap().to_owned(),
                r["inReplyTo"].as_str().unwrap().to_owned(),
                r["state"].as_str().map(ToOwned::to_owned),
            )
        })
        .collect();
    assert_eq!((ours.len(), ours), (2, theirs));
}

/// pdf.js draws as many operators for the answered report as for the marked-up one: a reply and
/// a state draw nothing.
#[test]
fn pdfjs_draws_nothing_for_a_reply_or_a_state() {
    let facts: serde_json::Value = serde_json::from_slice(&fixture("annotations/pdfjs-replies.json")).unwrap();
    let operators = |file: &str| -> Vec<u64> {
        facts["files"][file]
            .as_array()
            .unwrap()
            .iter()
            .map(|p| p["operators"].as_u64().unwrap())
            .collect()
    };
    assert_eq!(operators("review_comments.pdf"), operators("mark_up.pdf"));
}
