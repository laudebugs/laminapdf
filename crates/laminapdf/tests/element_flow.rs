//! The element-flow example's placements, read back from the PDF it writes. Each expectation is
//! worked out from Helvetica's AFM metrics (ascender 718, descender -207 per 1000 units) and the
//! example's layout numbers, not from the builder, so a wrong bottom or a lost gap fails.
#![expect(
    clippy::panic,
    clippy::unwrap_used,
    clippy::indexing_slicing,
    reason = "tests: a panic is a test failure"
)]

mod common;
#[expect(dead_code, reason = "the example's own `main` is not used here")]
#[path = "../examples/element_flow.rs"]
mod element_flow;

use common::FlowReport;
use element_flow::Report;
use laminapdf::extract::TextOptions;
use laminapdf::{BuildOptions, Document, OpenOptions};
use laminapdf_test_fixtures::fixture;

/// The example's layout numbers, restated.
const MARGIN: f64 = 48.0;
const GAP: f64 = 10.0;
const TITLE: f64 = 18.0;
const BODY: f64 = 11.0;
/// A5 in points: 148 × 210 mm.
const PAGE_WIDTH: f64 = 148.0 * 72.0 / 25.4;
const PAGE_HEIGHT: f64 = 210.0 * 72.0 / 25.4;
const PHOTO_HEIGHT: f64 = (PAGE_WIDTH - 2.0 * MARGIN) / 1.5;
/// A text box's default line height, at the body size.
const LINE: f64 = 1.2 * BODY;
/// Content streams round positions, so placements read back agree to this.
const TOLERANCE: f64 = 0.01;

fn ascent(size: f64) -> f64 {
    718.0 * size / 1000.0
}

/// The bottom of a text box at `top` whose `lines` lines are `size` points, from the first
/// line's ascent to the last line's descent.
fn bottom(top: f64, lines: u32, size: f64) -> f64 {
    top + 925.0 * size / 1000.0 + f64::from(lines - 1) * 1.2 * size
}

/// The fixture's report, with `edit` applied to it.
fn report_pdf(edit: impl FnOnce(&mut FlowReport)) -> Vec<u8> {
    let mut report = FlowReport::load();
    edit(&mut report);
    let options = BuildOptions::new().with_id(*b"lamina-ex-flow!!");
    element_flow::build(options, fixture("images/photo-rgb.jpg"), &report.as_report()).unwrap()
}

/// Each page's runs: their text and their baseline in page space (y down from the top).
fn runs(pdf: Vec<u8>) -> Vec<Vec<(String, f64)>> {
    let doc = Document::open_bytes(pdf, OpenOptions::new()).unwrap();
    doc.page_texts(&TextOptions::default())
        .unwrap()
        .into_iter()
        .map(|page| {
            let baseline = |first: usize| PAGE_HEIGHT - page.glyphs[first].origin.y;
            page.runs
                .iter()
                .map(|run| (run.text.clone(), baseline(run.glyphs.start)))
                .collect()
        })
        .collect()
}

/// The baseline of the first run on `page` whose text starts with `start`.
fn baseline(pages: &[Vec<(String, f64)>], page: usize, start: &str) -> f64 {
    pages[page]
        .iter()
        .find(|(text, _)| text.starts_with(start))
        .unwrap_or_else(|| panic!("no run starting {start:?} on page {page}"))
        .1
}

fn close(actual: f64, expected: f64) {
    assert!((actual - expected).abs() < TOLERANCE, "{actual} vs {expected}");
}

#[test]
fn each_element_starts_a_gap_below_where_the_one_before_ended() {
    let pages = runs(report_pdf(|_| {}));
    let title_top = MARGIN;
    close(baseline(&pages, 0, "Third-quarter"), title_top + ascent(TITLE));
    // The fixture's summary and notes each wrap onto two lines.
    let summary_top = bottom(title_top, 1, TITLE) + GAP;
    close(baseline(&pages, 0, "Revenue"), summary_top + ascent(BODY));
    let notes_top = bottom(summary_top, 2, BODY) + GAP;
    close(baseline(&pages, 0, "Figures"), notes_top + ascent(BODY));
    // The photo is placed by the caller's own rectangle, and the body keeps its gap below it.
    let photo_top = bottom(notes_top, 2, BODY) + GAP;
    close(
        baseline(&pages, 0, "The reader"),
        photo_top + PHOTO_HEIGHT + GAP + ascent(BODY),
    );
}

#[test]
fn a_summary_that_wraps_onto_another_line_moves_everything_after_it_by_one_line() {
    let before = runs(report_pdf(|_| {}));
    let longer = "Revenue rose twelve per cent over the quarter, led by the new reader and its offline mode, \
                  which readers on trains and planes now use every week.";
    let after = runs(report_pdf(|r| r.summary = longer.to_owned()));
    for start in ["Figures", "The reader"] {
        close(baseline(&after, 0, start) - baseline(&before, 0, start), LINE);
    }
}

#[test]
fn empty_notes_leave_no_gap() {
    let pages = runs(report_pdf(|r| r.notes.clear()));
    assert!(pages[0].iter().all(|(text, _)| !text.starts_with("Figures")));
    let summary_top = bottom(MARGIN, 1, TITLE) + GAP;
    let photo_top = bottom(summary_top, 2, BODY) + GAP;
    close(
        baseline(&pages, 0, "The reader"),
        photo_top + PHOTO_HEIGHT + GAP + ascent(BODY),
    );
}

#[test]
fn the_body_continues_at_the_top_margin_of_a_new_page_and_loses_no_text() {
    let pages = runs(report_pdf(|_| {}));
    assert_eq!(pages.len(), 2);
    let (first, _) = &pages[1][0];
    assert!(!first.starts_with(' '), "{first:?}");
    close(pages[1][0].1, MARGIN + ascent(BODY));
    let words = |text: &str| text.split_whitespace().map(str::to_owned).collect::<Vec<_>>();
    let drawn: Vec<String> = pages.iter().flatten().flat_map(|(text, _)| words(text)).collect();
    let body = FlowReport::load().body;
    assert!(
        drawn.ends_with(&words(&body)),
        "the body's words, in order, end the report"
    );
}

#[test]
fn a_header_too_long_for_the_first_page_fails_instead_of_dropping_text() {
    let long = "Revenue rose twelve per cent over the quarter. ".repeat(80);
    let options = BuildOptions::new();
    let report = Report {
        title: "Report",
        summary: &long,
        notes: "",
        body: "",
    };
    let error = element_flow::build(options, fixture("images/photo-rgb.jpg"), &report).unwrap_err();
    let error = error.downcast_ref::<laminapdf::BuildError>().unwrap();
    assert_eq!(error.code(), "text-overflow");
}

#[test]
fn blank_lines_at_the_end_of_the_body_add_no_page() {
    // More blank lines than a page holds: without trimming they would overflow onto a blank page.
    let pages = runs(report_pdf(|r| r.body.push_str(&"\n".repeat(80))));
    assert_eq!(pages.len(), 2);
}

#[test]
fn a_photo_that_would_cross_the_bottom_margin_fails() {
    // About twenty lines of summary: it fits its box, but leaves less room than the photo needs.
    let long = "Revenue rose twelve per cent over the quarter. ".repeat(26);
    let report = Report {
        title: "Report",
        summary: &long,
        notes: "",
        body: "",
    };
    let built = element_flow::build(BuildOptions::new(), fixture("images/photo-rgb.jpg"), &report);
    let Err(error) = built else {
        panic!("the photo was placed past the bottom margin")
    };
    assert_eq!(error.to_string(), "the photo doesn't fit below the notes");
}
