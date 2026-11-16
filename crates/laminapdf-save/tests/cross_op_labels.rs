//! Page labels across every page op (todo E2i, moved from E2g): after `SetPageLabels` gives
//! `pages/labels.pdf` roman front matter, an arabic body and a prefixed appendix, each op that
//! adds, removes or reorders pages leaves every surviving page the label it had, as
//! `PageLabels::remap` expects, and gives new pages theirs. pdf.js and PDFium show the same
//! labels on the saved files (`fixtures/pages/cross-op-labels/facts.json`).
#![expect(
    clippy::unwrap_used,
    clippy::indexing_slicing,
    reason = "tests: a panic is a test failure"
)]

mod common;

use common::fixture;
use common::save::editable;
use laminapdf_cos::OpenOptions;
use laminapdf_model::{LabelStyle, PageId, PageLabels, PagePosition, PageSize};
use laminapdf_ops::{
    DeletePages, Document, DuplicatePages, InsertBlankPages, InsertPagesFrom, MergeDocuments, MergeSource, MovePages,
    Op, Origin, PageLabelRange, ReplacePages, ReversePages, SetPageLabels,
};
use laminapdf_save::{Save as _, SaveMode, SaveOptions};

fn page_ids(doc: &Document) -> Vec<PageId> {
    doc.pages().unwrap().iter().map(|p| p.id().clone()).collect()
}

fn shown(labels: &PageLabels, count: usize) -> Vec<Option<String>> {
    (0..count).map(|i| labels.label(i)).collect()
}

fn labels_of(doc: &laminapdf_model::Document) -> Vec<Option<String>> {
    shown(&doc.page_labels().unwrap(), doc.pages().unwrap().len())
}

/// `pages/labels.pdf` (ten pages) after `SetPageLabels`: i–iii, 1–4, then A-1 to A-3.
fn labelled() -> Document {
    let mut doc = editable(&fixture("pages/labels.pdf"));
    let pages = page_ids(&doc);
    let ranges = vec![
        PageLabelRange::new(pages[0].clone(), Some(LabelStyle::LowerRoman)),
        PageLabelRange::new(pages[3].clone(), Some(LabelStyle::Decimal)),
        PageLabelRange::new(pages[7].clone(), Some(LabelStyle::Decimal)).with_prefix("A-"),
    ];
    doc.apply_all(Origin::Local, [Op::from(SetPageLabels::new(ranges))])
        .unwrap();
    doc
}

/// The ops, each on the labelled document: a name, then the op (or, for extract, the pages).
enum Edit {
    Op(fn(&mut Document, &[PageId]) -> Op),
    /// Merging `corpus/hello.pdf` in at the end.
    Merge,
    Extract(fn(&[PageId]) -> Vec<PageId>),
}

/// `corpus/hello.pdf` (one page) as a source, and its page.
fn hello(doc: &mut Document) -> (laminapdf_ops::SourceId, Vec<PageId>) {
    let id = doc.add_source(fixture("corpus/hello.pdf"), None).unwrap();
    let pages = doc
        .source_document(&id)
        .unwrap()
        .pages()
        .unwrap()
        .iter()
        .map(|p| p.id().clone())
        .collect();
    (id, pages)
}

const CASES: [(&str, Edit); 9] = [
    (
        "insert-blank.pdf",
        Edit::Op(|_, p| InsertBlankPages::new(PagePosition::Before(p[4].clone()), 2, PageSize::LETTER).into()),
    ),
    (
        "delete.pdf",
        Edit::Op(|_, p| DeletePages::new(vec![p[3].clone(), p[8].clone()]).into()),
    ),
    (
        "duplicate.pdf",
        Edit::Op(|_, p| DuplicatePages::new(vec![p[5].clone()], PagePosition::After(p[5].clone())).into()),
    ),
    (
        "move.pdf",
        Edit::Op(|_, p| MovePages::new(vec![p[8].clone()], PagePosition::Before(p[4].clone())).into()),
    ),
    (
        "reverse.pdf",
        Edit::Op(|_, p| ReversePages::new(p[3..7].to_vec()).into()),
    ),
    (
        "insert-from.pdf",
        Edit::Op(|doc, p| {
            let (id, pages) = hello(doc);
            InsertPagesFrom::new(id, pages, PagePosition::Before(p[6].clone())).into()
        }),
    ),
    (
        "replace.pdf",
        Edit::Op(|doc, p| {
            let (id, pages) = hello(doc);
            ReplacePages::new(vec![p[4].clone()], id, pages).into()
        }),
    ),
    ("merge.pdf", Edit::Merge),
    (
        "extract.pdf",
        Edit::Extract(|p| vec![p[1].clone(), p[5].clone(), p[8].clone()]),
    ),
];

/// The edited document and the labels expected on it: `PageLabels::remap`'s, except after a
/// merge, whose copies keep the label they had in their source (`MergeDocuments`): this
/// document's pages keep theirs, and `hello.pdf`, which has no labels, numbers its page 1.
fn run(edit: &Edit) -> (Document, Vec<Option<String>>) {
    let mut doc = labelled();
    let before = page_ids(&doc);
    let labels = doc.page_labels().unwrap();
    match edit {
        Edit::Op(op) => {
            let op = op(&mut doc, &before);
            doc.apply_all(Origin::Local, [op]).unwrap();
            let after = page_ids(&doc);
            let expected = shown(&labels.remap(&before, &after), after.len());
            (doc, expected)
        }
        Edit::Merge => {
            let (id, _) = hello(&mut doc);
            let op = MergeDocuments::new(vec![MergeSource::new(id)], PagePosition::End).into();
            doc.apply_all(Origin::Local, [op]).unwrap();
            let mut expected = shown(&labels, before.len());
            expected.push(Some("1".to_owned()));
            (doc, expected)
        }
        Edit::Extract(pick) => {
            let chosen = pick(&before);
            let expected = shown(&labels.remap(&before, &chosen), chosen.len());
            (doc.extract_pages(&chosen).unwrap(), expected)
        }
    }
}

/// Every op leaves each surviving page the label it had: only new pages get new labels.
#[test]
fn every_page_op_keeps_each_surviving_pages_label() {
    for (name, edit) in &CASES {
        let doc = labelled();
        let before: std::collections::BTreeMap<PageId, Option<String>> =
            page_ids(&doc).into_iter().zip(labels_of(&doc)).collect();
        let (edited, expected) = run(edit);
        let got = labels_of(&edited);
        assert_eq!(got, expected, "{name}: against PageLabels::remap");
        if !matches!(edit, Edit::Extract(_)) {
            for (id, label) in page_ids(&edited).iter().zip(&got) {
                if let Some(was) = before.get(id) {
                    assert_eq!(label, was, "{name}: page {id:?} was relabelled");
                }
            }
        }
    }
}

/// The saved bytes of a case: the edited document, incrementally; an extracted one whole.
fn case_bytes(edit: &Edit) -> (Vec<u8>, Vec<Option<String>>) {
    let (mut doc, expected) = run(edit);
    let mode = match edit {
        Edit::Op(_) | Edit::Merge => SaveMode::Incremental,
        Edit::Extract(_) => SaveMode::FullRewrite,
    };
    let mut out = Vec::new();
    doc.save(&mut out, &SaveOptions::new().with_mode(mode)).unwrap();
    (out, expected)
}

/// Each case's bytes match its committed file (`UPDATE_GOLDEN=1` rewrites them; then run
/// `tools/fixtures/label_facts.py`), and Lamina PDF, pdf.js and PDFium all show the labels
/// `PageLabels::remap` expects.
#[test]
fn pdfjs_and_pdfium_show_the_labels_after_every_page_op() {
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/pages/cross-op-labels");
    let update = std::env::var_os("UPDATE_GOLDEN").is_some();
    let facts: serde_json::Value = if update {
        serde_json::Value::Null
    } else {
        serde_json::from_slice(&fixture("pages/cross-op-labels/facts.json")).unwrap()
    };
    for (name, edit) in &CASES {
        let (bytes, expected) = case_bytes(edit);
        if update {
            std::fs::create_dir_all(&dir).unwrap();
            std::fs::write(dir.join(name), &bytes).unwrap();
            continue;
        }
        assert_eq!(
            bytes,
            fixture(&format!("pages/cross-op-labels/{name}")),
            "{name}: run with UPDATE_GOLDEN=1"
        );
        let reopened = laminapdf_model::Document::open(bytes, OpenOptions::default()).unwrap();
        let ours = serde_json::to_value(labels_of(&reopened)).unwrap();
        let file = &facts["files"][name];
        assert_eq!(
            (&ours, &ours, &ours, true),
            (
                &serde_json::to_value(&expected).unwrap(),
                &file["pdfjs"],
                &file["pdfium"],
                file["qpdf_check"].as_bool().unwrap()
            ),
            "{name}"
        );
    }
}
