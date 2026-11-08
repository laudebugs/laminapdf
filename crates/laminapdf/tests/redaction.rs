//! `Document::redaction_warnings` and the revision calls (todo T2b): covered text, unapplied
//! redaction marks and words left in earlier revisions, on built pages and the fixtures.
#![allow(clippy::unwrap_used, clippy::panic, clippy::indexing_slicing, reason = "test code")]

use laminapdf::{CancelToken, Document, OpenOptions};
use laminapdf_syntax::{ObjRef, Object, Stream};
use laminapdf_test_fixtures::{classic, fixture, fixtures_dir, stream_body};

/// A one-page file drawing `content` in Helvetica, with the annotation objects `annots`
/// (numbered from 6) on the page.
fn page(content: &[u8], annots: &[&str]) -> Document {
    let stream = stream_body("", content);
    let refs = (0..annots.len())
        .map(|i| format!("{} 0 R", 6 + i))
        .collect::<Vec<_>>()
        .join(" ");
    let page = format!(
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 600 800] /Contents 4 0 R /Annots [{refs}] \
         /Resources << /Font << /F1 5 0 R >> >> >>"
    );
    let mut bodies: Vec<&[u8]> = vec![
        b"<< /Type /Catalog /Pages 2 0 R >>",
        b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>",
        page.as_bytes(),
        &stream,
        b"<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica /Encoding /WinAnsiEncoding >>",
    ];
    bodies.extend(annots.iter().map(|a| a.as_bytes()));
    let trailer = format!("<< /Size {} /Root 1 0 R >>", bodies.len() + 1);
    Document::open_bytes(classic(&bodies, &trailer), OpenOptions::default()).unwrap()
}

const SENTENCE: &[u8] = b"BT /F1 12 Tf 72 700 Td (Pay John Smith now.) Tj ET";

/// Each warning's code and data.
fn findings(doc: &Document) -> Vec<(&'static str, serde_json::Value)> {
    doc.redaction_warnings(&OpenOptions::default(), &CancelToken::new())
        .unwrap()
        .iter()
        .map(|w| (w.code(), serde_json::to_value(w).unwrap()))
        .collect()
}

#[test]
fn a_black_box_over_a_name_is_covered_text_on_its_page() {
    let doc = page(&[SENTENCE, b" 0 g 95 694 62 16 re f"].concat(), &[]);
    let found = findings(&doc);
    assert_eq!(found.len(), 1);
    let (code, json) = &found[0];
    assert_eq!(*code, "covered-text");
    assert_eq!(json["location"]["pageIndex"], 0);
    assert_eq!(
        (json["data"]["chars"].as_u64(), json["data"]["regions"].as_u64()),
        (Some(9), Some(1))
    );
}

#[test]
fn two_redaction_marks_are_one_warning_and_none_without_them() {
    let mark = "<< /Type /Annot /Subtype /Redact /Rect [95 694 157 710] >>";
    let marked = page(SENTENCE, &[mark, mark]);
    let found = findings(&marked);
    assert_eq!(
        found
            .iter()
            .map(|(c, j)| (*c, j["data"]["annotations"].as_u64()))
            .collect::<Vec<_>>(),
        [("redaction-not-applied", Some(2))]
    );
    assert_eq!(findings(&page(SENTENCE, &[])), []);
}

#[test]
fn an_update_that_replaced_a_word_leaves_it_in_revision_zero() {
    let doc = Document::open_bytes(fixture("corpus/hello-incremental.pdf"), OpenOptions::default()).unwrap();
    assert_eq!(doc.revision_count().unwrap(), 2);
    let found = findings(&doc);
    assert_eq!(found.len(), 1);
    let (code, json) = &found[0];
    assert_eq!(*code, "text-in-earlier-revision");
    assert_eq!(
        json["data"],
        serde_json::json!({ "revision": 0, "words": 1, "pages": [0] })
    );
    assert!(
        json.get("location")
            .is_none_or(|l| l.as_object().is_none_or(serde_json::Map::is_empty))
    );
}

#[test]
fn a_linearized_file_is_one_revision_until_an_update_replaces_its_text() {
    let bytes = fixture("revisions/hello-linearized.pdf");
    let doc = Document::open_bytes(bytes.clone(), OpenOptions::default()).unwrap();
    assert_eq!(doc.revision_count().unwrap(), 1);
    assert_eq!(findings(&doc), []);
    // Replace the page's content (object 7) in an appended update.
    let cos = laminapdf_cos::Document::open(bytes, OpenOptions::default()).unwrap();
    let content = Stream::new(
        laminapdf_syntax::Dict::new(),
        b"BT /F1 24 Tf 72 700 Td (Goodbye) Tj ET".to_vec(),
    );
    let updated = laminapdf_save::test_support::incremental_update(
        &cos,
        &[(ObjRef::new(7, 0), Object::Stream(content))],
        laminapdf_save::XrefEol::CrLf,
    )
    .unwrap();
    let doc = Document::open_bytes(updated, OpenOptions::default()).unwrap();
    assert_eq!(doc.revision_count().unwrap(), 2);
    let found = findings(&doc);
    assert_eq!(
        found
            .iter()
            .map(|(c, j)| (*c, j["data"]["revision"].as_u64()))
            .collect::<Vec<_>>(),
        [("text-in-earlier-revision", Some(0))]
    );
}

#[test]
fn a_revision_past_the_last_is_out_of_range() {
    let doc = Document::open_bytes(fixture("corpus/hello-incremental.pdf"), OpenOptions::default()).unwrap();
    let count = doc.revision_count().unwrap();
    let err = doc.open_revision(count, OpenOptions::default()).unwrap_err();
    assert_eq!(err.code(), "revision-index-out-of-range");
    assert!(doc.open_revision(count - 1, OpenOptions::default()).is_ok());
}

#[test]
fn a_cancelled_token_stops_the_check() {
    let doc = Document::open_bytes(fixture("corpus/hello-incremental.pdf"), OpenOptions::default()).unwrap();
    let cancel = CancelToken::new();
    cancel.cancel();
    let err = doc.redaction_warnings(&OpenOptions::default(), &cancel).unwrap_err();
    assert_eq!(err.code(), "cancelled");
}

/// Every PDF under `dir`, recursively.
fn pdfs(dir: &std::path::Path, out: &mut Vec<std::path::PathBuf>) {
    for entry in std::fs::read_dir(dir).unwrap().flatten() {
        let path = entry.path();
        if path.is_dir() {
            pdfs(&path, out);
        } else if path.extension().is_some_and(|x| x == "pdf") {
            out.push(path);
        }
    }
}

#[test]
fn only_the_named_fixtures_give_redaction_warnings() {
    // Signatures, metadata edits and rotation add revisions but remove no text.
    let expected = [("corpus/hello-incremental.pdf", vec!["text-in-earlier-revision"])];
    let root = fixtures_dir();
    let mut files = Vec::new();
    for dir in ["corpus", "golden", "signed", "revisions"] {
        pdfs(&root.join(dir), &mut files);
    }
    files.sort();
    assert!(files.len() > 20);
    for path in files {
        let name = path.strip_prefix(&root).unwrap().to_string_lossy().replace('\\', "/");
        let Ok(doc) = Document::open_bytes(std::fs::read(&path).unwrap(), OpenOptions::default()) else {
            continue;
        };
        // A file whose text can't be read fails the check as it failed `check` before.
        let Ok(found) = doc.redaction_warnings(&OpenOptions::default(), &CancelToken::new()) else {
            continue;
        };
        let codes: Vec<&str> = found.iter().map(laminapdf::Warning::code).collect();
        let want = expected
            .iter()
            .find(|(n, _)| *n == name)
            .map(|(_, c)| c.clone())
            .unwrap_or_default();
        assert_eq!(codes, want, "{name}");
    }
}
