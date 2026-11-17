//! The comment summary cases of the round-trip suite (D.15): the summary of
//! `fixtures/annotations/review.pdf` in both layouts, a new PDF/UA-1 document, saved as a full
//! file. Lamina reopens each with its pages, its title and no marks, unrepaired; the tools compare
//! the same facts, and pdf.js also counts the highlighted words in the text it extracts: twice
//! beside the page (drawn on the page, then quoted), once in the list (`phrases`).
#![allow(
    unreachable_pub,
    reason = "shared by the tests and the `roundtrip` example, as `tests/common` helpers are"
)]
#![allow(
    clippy::unwrap_used,
    clippy::indexing_slicing,
    reason = "test support: a panic is a test failure"
)]

use std::path::Path;

use laminapdf_cos::OpenOptions;
use serde_json::json;

use super::roundtrip::{Suite, facts, mark_facts};

pub fn summarised(root: &Path, suite: &mut Suite) {
    let source = "annotations/review.pdf";
    let bytes = std::fs::read(root.join("fixtures").join(source)).unwrap();
    let font = std::fs::read(root.join("fixtures/fonts/NotoSans-Regular.ttf")).unwrap();
    let doc = laminapdf::Document::open_bytes(bytes, OpenOptions::default()).unwrap();
    for (id, layout, quoted) in [
        ("summary-beside", laminapdf::SummaryLayout::BesidePage, 2),
        ("summary-list", laminapdf::SummaryLayout::List, 1),
    ] {
        let options =
            laminapdf::SummaryOptions::new(laminapdf::Font::from_bytes(font.clone()).unwrap()).with_layout(layout);
        let mut summary = doc.comment_summary(&options).unwrap();
        let saved = summary.save(&laminapdf::SaveOptions::new()).unwrap();
        suite.saved += 1;
        let output = saved.bytes;
        let reopened = laminapdf_model::Document::open(output.clone(), OpenOptions::default()).unwrap();
        let (pages, rotations, info, _) = facts(&reopened);
        let marks = mark_facts(&reopened);
        let found = quoted_words(&output);
        if pages == 0
            || found != quoted
            || info[0].as_deref() != Some("Comment summary")
            || marks.iter().any(|page| !page.is_empty())
            || reopened.was_repaired()
        {
            suite.failures.push(format!(
                "{id}: lamina reopened {pages} pages titled {:?} with marks {marks:?} and {found} \"Quoted words\" (repaired {})",
                info[0],
                reopened.was_repaired()
            ));
        }
        let file = format!("{id}.pdf");
        suite.manifest.push(json!({
            "file": &file,
            "source": source,
            "mode": format!("{:?}", saved.report.mode()),
            "pages": pages,
            "rotations": rotations,
            "title": info[0],
            "info": null,
            "encrypted": false,
            "direct_info": matches!(reopened.trailer().get("Info"), Some(laminapdf_cos::Object::Dict(_))),
            "invalidated": 0,
            "marks": marks,
            "phrases": { "Quoted words": quoted },
        }));
        suite.outputs.push((file, output));
    }
}

/// How often "Quoted words" occurs in the text Lamina PDF extracts from `pdf`, its runs joined by
/// single spaces, as pdf.js's checker counts it.
fn quoted_words(pdf: &[u8]) -> usize {
    let doc = laminapdf::Document::open_bytes(pdf.to_vec(), OpenOptions::default()).unwrap();
    let mut runs = Vec::new();
    for page in 0..doc.page_count().unwrap() {
        let text = doc
            .page_text(page, &laminapdf::extract::TextOptions::default())
            .unwrap();
        runs.extend(text.runs.into_iter().map(|r| r.text));
    }
    let text = runs.join(" ").split_whitespace().collect::<Vec<_>>().join(" ");
    text.matches("Quoted words").count()
}
