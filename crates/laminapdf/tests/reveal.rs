//! `Document::reveal` over the fixtures: what each category finds.
#![cfg(feature = "extract")]
#![allow(
    clippy::unwrap_used,
    clippy::indexing_slicing,
    reason = "tests: a panic is a test failure"
)]

use laminapdf::reveal::{Category, Reveal};
use laminapdf::{CancelToken, Document, OpenOptions};
use laminapdf_test_fixtures::{disclosing, fixture};

fn reveal(bytes: Vec<u8>) -> Reveal {
    let doc = Document::open_bytes(bytes, OpenOptions::default()).unwrap();
    doc.reveal(&OpenOptions::default(), &CancelToken::new()).unwrap()
}

#[test]
fn a_file_built_to_disclose_finds_all_seven_categories() {
    let r = reveal(disclosing());
    assert_eq!(r.found(), Category::ALL);
    assert_eq!(r.revisions.len(), 2);
    assert_eq!(r.revisions[1].changed, [4]);
    assert_eq!(r.revisions[1].pages, [0]);
    assert_eq!(r.revisions[0].removed_words, 3, "Removed draft wording");
    let keys: Vec<&str> = r.info.iter().map(|(k, _)| k.as_str()).collect();
    assert_eq!(keys, ["Title", "Producer", "SourceModified"]);
    assert!(r.hidden.iter().any(|d| d.page == 0));
}

#[test]
fn a_plain_file_finds_nothing() {
    let r = reveal(fixture("corpus/hello.pdf"));
    assert_eq!(r.found(), []);
    assert_eq!(r.revisions.len(), 1);
    assert!(!r.linearized && !r.repaired);
}

#[test]
fn an_incremental_update_rewrites_one_content_stream_and_removes_a_word() {
    let r = reveal(fixture("corpus/hello-incremental.pdf"));
    assert_eq!(r.revisions.len(), 2);
    let update = &r.revisions[1];
    assert_eq!(
        (update.added.as_slice(), update.changed.as_slice()),
        ([].as_slice(), [5].as_slice())
    );
    assert_eq!(update.pages, [0]);
    assert_eq!(r.revisions[0].removed_words, 1);
    assert_eq!(r.revisions[0].removed_on, [0]);
    assert!(r.found().contains(&Category::History));
}

#[test]
fn a_linearized_file_is_one_revision_and_says_so() {
    let r = reveal(fixture("revisions/hello-linearized.pdf"));
    assert_eq!(r.revisions.len(), 1);
    assert!(r.linearized);
}

#[test]
fn a_cancelled_token_stops_it() {
    let doc = Document::open_bytes(disclosing(), OpenOptions::default()).unwrap();
    let cancel = CancelToken::new();
    cancel.cancel();
    assert_eq!(
        doc.reveal(&OpenOptions::default(), &cancel).unwrap_err().code(),
        "cancelled"
    );
}

#[test]
fn every_fixture_reveals_without_error() {
    for dir in ["corpus", "golden", "signed"] {
        for entry in std::fs::read_dir(laminapdf_test_fixtures::fixtures_dir().join(dir)).unwrap() {
            let path = entry.unwrap().path();
            if path.extension().is_none_or(|e| e != "pdf") {
                continue;
            }
            let Ok(doc) = Document::open_bytes(std::fs::read(&path).unwrap(), OpenOptions::default()) else {
                continue;
            };
            if let Err(e) = doc.reveal(&OpenOptions::default(), &CancelToken::new()) {
                panic!("{}: {} {e}", path.display(), e.code());
            }
        }
    }
}
