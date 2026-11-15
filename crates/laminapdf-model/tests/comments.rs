//! `Document::comment_threads` over `fixtures/annotations/` (written by
//! `tools/fixtures/annotations.py`): replies in order across pages, states as names or strings
//! and the latest per model, groups, and broken threads, none dropped.
#![expect(clippy::unwrap_used, reason = "tests: a panic is a test failure")]

use laminapdf_model::{Annot, BrokenThread, CommentThread, Document, OpenOptions, StateModel};
use laminapdf_test_fixtures::fixture;

fn threads(name: &str) -> Vec<CommentThread> {
    let doc = Document::open(fixture(&format!("annotations/{name}")), OpenOptions::default()).unwrap();
    doc.comment_threads().unwrap()
}

fn object(a: &Annot) -> u32 {
    a.id.as_lamina_id().obj_ref().unwrap().num
}

fn objects(list: &[Annot]) -> Vec<u32> {
    list.iter().map(object).collect()
}

fn thread_of(threads: &[CommentThread], num: u32) -> &CommentThread {
    threads.iter().find(|t| object(&t.comment) == num).unwrap()
}

#[test]
fn every_markup_annotation_of_review_pdf_is_in_one_thread() {
    let threads = threads("review.pdf");
    let mut seen: Vec<u32> = threads
        .iter()
        .flat_map(|t| {
            std::iter::once(object(&t.comment))
                .chain(objects(&t.replies))
                .chain(objects(&t.group))
                .chain(objects(&t.states))
        })
        .collect();
    seen.sort_unstable();
    assert_eq!(seen, [10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 24, 30]);
}

#[test]
fn the_threads_start_at_their_comments_in_document_order() {
    let starts: Vec<u32> = threads("review.pdf").iter().map(|t| object(&t.comment)).collect();
    assert_eq!(starts, [10, 16, 17, 18, 20]);
}

#[test]
fn replies_to_replies_and_on_other_pages_join_the_comment_s_thread() {
    let threads = threads("review.pdf");
    assert_eq!(objects(&thread_of(&threads, 10).replies), [11, 12, 30]);
}

#[test]
fn a_reply_says_what_it_answers() {
    let threads = threads("review.pdf");
    let reply = &thread_of(&threads, 10).replies[1];
    assert_eq!(
        reply.in_reply_to.as_ref().map(ToString::to_string),
        Some("a11.0".to_owned())
    );
}

#[test]
fn states_read_as_strings_and_as_names() {
    let threads = threads("review.pdf");
    let states: Vec<(StateModel, String)> = thread_of(&threads, 10)
        .states
        .iter()
        .map(|a| {
            let s = a.state.clone().unwrap();
            (s.model, s.state)
        })
        .collect();
    assert_eq!(
        states,
        [
            (StateModel::Review, "Accepted".to_owned()),
            (StateModel::Review, "Rejected".to_owned()),
            (StateModel::Marked, "Marked".to_owned()),
        ]
    );
}

#[test]
fn the_latest_review_state_goes_by_date_not_by_position() {
    let threads = threads("review.pdf");
    let latest = thread_of(&threads, 10).latest_state(&StateModel::Review).unwrap();
    assert_eq!(latest.state.as_ref().unwrap().state, "Accepted");
}

#[test]
fn each_model_has_its_own_latest_state() {
    let threads = threads("review.pdf");
    let marked = thread_of(&threads, 10).latest_state(&StateModel::Marked).unwrap();
    assert_eq!(object(marked), 15);
}

#[test]
fn a_state_without_a_model_takes_the_model_of_its_name() {
    let threads = threads("review.pdf");
    let latest = thread_of(&threads, 16).latest_state(&StateModel::Review).unwrap();
    assert_eq!(latest.state.as_ref().unwrap().state, "Completed");
}

#[test]
fn a_reply_to_a_missing_object_is_a_broken_thread_of_its_own() {
    let threads = threads("review.pdf");
    assert_eq!(thread_of(&threads, 17).broken, Some(BrokenThread::MissingParent));
}

#[test]
fn a_ring_of_replies_is_one_broken_thread_from_its_first_annotation() {
    let threads = threads("review.pdf");
    let ring = thread_of(&threads, 18);
    assert_eq!(
        (ring.broken, objects(&ring.replies)),
        (Some(BrokenThread::Cycle), vec![19])
    );
}

#[test]
fn a_grouped_caret_belongs_to_its_strike_out() {
    let threads = threads("review.pdf");
    assert_eq!(objects(&thread_of(&threads, 20).group), [21]);
}

#[test]
fn a_comment_that_answers_nothing_is_not_broken() {
    let threads = threads("review.pdf");
    assert_eq!(thread_of(&threads, 10).broken, None);
}

#[test]
fn popups_are_in_no_thread() {
    let threads = threads("review.pdf");
    assert!(threads.iter().all(|t| object(&t.comment) != 23));
}

#[test]
fn the_older_threads_fixture_reads_without_dropping_anything() {
    let threads = threads("threads.pdf");
    let count: usize = threads
        .iter()
        .map(|t| 1 + t.replies.len() + t.group.len() + t.states.len())
        .sum();
    assert_eq!(count, 8);
}
