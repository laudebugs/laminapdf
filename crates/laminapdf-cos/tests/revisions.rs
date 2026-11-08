//! `Document::revision_ends` on the fixtures (todo T2b): where each revision of a real file ends.
#![allow(clippy::unwrap_used, reason = "tests: a panic is a test failure")]

use laminapdf_cos::{Document, OpenOptions};
use laminapdf_test_fixtures::fixture;

fn ends(rel: &str) -> (Vec<u64>, usize) {
    let bytes = fixture(rel);
    let len = bytes.len();
    let doc = Document::open(bytes, OpenOptions::default()).unwrap();
    (doc.revision_ends().unwrap(), len)
}

#[test]
fn a_file_never_updated_has_one_revision_the_whole_file() {
    let (ends, len) = ends("corpus/hello.pdf");
    assert_eq!(ends, [len as u64]);
}

#[test]
fn an_incremental_update_ends_the_first_revision_at_its_eof() {
    let bytes = fixture("corpus/hello-incremental.pdf");
    let (ends, len) = ends("corpus/hello-incremental.pdf");
    assert_eq!(ends.len(), 2);
    let first = usize::try_from(ends[0]).unwrap();
    assert!(bytes[..first].trim_ascii_end().ends_with(b"%%EOF"));
    assert_eq!(ends[1], len as u64);
    // The prefix opens on its own, as the file before the update.
    let original = Document::open(bytes[..first].to_vec(), OpenOptions::default()).unwrap();
    assert_eq!(original.revision_ends().unwrap(), [first as u64]);
}

#[test]
fn a_linearized_file_is_one_revision() {
    let (ends, len) = ends("revisions/hello-linearized.pdf");
    assert_eq!(ends, [len as u64]);
}

#[test]
fn a_hybrid_xref_stream_section_is_no_revision_of_its_own() {
    let doc = Document::open(fixture("corpus/hello-xrefstm.pdf"), OpenOptions::default()).unwrap();
    assert_eq!(doc.revision_ends().unwrap().len(), doc.xref_section_offsets().len());
}

#[test]
fn a_repaired_file_is_one_revision() {
    let (ends, len) = ends("corpus/hello-broken.pdf");
    assert_eq!(ends, [len as u64]);
}
