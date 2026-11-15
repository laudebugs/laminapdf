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
        (5, 0)
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
    assert_eq!((after, doc.comment_threads(None).unwrap().len()), (4, 5));
}
