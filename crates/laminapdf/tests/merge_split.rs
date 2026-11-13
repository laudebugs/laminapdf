//! Merging and splitting through the façade: a merge into an open document reported in its
//! `Edited`, a new document merged from files (in one call and as a job, cancelled between
//! files), and a document split by bookmarks, saved and reopened with each chapter's bookmarks.
#![allow(
    clippy::unwrap_used,
    clippy::indexing_slicing,
    clippy::panic,
    reason = "tests: a panic is a test failure"
)]

use std::num::NonZeroUsize;

use laminapdf::{
    Budget, Document, Error, MergeDocuments, MergeInput, MergeOptions, MergeSource, OpenOptions, OutlineMerge,
    PagePosition, SaveOptions, SplitBy, Step,
};
use laminapdf_test_fixtures::fixture;

fn open(name: &str) -> Document {
    Document::open_bytes(fixture(name), OpenOptions::default()).unwrap()
}

fn reopened(doc: &mut Document) -> Document {
    let saved = doc.save(&SaveOptions::new()).unwrap();
    Document::open_bytes(saved.bytes, OpenOptions::default()).unwrap()
}

fn titles(doc: &Document) -> Vec<String> {
    doc.outline().unwrap().into_iter().map(|n| n.title).collect()
}

#[test]
fn a_merge_reports_renamed_destinations_and_saves_with_every_page() {
    let mut doc = open("pages/labels.pdf");
    let source = doc.add_source(fixture("pages/chapters.pdf"), None).unwrap();
    let edited = doc
        .apply(vec![
            MergeDocuments::new(
                vec![MergeSource::new(source), MergeSource::new(source)],
                PagePosition::End,
            )
            .with_options(MergeOptions::default().with_outline(OutlineMerge::Keep))
            .into(),
        ])
        .unwrap();
    let renamed: Vec<&str> = edited.renamed_destinations.iter().map(|r| r.to.as_str()).collect();
    assert_eq!(renamed, ["chapter-2#2"]);
    let again = reopened(&mut doc);
    assert_eq!(again.page_count().unwrap(), 10 + 24);
    assert_eq!(titles(&again).len(), 6);
}

#[test]
fn merge_makes_one_document_with_a_bookmark_per_file() {
    let merged = Document::merge(
        vec![
            MergeInput::new(fixture("pages/chapters.pdf")).with_title("chapters.pdf"),
            MergeInput::new(fixture("pages/references.pdf")).with_title("references.pdf"),
        ],
        MergeOptions::default(),
    )
    .unwrap();
    let mut merged = merged;
    let again = reopened(&mut merged);
    assert_eq!(titles(&again)[..2], ["chapters.pdf", "references.pdf"]);
}

#[test]
fn the_merge_job_opens_a_file_per_unit_then_merges() {
    let inputs = vec![
        MergeInput::new(fixture("pages/chapters.pdf")),
        MergeInput::new(fixture("pages/labels.pdf")),
    ];
    let mut job = Document::merge_job(inputs, MergeOptions::default());
    let mut steps = Vec::new();
    let merged = loop {
        match job.step(Budget::units(1)).unwrap() {
            Step::Done(doc) => break doc,
            Step::Progress { done, total, .. } => steps.push((done, total)),
            other => panic!("{other:?}"),
        }
    };
    assert_eq!(steps, [(1, Some(3)), (2, Some(3))]);
    assert_eq!(merged.page_count().unwrap(), 22);
    assert_eq!(job.step(Budget::units(1)).err(), Some(Error::JobFinished));
}

#[test]
fn a_cancelled_merge_job_stops_between_files() {
    let inputs = vec![
        MergeInput::new(fixture("pages/chapters.pdf")),
        MergeInput::new(fixture("pages/labels.pdf")),
    ];
    let mut job = Document::merge_job(inputs, MergeOptions::default());
    assert!(matches!(job.step(Budget::units(1)).unwrap(), Step::Progress { .. }));
    job.cancel_token().cancel();
    assert_eq!(job.step(Budget::units(1)).err(), Some(Error::Cancelled));
}

#[test]
fn a_wrong_password_fails_the_merge() {
    let inputs = vec![MergeInput::new(fixture("text/encrypted/text-shaped-aes-256.pdf")).with_password("nope")];
    let code = Document::merge(inputs, MergeOptions::default()).err().map(|e| e.code());
    assert_eq!(code, Some("wrong-password"));
}

#[test]
fn split_by_bookmarks_saves_each_chapter_with_its_bookmarks() {
    let doc = open("pages/chapters.pdf");
    let parts = doc
        .split(&SplitBy::Bookmarks {
            level: NonZeroUsize::MIN,
        })
        .unwrap();
    assert_eq!(parts.len(), 3);
    for (i, mut part) in parts.into_iter().enumerate() {
        let again = reopened(&mut part);
        assert_eq!(again.page_count().unwrap(), 4);
        assert_eq!(titles(&again), [format!("Chapter {}", i + 1)]);
    }
}

#[test]
fn the_split_job_steps_one_part_per_unit() {
    let doc = open("pages/chapters.pdf");
    let mut job = doc.split_job(&SplitBy::Count(NonZeroUsize::new(6).unwrap())).unwrap();
    let mut progress = 0;
    let parts = loop {
        match job.step(&doc, Budget::units(1)).unwrap() {
            Step::Done(parts) => break parts,
            Step::Progress { .. } => progress += 1,
            other => panic!("{other:?}"),
        }
    };
    assert_eq!((parts.len(), progress), (2, 1));
}
