//! `Document::revision_ends` and `revision_index` on the fixtures: where each revision of a
//! real file ends and what its sections list.
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

/// What `Document::revision_index` lists for `rel`.
fn index(bytes: Vec<u8>) -> Vec<laminapdf_cos::Revision> {
    Document::open(bytes, OpenOptions::default())
        .unwrap()
        .revision_index()
        .unwrap()
}

/// capability: M.12 core read
#[test]
fn the_revision_index_lists_what_each_update_adds_and_frees() {
    let bytes = fixture("corpus/hello-incremental.pdf");
    let revisions = index(bytes.clone());
    assert_eq!(revisions.len(), 2);
    assert_eq!(revisions[0].in_use, [1, 2, 3, 4, 5]);
    assert_eq!(
        revisions[1].in_use,
        [5],
        "the update rewrites page 1's content stream only"
    );
    assert!(revisions.iter().all(|r| r.freed.is_empty()));
    let ends = Document::open(bytes, OpenOptions::default())
        .unwrap()
        .revision_ends()
        .unwrap();
    assert_eq!(revisions.iter().map(|r| r.end).collect::<Vec<_>>(), ends);
}

#[test]
fn a_third_revision_that_frees_an_object_lists_it_and_never_object_zero() {
    let mut bytes = fixture("corpus/hello-incremental.pdf");
    let prev = Document::open(bytes.clone(), OpenOptions::default())
        .unwrap()
        .xref_section_offsets()[0];
    let start = bytes.len();
    bytes.extend_from_slice(b"xref\n0 1\n0000000000 65535 f \n4 1\n0000000000 00001 f \n");
    bytes.extend_from_slice(
        format!("trailer\n<< /Size 6 /Root 1 0 R /Prev {prev} >>\nstartxref\n{start}\n%%EOF\n").as_bytes(),
    );
    let revisions = index(bytes);
    assert_eq!(revisions.len(), 3);
    assert_eq!(revisions[2].section, start as u64);
    assert!(revisions[2].in_use.is_empty());
    assert_eq!(revisions[2].freed, [4]);
}

#[test]
fn a_linearized_file_indexes_as_one_revision_with_both_sections_objects() {
    let bytes = fixture("revisions/hello-linearized.pdf");
    let doc = Document::open(bytes, OpenOptions::default()).unwrap();
    assert_eq!(doc.xref_section_offsets().len(), 2);
    let revisions = doc.revision_index().unwrap();
    assert_eq!(revisions.len(), 1);
    let mut all = revisions[0].in_use.clone();
    all.dedup();
    assert_eq!(all, revisions[0].in_use, "sorted and without duplicates");
    assert!(revisions[0].in_use.len() >= 5);
}

#[test]
fn a_signed_files_index_matches_its_revision_ends() {
    let doc = Document::open(fixture("signed/approval.pdf"), OpenOptions::default()).unwrap();
    let revisions = doc.revision_index().unwrap();
    assert!(!revisions.is_empty());
    assert_eq!(
        revisions.iter().map(|r| r.end).collect::<Vec<_>>(),
        doc.revision_ends().unwrap()
    );
}

#[test]
fn a_repaired_file_has_no_revision_index() {
    assert!(index(fixture("corpus/hello-broken.pdf")).is_empty());
}
