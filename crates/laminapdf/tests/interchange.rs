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
    AddReply, CommentFormat, ConstraintKind, Document, ExportComments, ExportCommentsOptions, ImportCommentsOptions,
    ImportMode, ImportWarning, LineEnding, MarkupKind, NoteIcon, Op, OpenOptions, PointPt, Pt, RectPt, ReviewState,
    SaveOptions, SetReviewState, ShapeKind, SkipCause, StampKind, StampName,
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
    let mut options = ExportCommentsOptions::new(format);
    options.file = Some("hello.pdf".to_owned());
    doc.export_annotations(&options).unwrap()
}

#[test]
fn exporting_importing_into_a_copy_and_exporting_again_gives_the_same_file() {
    for format in [CommentFormat::Xfdf, CommentFormat::Fdf] {
        let first = export(&reviewed(), format);
        let mut copy = open("corpus/hello.pdf");
        let (summary, _) = copy
            .import_annotations(&first, &ImportCommentsOptions::new(ImportMode::Merge))
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
fn an_import_is_one_undo_step_undoing_it_saves_the_same_bytes_and_redoing_it_the_imports() {
    let xfdf = export(&reviewed(), CommentFormat::Xfdf);
    let save = |doc: &mut Document| doc.save(&SaveOptions::new()).unwrap().bytes;
    let before = save(&mut open("corpus/hello.pdf"));
    let mut imported = open("corpus/hello.pdf");
    imported
        .import_annotations(&xfdf, &ImportCommentsOptions::default())
        .unwrap();
    // Undone: nothing left to save, and the save is the file as it was.
    let mut undone = open("corpus/hello.pdf");
    undone
        .import_annotations(&xfdf, &ImportCommentsOptions::default())
        .unwrap();
    undone.undo().unwrap();
    let after_undo = (undone.has_unsaved_changes(), save(&mut undone) == before);
    // Redone: the same bytes as the import saved.
    let mut redone = open("corpus/hello.pdf");
    redone
        .import_annotations(&xfdf, &ImportCommentsOptions::default())
        .unwrap();
    redone.undo().unwrap();
    redone.redo().unwrap();
    assert_eq!(
        (after_undo, save(&mut redone) == save(&mut imported)),
        ((false, true), true)
    );
}

#[test]
fn merge_finds_what_is_there_replace_deletes_it_first_and_add_makes_copies() {
    let xfdf = export(&reviewed(), CommentFormat::Xfdf);
    let count = |doc: &Document| doc.page_annotations(0).unwrap().len();
    let summarise = |mode| {
        let mut doc = reviewed();
        let (s, _) = doc
            .import_annotations(&xfdf, &ImportCommentsOptions::new(mode))
            .unwrap();
        (s.added, s.matched, s.deleted, count(&doc))
    };
    let start = count(&reviewed());
    assert_eq!(
        [
            summarise(ImportMode::Merge),
            summarise(ImportMode::Replace),
            summarise(ImportMode::Add)
        ],
        [(0, 9, 0, start), (9, 9, 9, start), (9, 9, 0, start * 2)]
    );
}

#[test]
fn a_preview_says_what_an_import_would_do_and_changes_nothing() {
    let xfdf = export(&reviewed(), CommentFormat::Xfdf);
    let doc = open("corpus/hello.pdf");
    let summary = doc.preview_import(&xfdf, &ImportCommentsOptions::default()).unwrap();
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
    let mut options = ImportCommentsOptions::default();
    options.file = Some("archive.pdf".to_owned());
    let (summary, _) = other.import_annotations(xfdf.as_bytes(), &options).unwrap();
    let said: Vec<String> = summary
        .warnings
        .iter()
        .map(|w| match w {
            ImportWarning::Skipped(s) => format!("{} {} {:?}", w.code(), s.kind, s.cause),
            other => other.code().to_owned(),
        })
        .collect();
    assert_eq!(
        said,
        [
            "interchange-document-mismatch".to_owned(),
            format!("interchange-skipped Sound {:?}", SkipCause::Unsupported),
            format!("interchange-skipped Text {:?}", SkipCause::MissingPage),
        ]
    );
}

#[test]
fn markdown_quotes_each_highlight_under_its_pages_heading() {
    let mut options = ExportCommentsOptions::new(CommentFormat::Markdown);
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
            .import_annotations(
                &fixture(&format!("interchange/{name}")),
                &ImportCommentsOptions::default(),
            )
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
        .import_annotations(&fixture("interchange/skipped.xfdf"), &ImportCommentsOptions::default())
        .unwrap();
    let reasons: Vec<(String, SkipCause)> = summary
        .warnings
        .into_iter()
        .filter_map(|w| match w {
            ImportWarning::Skipped(s) => Some((s.kind, s.cause)),
            _ => None,
        })
        .collect();
    assert_eq!(
        (summary.added, reasons),
        (
            1,
            vec![
                ("Sound".to_owned(), SkipCause::Unsupported),
                ("Link".to_owned(), SkipCause::Unsupported),
                ("field".to_owned(), SkipCause::FormField),
                ("field".to_owned(), SkipCause::FormField),
                ("Text".to_owned(), SkipCause::MissingPage),
                ("Text".to_owned(), SkipCause::MissingParent),
            ]
        )
    );
}

#[test]
fn a_dtd_is_refused_a_malformed_file_names_its_line_and_another_documents_file_warns() {
    let mut doc = open("golden/examples/pdf_a_archive-2b.pdf");
    let dtd = doc.import_annotations(&fixture("interchange/dtd.xfdf"), &ImportCommentsOptions::default());
    let malformed = doc.import_annotations(
        &fixture("interchange/malformed.xfdf"),
        &ImportCommentsOptions::default(),
    );
    let other = doc
        .preview_import(
            &fixture("interchange/other-document.xfdf"),
            &ImportCommentsOptions::default(),
        )
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
        let mut options = ExportCommentsOptions::new(format);
        options.file = Some("review.pdf".to_owned());
        golden(name, &doc.export_annotations(&options).unwrap());
    }
}

#[test]
fn review_pdf_imported_into_a_copy_without_comments_is_the_same_file_from_the_first_import_on() {
    let mut options = ExportCommentsOptions::new(CommentFormat::Xfdf);
    options.file = Some("review.pdf".to_owned());
    // review.pdf with every comment deleted.
    let blank = || {
        let mut doc = open("annotations/review.pdf");
        let ids: Vec<laminapdf::AnnotId> = (0..2)
            .flat_map(|page| doc.page_annotations(page).unwrap())
            .map(|a| a.id.parse().unwrap())
            .collect();
        doc.delete_annotations(&ids).unwrap();
        doc
    };
    let cycle = |xfdf: &[u8]| {
        let mut doc = blank();
        let (summary, _) = doc.import_annotations(xfdf, &ImportCommentsOptions::default()).unwrap();
        (doc.export_annotations(&options).unwrap(), summary)
    };
    let first = open("annotations/review.pdf").export_annotations(&options).unwrap();
    let (second, summary) = cycle(&first);
    let (third, _) = cycle(&second);
    // Two replies that answer each other can't be made with ops, each needing the other first:
    // the first import skips them and says so. The replies it makes write /RT /R and /M, which
    // review.pdf, written by hand, leaves out; from then on the file stays the same.
    let skipped: Vec<SkipCause> = summary
        .warnings
        .iter()
        .filter_map(|w| match w {
            ImportWarning::Skipped(s) => Some(s.cause),
            _ => None,
        })
        .collect();
    assert_eq!(
        (String::from_utf8_lossy(&third), skipped),
        (String::from_utf8_lossy(&second), vec![SkipCause::MissingParent; 2])
    );
}

#[test]
fn for_every_prefix_of_review_xfdf_undoing_the_import_saves_the_file_as_it_was_and_redoing_it_as_imported() {
    let full = laminapdf_interchange::read(&fixture("interchange/review.xfdf"), &laminapdf::Limits::default()).unwrap();
    let save = |doc: &mut Document| doc.save(&SaveOptions::new()).unwrap().bytes;
    let before = save(&mut open("corpus/hello.pdf"));
    for n in 1..=full.comments.len() {
        let mut part = full.clone();
        part.comments.truncate(n);
        let xfdf = laminapdf_interchange::write_xfdf(&part);
        let mut imported = open("corpus/hello.pdf");
        imported
            .import_annotations(&xfdf, &ImportCommentsOptions::default())
            .unwrap();
        let mut undone = open("corpus/hello.pdf");
        undone
            .import_annotations(&xfdf, &ImportCommentsOptions::default())
            .unwrap();
        undone.undo().unwrap();
        let after_undo = (undone.has_unsaved_changes(), save(&mut undone) == before);
        let mut redone = open("corpus/hello.pdf");
        redone
            .import_annotations(&xfdf, &ImportCommentsOptions::default())
            .unwrap();
        redone.undo().unwrap();
        redone.redo().unwrap();
        assert_eq!(
            (after_undo, save(&mut redone) == save(&mut imported)),
            ((false, true), true),
            "{n} comments"
        );
    }
}

#[test]
fn on_a_certified_file_an_import_is_refused_by_the_save_plan_unless_annotations_are_allowed() {
    let xfdf = fixture("interchange/review.xfdf");
    let verdict = |name: &str| {
        let mut doc = open(&format!("signed/{name}"));
        doc.import_annotations(&xfdf, &ImportCommentsOptions::default())
            .unwrap();
        let plan = doc.plan(&SaveOptions::new()).unwrap();
        (plan.constraint, plan.violations.is_empty())
    };
    // DocMDP P1 allows no change; P3 allows comments (ISO 32000-2 §12.8.2.2).
    assert_eq!(
        [verdict("certified-p1.pdf"), verdict("certified-p3.pdf")],
        [(ConstraintKind::Conflict, false), (ConstraintKind::AppendOnly, true)]
    );
}

#[test]
fn a_reply_listed_before_the_comment_it_answers_joins_the_replacement_and_a_copy() {
    let note = r#"<text page="0" rect="72,700,92,720" name="n1"><contents>Note</contents></text>"#;
    let reply = r#"<text page="0" rect="72,700,92,720" name="r1" inreplyto="n1"><contents>Reply</contents></text>"#;
    let xfdf = |annots: &str| format!(r#"<xfdf xmlns="http://ns.adobe.com/xfdf/"><annots>{annots}</annots></xfdf>"#);
    let answered = |mode, parent: &str| {
        let mut doc = open("corpus/hello.pdf");
        doc.import_annotations(xfdf(note).as_bytes(), &ImportCommentsOptions::default())
            .unwrap();
        let (summary, _) = doc
            .import_annotations(
                xfdf(&format!("{reply}{note}")).as_bytes(),
                &ImportCommentsOptions::new(mode),
            )
            .unwrap();
        let annots = doc.page_annotations(0).unwrap();
        let named = |n: &str| {
            annots
                .iter()
                .rev()
                .find(|a| a.name.as_deref() == Some(n))
                .map(|a| a.id.clone())
        };
        let reply_parent = annots
            .iter()
            .find(|a| a.contents.as_deref() == Some("Reply"))
            .and_then(|a| a.in_reply_to.clone());
        (summary.added, summary.warnings.len(), reply_parent == named(parent))
    };
    // Replace: the reply answers the new n1; Add: the copy, n1-2, not the original.
    assert_eq!(
        [answered(ImportMode::Replace, "n1"), answered(ImportMode::Add, "n1-2")],
        [(2, 0, true), (2, 0, true)]
    );
}
