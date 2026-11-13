//! `SetPageLabels` through the save planner and both writers: the planner's verdict on the
//! signed fixtures (E2 plan §6), the labels after a save and reopen in each mode, qpdf's check
//! when it is installed, and an undone edit saving no change.
#![expect(
    clippy::unwrap_used,
    clippy::indexing_slicing,
    reason = "tests: a panic is a test failure"
)]

mod common;

use common::fixture;
use common::save::editable;
use laminapdf_cos::OpenOptions;
use laminapdf_model::{LabelStyle, PageId};
use laminapdf_ops::{Document, Op, Origin, PageLabelRange, SetPageLabels};
use laminapdf_save::{Save as _, SaveMode, SaveOptions, SavePlanner};

fn page_ids(doc: &Document) -> Vec<PageId> {
    doc.pages().unwrap().iter().map(|p| p.id().clone()).collect()
}

/// Roman front matter on the first two pages, then decimal numbers from 1.
fn front_matter(doc: &Document) -> Op {
    let pages = page_ids(doc);
    let mut ranges = vec![PageLabelRange::new(pages[0].clone(), Some(LabelStyle::LowerRoman))];
    if let Some(third) = pages.get(2) {
        ranges.push(PageLabelRange::new(third.clone(), Some(LabelStyle::Decimal)));
    }
    SetPageLabels::new(ranges).into()
}

fn labels_of(bytes: Vec<u8>) -> Vec<Option<String>> {
    let doc = laminapdf_model::Document::open(bytes, OpenOptions::default()).unwrap();
    let labels = doc.page_labels().unwrap();
    (0..doc.pages().unwrap().len()).map(|i| labels.label(i)).collect()
}

fn qpdf_accepts(bytes: &[u8]) -> bool {
    let dir = std::env::temp_dir().join(format!("lamina-labels-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("out.pdf");
    std::fs::write(&path, bytes).unwrap();
    if let Ok(out) = std::process::Command::new("qpdf").arg("--check").arg(&path).output() {
        return out.status.success();
    }
    // The full lane's round trip runs qpdf on these outputs; here the test says it skipped.
    #[expect(clippy::print_stderr, reason = "the test says which check it skipped")]
    {
        eprintln!("qpdf not found: the qpdf --check of this test was skipped");
    }
    true
}

#[test]
fn new_labels_save_in_both_modes_and_reopen() {
    for mode in [SaveMode::Incremental, SaveMode::FullRewrite] {
        let mut doc = editable(&fixture("pages/labels.pdf"));
        let op = front_matter(&doc);
        doc.apply_all(Origin::Local, [op]).unwrap();
        let mut out = Vec::new();
        doc.save(&mut out, &SaveOptions::new().with_mode(mode)).unwrap();
        assert!(qpdf_accepts(&out), "{mode:?}");
        let shown = labels_of(out);
        assert_eq!(
            shown.iter().map(|l| l.as_deref().unwrap_or("")).collect::<Vec<_>>(),
            ["i", "ii", "1", "2", "3", "4", "5", "6", "7", "8"],
            "{mode:?}"
        );
    }
}

#[test]
fn removing_the_labels_saves_a_file_without_them() {
    let mut doc = editable(&fixture("pages/labels.pdf"));
    doc.apply_all(Origin::Local, [SetPageLabels::new(Vec::new()).into()])
        .unwrap();
    let mut out = Vec::new();
    doc.save(&mut out, &SaveOptions::new()).unwrap();
    assert!(labels_of(out).iter().all(Option::is_none));
}

#[test]
fn an_undone_edit_saves_the_original_bytes() {
    let bytes = fixture("pages/labels.pdf");
    let mut doc = editable(&bytes);
    let op = front_matter(&doc);
    doc.apply_all(Origin::Local, [op]).unwrap();
    doc.undo().unwrap();
    let mut out = Vec::new();
    doc.save(&mut out, &SaveOptions::new()).unwrap();
    assert_eq!(out, bytes);
}

/// The E2 plan's §6 table: page labels are a `Document` change, which `DocMDP` forbids at every
/// level (ISO 32000-2 §12.8.2.2); approval signatures and a `FieldMDP` lock forbid nothing a
/// label touches.
#[test]
fn the_planner_refuses_labels_on_certified_files_only() {
    let verdicts: Vec<(&str, usize)> = [
        "approval.pdf",
        "locked.pdf",
        "certified-p1.pdf",
        "certified-p2.pdf",
        "certified-p3.pdf",
    ]
    .into_iter()
    .map(|name| {
        let doc = editable(&fixture(&format!("signed/{name}")));
        let op = front_matter(&doc);
        (name, SavePlanner::new(&doc).unwrap().would_violate(&op).len())
    })
    .collect();
    assert_eq!(
        verdicts,
        [
            ("approval.pdf", 0),
            ("locked.pdf", 0),
            ("certified-p1.pdf", 1),
            ("certified-p2.pdf", 1),
            ("certified-p3.pdf", 1),
        ]
    );
}

/// The five files of `fixtures/pages/set-labels/`: `pages/labels.pdf` given new labels and
/// saved incrementally. Each is `(file name, ranges as (page index, style, prefix, first))`.
type Case = (&'static str, &'static [(usize, Option<LabelStyle>, &'static str, u32)]);

const CASES: [Case; 5] = [
    ("roman.pdf", &[(0, Some(LabelStyle::UpperRoman), "", 1)]),
    // From Y, so the letters run past Z to AA, BB, …
    ("alpha.pdf", &[(0, Some(LabelStyle::UpperLetters), "", 25)]),
    ("prefixed.pdf", &[(0, Some(LabelStyle::Decimal), "Ch-", 1)]),
    (
        "restarts.pdf",
        &[
            (0, Some(LabelStyle::Decimal), "", 1),
            (3, Some(LabelStyle::Decimal), "", 1),
            (7, Some(LabelStyle::Decimal), "", 10),
        ],
    ),
    (
        "mixed.pdf",
        &[
            (0, Some(LabelStyle::LowerRoman), "", 1),
            (2, Some(LabelStyle::Decimal), "", 5),
            (8, Some(LabelStyle::LowerLetters), "App. ", 1),
            (9, None, "Back cover", 1),
        ],
    ),
];

fn case_bytes(ranges: &[(usize, Option<LabelStyle>, &str, u32)]) -> Vec<u8> {
    let mut doc = editable(&fixture("pages/labels.pdf"));
    let pages = page_ids(&doc);
    let ranges = ranges
        .iter()
        .map(|&(index, style, prefix, first)| {
            PageLabelRange::new(pages[index].clone(), style)
                .with_prefix(prefix)
                .with_first(first)
        })
        .collect();
    doc.apply_all(Origin::Local, [SetPageLabels::new(ranges).into()])
        .unwrap();
    let mut out = Vec::new();
    doc.save(&mut out, &SaveOptions::new().with_mode(SaveMode::Incremental))
        .unwrap();
    out
}

/// Each case's bytes match its committed file (`UPDATE_GOLDEN=1` rewrites them; then run
/// `tools/fixtures/label_facts.py`), and Lamina PDF, pdf.js and PDFium show the same label on
/// every page of it (`fixtures/pages/set-labels/facts.json`).
#[test]
fn pdfjs_and_pdfium_show_the_labels_the_op_wrote() {
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/pages/set-labels");
    let update = std::env::var_os("UPDATE_GOLDEN").is_some();
    let facts: serde_json::Value = if update {
        serde_json::Value::Null
    } else {
        serde_json::from_slice(&fixture("pages/set-labels/facts.json")).unwrap()
    };
    for (name, ranges) in CASES {
        let bytes = case_bytes(ranges);
        if update {
            std::fs::create_dir_all(&dir).unwrap();
            std::fs::write(dir.join(name), &bytes).unwrap();
            continue;
        }
        assert_eq!(
            bytes,
            fixture(&format!("pages/set-labels/{name}")),
            "{name}: run with UPDATE_GOLDEN=1"
        );
        let file = &facts["files"][name];
        let ours = serde_json::to_value(labels_of(bytes)).unwrap();
        assert_eq!(
            (&ours, &ours, true),
            (&file["pdfjs"], &file["pdfium"], file["qpdf_check"].as_bool().unwrap()),
            "{name}"
        );
    }
}
