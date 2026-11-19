//! Comments to and from other apps through the façade: export, import into a copy and export
//! again gives the same file; each import mode; one undo step; a file for another PDF; what is
//! skipped; Markdown.

#![allow(
    clippy::unwrap_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "test code: a panic is a test failure"
)]

use laminapdf::{
    AddReply, CommentFormat, Document, ExportComments, ExportOptions, ImportMode, ImportOptions, ImportWarning,
    LineEnding, MarkupKind, NoteIcon, Op, OpenOptions, PointPt, Pt, RectPt, ReviewState, SaveOptions, SetReviewState,
    ShapeKind, SkipReason, StampKind, StampName,
};
use laminapdf_test_fixtures::fixture;

fn open(name: &str) -> Document {
    Document::open_bytes(fixture(name), OpenOptions::default()).unwrap()
}

fn pt(x: f64, y: f64) -> PointPt {
    PointPt::new(Pt::new(x), Pt::new(y))
}

/// `hello.pdf` with a comment of each kind an import makes, a reply and a status.
fn reviewed() -> Document {
    let mut doc = open("corpus/hello.pdf");
    let ops: Vec<Op> = vec![
        doc.mark_text(
            0,
            MarkupKind::Highlight,
            &[RectPt::from_points(72.0, 72.0, 120.0, 14.0)],
        )
        .unwrap()
        .into(),
        doc.note_at(0, pt(300.0, 100.0), "Check the total", NoteIcon::Comment)
            .unwrap()
            .into(),
        doc.ink_at(0, &[vec![pt(100.0, 300.0), pt(140.0, 340.0), pt(180.0, 300.0)]], 2.0)
            .unwrap()
            .into(),
        doc.line_at(
            0,
            pt(100.0, 400.0),
            pt(200.0, 450.0),
            [LineEnding::None, LineEnding::OpenArrow],
        )
        .unwrap()
        .into(),
        doc.shape_at(0, ShapeKind::Square, RectPt::from_points(300.0, 300.0, 80.0, 40.0))
            .unwrap()
            .into(),
        doc.stamp_at(
            0,
            RectPt::from_points(300.0, 500.0, 180.0, 50.0),
            StampKind::Standard(StampName::Approved),
        )
        .unwrap()
        .into(),
        doc.attach_file_at(0, pt(40.0, 40.0), "readings.csv", b"day,1\n".to_vec(), "text/csv")
            .unwrap()
            .into(),
    ];
    doc.apply(ops).unwrap();
    let note: laminapdf::AnnotId = doc
        .page_annotations(0)
        .unwrap()
        .into_iter()
        .find(|a| a.subtype == "Text")
        .unwrap()
        .id
        .parse()
        .unwrap();
    doc.apply(vec![
        AddReply::new(note.clone(), "Agreed").into(),
        SetReviewState::new(note, ReviewState::Accepted).into(),
    ])
    .unwrap();
    doc
}

fn export(doc: &Document, format: CommentFormat) -> Vec<u8> {
    let mut options = ExportOptions::new(format);
    options.file = Some("hello.pdf".to_owned());
    doc.export_annotations(&options).unwrap()
}

#[test]
fn exporting_importing_into_a_copy_and_exporting_again_gives_the_same_file() {
    for format in [CommentFormat::Xfdf, CommentFormat::Fdf] {
        let first = export(&reviewed(), format);
        let mut copy = open("corpus/hello.pdf");
        let (summary, _) = copy
            .import_annotations(&first, &ImportOptions::new(ImportMode::Merge))
            .unwrap();
        let again = export(&copy, format);
        assert_eq!(
            (summary.added, String::from_utf8_lossy(&again)),
            (9, String::from_utf8_lossy(&first)),
            "{format:?}"
        );
    }
}

#[test]
fn an_import_is_one_undo_step_and_undoing_it_saves_the_same_bytes() {
    let xfdf = export(&reviewed(), CommentFormat::Xfdf);
    let mut doc = open("corpus/hello.pdf");
    let before = doc.save(&SaveOptions::new()).unwrap();
    doc.import_annotations(&xfdf, &ImportOptions::default()).unwrap();
    doc.undo().unwrap();
    let after = doc.save(&SaveOptions::new()).unwrap();
    doc.redo().unwrap();
    assert_eq!(
        (after == before, doc.page_annotations(0).unwrap().is_empty()),
        (true, false)
    );
}

#[test]
fn merge_finds_what_is_there_replace_deletes_it_first_and_add_makes_copies() {
    let xfdf = export(&reviewed(), CommentFormat::Xfdf);
    let count = |doc: &Document| doc.page_annotations(0).unwrap().len();
    let summarise = |mode| {
        let mut doc = reviewed();
        let (s, _) = doc.import_annotations(&xfdf, &ImportOptions::new(mode)).unwrap();
        (s.added, s.matched, s.deleted, count(&doc))
    };
    let start = count(&reviewed());
    assert_eq!(
        [
            summarise(ImportMode::Merge),
            summarise(ImportMode::Replace),
            summarise(ImportMode::Add)
        ],
        [(0, 9, 0, start), (9, 0, 7, start), (9, 0, 0, start * 2)]
    );
}

#[test]
fn a_preview_says_what_an_import_would_do_and_changes_nothing() {
    let xfdf = export(&reviewed(), CommentFormat::Xfdf);
    let doc = open("corpus/hello.pdf");
    let summary = doc.preview_import(&xfdf, &ImportOptions::default()).unwrap();
    assert_eq!(
        (summary.added, summary.pages, doc.page_annotations(0).unwrap().len()),
        (9, vec![0], 0)
    );
}

#[test]
fn a_file_for_another_pdf_warns_and_what_cant_be_made_is_skipped() {
    let xfdf = String::from_utf8(export(&reviewed(), CommentFormat::Xfdf))
        .unwrap()
        .replace(
            "</annots>",
            "<sound page=\"0\" rect=\"0,0,20,20\" name=\"snd\"/><text page=\"7\" rect=\"0,0,20,20\" name=\"far\"/></annots>",
        );
    let mut other = open("golden/examples/pdf_a_archive-2b.pdf");
    let mut options = ImportOptions::default();
    options.file = Some("archive.pdf".to_owned());
    let (summary, _) = other.import_annotations(xfdf.as_bytes(), &options).unwrap();
    let said: Vec<String> = summary
        .warnings
        .iter()
        .map(|w| match w {
            ImportWarning::Skipped(s) => format!("{} {} {:?}", w.code(), s.kind, s.reason),
            other => other.code().to_owned(),
        })
        .collect();
    assert_eq!(
        said,
        [
            "interchange-document-mismatch".to_owned(),
            format!("interchange-skipped Sound {:?}", SkipReason::Unsupported),
            format!("interchange-skipped Text {:?}", SkipReason::MissingPage),
        ]
    );
}

#[test]
fn markdown_quotes_each_highlight_under_its_pages_heading() {
    let mut options = ExportOptions::new(CommentFormat::Markdown);
    options.comments = ExportComments::HighlightsAndNotes;
    let md = String::from_utf8(open("annotations/review.pdf").export_annotations(&options).unwrap()).unwrap();
    assert!(
        md.starts_with("## Page 1\n\n") && md.contains("\n> Quoted words\n"),
        "{md}"
    );
}

#[test]
fn every_kind_in_review_xfdf_and_review_fdf_imports_and_the_two_agree() {
    let import = |name: &str| {
        let mut doc = open("corpus/hello.pdf");
        let (summary, _) = doc
            .import_annotations(&fixture(&format!("interchange/{name}")), &ImportOptions::default())
            .unwrap();
        let mut kinds: Vec<String> = doc
            .page_annotations(0)
            .unwrap()
            .into_iter()
            .map(|a| a.subtype)
            .collect();
        kinds.sort();
        (summary.added, summary.warnings, kinds)
    };
    let xfdf = import("review.xfdf");
    assert_eq!(
        (xfdf.0, xfdf.1.len(), xfdf.2.len()),
        (16, 0, 18),
        "16 comments and the two Popups the file places: {:?}",
        xfdf.2
    );
    assert_eq!(import("review.fdf"), xfdf);
}

#[test]
fn skipped_xfdf_lists_what_an_import_leaves_out() {
    let mut doc = open("corpus/hello.pdf");
    let (summary, _) = doc
        .import_annotations(&fixture("interchange/skipped.xfdf"), &ImportOptions::default())
        .unwrap();
    let reasons: Vec<(String, SkipReason)> = summary
        .warnings
        .into_iter()
        .filter_map(|w| match w {
            ImportWarning::Skipped(s) => Some((s.kind, s.reason)),
            _ => None,
        })
        .collect();
    assert_eq!(
        (summary.added, reasons),
        (
            1,
            vec![
                ("Sound".to_owned(), SkipReason::Unsupported),
                ("Link".to_owned(), SkipReason::Unsupported),
                ("field".to_owned(), SkipReason::FormField),
                ("field".to_owned(), SkipReason::FormField),
                ("Text".to_owned(), SkipReason::MissingPage),
                ("Text".to_owned(), SkipReason::MissingParent),
            ]
        )
    );
}

#[test]
fn a_dtd_is_refused_a_malformed_file_names_its_line_and_another_documents_file_warns() {
    let mut doc = open("golden/examples/pdf_a_archive-2b.pdf");
    let dtd = doc.import_annotations(&fixture("interchange/dtd.xfdf"), &ImportOptions::default());
    let malformed = doc.import_annotations(&fixture("interchange/malformed.xfdf"), &ImportOptions::default());
    let other = doc
        .preview_import(&fixture("interchange/other-document.xfdf"), &ImportOptions::default())
        .unwrap();
    assert_eq!(
        (
            dtd.unwrap_err().code(),
            malformed.unwrap_err().to_string(),
            other.warnings.first().map(ImportWarning::code),
            doc.page_annotations(0).unwrap().len()
        ),
        (
            "interchange-dtd",
            "the file isn't valid XFDF near line 5: it isn't well-formed XML".to_owned(),
            Some("interchange-document-mismatch"),
            0
        )
    );
}

/// `actual` against the golden `name` in `fixtures/interchange/`, which `UPDATE_GOLDEN` writes.
fn golden(name: &str, actual: &[u8]) {
    let path = laminapdf_test_fixtures::fixtures_dir().join("interchange").join(name);
    if std::env::var_os("UPDATE_GOLDEN").is_some() {
        std::fs::write(&path, actual).unwrap();
    }
    let expected = std::fs::read(&path).unwrap_or_default();
    assert!(
        actual == expected.as_slice(),
        "fixtures/interchange/{name} is stale: run with UPDATE_GOLDEN=1"
    );
}

#[test]
fn review_pdf_exports_as_its_goldens() {
    let doc = open("annotations/review.pdf");
    for (format, name) in [
        (CommentFormat::Xfdf, "review-pdf.xfdf"),
        (CommentFormat::Fdf, "review-pdf.fdf"),
    ] {
        let mut options = ExportOptions::new(format);
        options.file = Some("review.pdf".to_owned());
        golden(name, &doc.export_annotations(&options).unwrap());
    }
}

#[test]
fn review_pdf_through_an_import_and_back_is_the_same_file_from_the_first_import_on() {
    let mut options = ExportOptions::new(CommentFormat::Xfdf);
    options.file = Some("review.pdf".to_owned());
    // Replace takes every comment off the pages the file names, then imports the file's.
    let cycle = |xfdf: &[u8]| {
        let mut doc = open("annotations/review.pdf");
        let (summary, _) = doc
            .import_annotations(xfdf, &ImportOptions::new(ImportMode::Replace))
            .unwrap();
        (doc.export_annotations(&options).unwrap(), summary)
    };
    let first = open("annotations/review.pdf").export_annotations(&options).unwrap();
    let (second, summary) = cycle(&first);
    let (third, _) = cycle(&second);
    // Two replies that answer each other can't be made with ops, each needing the other first:
    // the first import skips them and says so. The replies it makes write /RT /R and /M, which
    // review.pdf, written by hand, leaves out; from then on the file stays the same.
    let skipped: Vec<SkipReason> = summary
        .warnings
        .iter()
        .filter_map(|w| match w {
            ImportWarning::Skipped(s) => Some(s.reason),
            _ => None,
        })
        .collect();
    assert_eq!(
        (String::from_utf8_lossy(&third), skipped),
        (String::from_utf8_lossy(&second), vec![SkipReason::MissingParent; 2])
    );
}
