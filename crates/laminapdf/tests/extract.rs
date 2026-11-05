//! Extraction through the façade: the same result as a stepped job, a job ended by an edit,
//! errors in the façade's codes, and JSON that carries the document's warnings.
#![cfg(feature = "extract")]
#![allow(clippy::unwrap_used, clippy::panic, clippy::indexing_slicing, reason = "test code")]

use laminapdf::extract::{ExtractOptions, MarkdownOptions};
use laminapdf::{Budget, Document, Op, OpenOptions, RotatePage, Rotation, Step};

fn open(rel: &str) -> Document {
    Document::open_bytes(laminapdf_test_fixtures::fixture(rel), OpenOptions::default()).unwrap()
}

#[test]
fn a_stepped_job_gives_what_extract_gives() {
    let doc = open("golden/examples/links_and_outlines.pdf");
    let whole = doc.extract(&ExtractOptions::new()).unwrap();
    let mut job = doc.extract_job(&ExtractOptions::new()).unwrap();
    let mut steps = 0;
    let stepped = loop {
        steps += 1;
        match job.step(&doc, Budget::units(1)).unwrap() {
            Step::Done(out) => break out,
            Step::Progress { .. } => {}
            _ => panic!("unexpected step"),
        }
    };
    assert_eq!(stepped, whole);
    assert!(steps > 1, "one page per unit");
    assert_eq!(job.step(&doc, Budget::units(1)).unwrap_err().code(), "job-finished");
}

#[test]
fn an_edit_between_steps_ends_the_job() {
    let mut doc = open("golden/examples/links_and_outlines.pdf");
    let mut job = doc.extract_job(&ExtractOptions::new()).unwrap();
    assert!(matches!(
        job.step(&doc, Budget::units(1)).unwrap(),
        Step::Progress { .. }
    ));
    let page = doc.page_info(0).unwrap().id;
    doc.apply(vec![Op::from(RotatePage::new(page, Rotation::Clockwise90))])
        .unwrap();
    assert_eq!(job.step(&doc, Budget::units(1)).unwrap_err().code(), "document-changed");
}

#[test]
fn a_missing_page_is_the_facades_page_index_error() {
    let doc = open("golden/examples/hello.pdf");
    let err = doc.extract(&ExtractOptions::new().with_pages(vec![7])).unwrap_err();
    assert_eq!(err.code(), "page-index-out-of-range");
    assert_eq!(err.diagnostic().data["index"], 7);
}

#[test]
fn a_cancelled_extraction_is_cancelled() {
    let doc = open("golden/examples/hello.pdf");
    let token = laminapdf::CancelToken::new();
    token.cancel();
    assert_eq!(
        doc.extract_with(&ExtractOptions::new(), &token).unwrap_err().code(),
        "cancelled"
    );
}

#[test]
fn the_json_carries_the_documents_warnings() {
    // A damaged file whose repair raises warnings on open.
    let doc = open("corpus/hello-broken.pdf");
    let extraction = doc.extract(&ExtractOptions::new()).unwrap();
    let markdown = extraction.to_markdown(&MarkdownOptions::new());
    let json: serde_json::Value = serde_json::from_str(&doc.extract_json(&extraction, &markdown, None)).unwrap();
    assert_eq!(json["schema"], "laminapdf.extract/1");
    assert_eq!(json["generator"]["name"], "laminapdf");
    let codes: Vec<&str> = json["warnings"]
        .as_array()
        .unwrap()
        .iter()
        .map(|w| w["code"].as_str().unwrap())
        .collect();
    assert!(!codes.is_empty(), "the repaired file's warnings are in the JSON");
    assert!(json.get("chunks").is_none(), "no chunks unless they were made");
}

/// The files whose JSON `@laminapdf/sdk` must reproduce byte for byte (its `extract` test reads
/// the same goldens): a ruled table, two columns, and a repaired file with warnings.
const SHARED: [&str; 3] = [
    "reading-order/table-ordered.pdf",
    "reading-order/two-columns-ordered.pdf",
    "corpus/hello-broken.pdf",
];

/// The JSON with the generator's version replaced by `*`, so a release doesn't stale the goldens.
fn without_version(json: &str) -> String {
    json.replacen(
        &format!("\"version\":\"{}\"", env!("CARGO_PKG_VERSION")),
        "\"version\":\"*\"",
        1,
    )
}

/// `doc.extract({ format: "json", pageMarkers: true, chunk: { maxTokens: 400 } })` in
/// TypeScript gives these files. `UPDATE_GOLDEN=1` rewrites them.
#[test]
fn the_json_matches_the_goldens_the_sdk_shares() {
    use laminapdf::extract::ChunkOptions;
    for rel in SHARED {
        let doc = open(rel);
        let extraction = doc.extract(&ExtractOptions::new()).unwrap();
        let markdown = extraction.to_markdown(&MarkdownOptions::new().with_page_markers(true));
        let chunks = extraction.chunks(&markdown, &ChunkOptions::new().with_max_tokens(400));
        let json = without_version(&doc.extract_json(&extraction, &markdown, Some(chunks)));
        let stem = rel.rsplit('/').next().unwrap().trim_end_matches(".pdf");
        let golden = laminapdf_test_fixtures::fixtures_dir().join(format!("extract/json/{stem}.json"));
        if std::env::var_os("UPDATE_GOLDEN").is_some() {
            std::fs::create_dir_all(golden.parent().unwrap()).unwrap();
            std::fs::write(&golden, &json).unwrap();
        }
        let want = std::fs::read_to_string(&golden).unwrap_or_default();
        assert!(json == want, "{} is stale: run with UPDATE_GOLDEN=1", golden.display());
    }
}
