//! Pages across documents through the façade: copying pages in from a source, replacing a page
//! with the bookmarks, links and named destinations that pointed at it kept on the page that
//! takes its place, and new documents from pages (saved, reopened, with their labels).
#![allow(
    clippy::unwrap_used,
    clippy::indexing_slicing,
    reason = "tests: a panic is a test failure"
)]

use laminapdf::{
    Budget, Document, InsertPagesFrom, OpenOptions, PageId, PagePosition, ReplacePages, SaveMode, SaveOptions, Step,
};
use laminapdf_test_fixtures::fixture;

fn open(name: &str) -> Document {
    Document::open_bytes(fixture(name), OpenOptions::default()).unwrap()
}

fn ids(doc: &Document) -> Vec<PageId> {
    doc.page_infos().unwrap().into_iter().map(|p| p.id).collect()
}

/// `doc` saved and opened again.
fn reopened(doc: &mut Document) -> Document {
    let saved = doc.save(&SaveOptions::new()).unwrap();
    Document::open_bytes(saved.bytes, OpenOptions::default()).unwrap()
}

#[test]
fn copied_pages_save_and_reopen_after_the_targets_own() {
    let mut doc = open("corpus/hello.pdf");
    let source = doc.add_source(fixture("pages/references.pdf"), None).unwrap();
    let pages = doc.source_pages(&source).unwrap();
    let edited = doc
        .apply(vec![InsertPagesFrom::new(source, pages, PagePosition::End).into()])
        .unwrap();
    assert_eq!(edited.page_order.as_ref().map(Vec::len), Some(6));
    let again = reopened(&mut doc);
    assert_eq!(again.page_count().unwrap(), 6);
    // Page 4's link names "chap3", which came along.
    assert!(again.named_destination("chap3").unwrap().is_some());
}

#[test]
fn a_replaced_page_keeps_what_pointed_at_it() {
    let mut doc = open("pages/references.pdf");
    let before = ids(&doc);
    let source = doc.add_source(fixture("corpus/hello.pdf"), None).unwrap();
    let page = doc.source_pages(&source).unwrap();
    doc.apply(vec![ReplacePages::new(vec![before[2].clone()], source, page).into()])
        .unwrap();
    let again = reopened(&mut doc);
    assert_eq!(again.page_count().unwrap(), 5);
    let outline = again.outline().unwrap();
    let first = outline[0].target.as_ref().unwrap();
    assert_eq!(first.page, 2);
    // The link on page 1 names the third page too.
    let links = again.page_links(0).unwrap();
    assert!(
        links
            .iter()
            .any(|l| matches!(&l.to, laminapdf::LinkTo::Destination { target: Some(t), .. } if t.page == 2))
    );
}

#[test]
fn an_unknown_source_is_refused() {
    let mut doc = open("corpus/hello.pdf");
    let other = open("pages/references.pdf");
    let missing = laminapdf::SourceId::of(b"never added");
    let err = doc
        .apply(vec![
            InsertPagesFrom::new(missing, ids(&other), PagePosition::End).into(),
        ])
        .unwrap_err();
    assert_eq!(err.code(), "unknown-source");
    assert_eq!(doc.source_pages(&missing).unwrap_err().code(), "unknown-source");
}

#[test]
fn extracted_pages_make_a_whole_file_with_their_labels() {
    let doc = open("pages/labels.pdf");
    let pages = ids(&doc);
    let mut part = doc.extract_pages(&[pages[2].clone(), pages[3].clone()]).unwrap();
    let saved = part.save(&SaveOptions::new()).unwrap();
    assert_eq!(saved.report.mode(), SaveMode::FullRewrite);
    let again = Document::open_bytes(saved.bytes, OpenOptions::default()).unwrap();
    let labels: Vec<_> = again.page_infos().unwrap().into_iter().map(|p| p.label).collect();
    let want: Vec<_> = doc.page_infos().unwrap()[2..4]
        .iter()
        .map(|p| p.label.clone())
        .collect();
    assert_eq!(labels, want);
}

#[test]
fn export_job_steps_one_range_per_unit_and_stops_when_cancelled() {
    let doc = open("pages/references.pdf");
    let pages = ids(&doc);
    let ranges = vec![pages[..1].to_vec(), pages[1..3].to_vec(), pages[3..].to_vec()];
    let mut job = doc.export_job(ranges.clone());
    assert!(matches!(
        job.step(&doc, Budget::units(1)).unwrap(),
        Step::Progress { done: 1, .. }
    ));
    let parts = loop {
        if let Step::Done(parts) = job.step(&doc, Budget::units(1)).unwrap() {
            break parts;
        }
    };
    let counts: Vec<u32> = parts.iter().map(|p| p.page_count().unwrap()).collect();
    assert_eq!(counts, [1, 2, 2]);
    let mut job = doc.export_job(ranges);
    job.cancel_token().cancel();
    assert_eq!(job.step(&doc, Budget::units(1)).unwrap_err().code(), "cancelled");
}
