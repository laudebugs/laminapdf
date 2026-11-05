//! Headings, list items and page furniture (ADR 0030 §3–§4) on files built for each rule:
//! sizes and bold for headings, bullets and numbers with hanging indents for lists, and
//! running headers, footers, page numbers and artifacts on a five-page file.
#![allow(clippy::unwrap_used, clippy::indexing_slicing, reason = "test code")]

use laminapdf_build::{ArtifactKind, BuildOptions, Builder, PageSize, Standard14};
use laminapdf_extract::{BlockKind, DropReason, ExtractOptions, Extraction, extract};
use laminapdf_model::{Document, OpenOptions, PointPt, Pt};
use laminapdf_save::{Save as _, SaveOptions};
use laminapdf_syntax::CancelToken;

/// One line to draw: text, x, baseline, size, bold, inside a pagination artifact.
type Line<'a> = (&'a str, f64, f64, f64, bool, bool);

/// A document with one A4 page per entry of `pages`, each drawing its lines in Helvetica.
fn build(pages: &[Vec<Line<'_>>]) -> Document {
    let mut b = Builder::new(BuildOptions::default());
    let regular = b.standard_font(Standard14::Helvetica).unwrap();
    let bold = b.standard_font(Standard14::HelveticaBold).unwrap();
    for lines in pages {
        let mut page = b.page(PageSize::A4);
        let s = page.surface();
        for &(text, x, y, size, heavy, artifact) in lines {
            let font = if heavy { &bold } else { &regular };
            let at = PointPt::from_points(x, y);
            if artifact {
                s.artifact(ArtifactKind::Pagination, |s| {
                    s.text(text, font, Pt::new(size), at).map(|_| ())
                })
                .unwrap();
            } else {
                s.text(text, font, Pt::new(size), at).unwrap();
            }
        }
        page.finish().unwrap();
    }
    let mut doc = b.finish().unwrap();
    let mut bytes = Vec::new();
    doc.save(&mut bytes, &SaveOptions::new()).unwrap();
    Document::open(bytes, OpenOptions::default()).unwrap()
}

fn run(doc: &Document, options: &ExtractOptions) -> Extraction {
    extract(doc, options, &CancelToken::new()).unwrap()
}

fn kinds(out: &Extraction) -> Vec<(BlockKind, &str)> {
    out.blocks.iter().map(|b| (b.kind.clone(), b.text.as_str())).collect()
}

fn item(marker: &str, depth: u8) -> BlockKind {
    BlockKind::ListItem {
        marker: marker.to_owned(),
        depth,
    }
}

const BODY: &str = "Body text set at the size most of the page uses.";

#[test]
fn headings_list_items_and_paragraphs_get_their_kinds() {
    let page = vec![
        ("Annual report", 72.0, 80.0, 24.0, false, false),
        ("Results", 72.0, 130.0, 16.0, false, false),
        (BODY, 72.0, 160.0, 11.0, false, false),
        ("Regional notes", 72.0, 200.0, 11.0, true, false),
        (BODY, 72.0, 225.0, 11.0, false, false),
        // A bullet a tab from its text, the item's second line hanging under the text.
        ("•", 72.0, 260.0, 11.0, false, false),
        ("Apples sold well in the north", 86.0, 260.0, 11.0, false, false),
        ("and in the south.", 86.0, 273.0, 11.0, false, false),
        ("–", 100.0, 290.0, 11.0, false, false),
        ("Green ones most.", 114.0, 290.0, 11.0, false, false),
        ("•", 72.0, 307.0, 11.0, false, false),
        ("Pears did not.", 86.0, 307.0, 11.0, false, false),
        ("1. Sow early.", 72.0, 340.0, 11.0, false, false),
        ("2. Water often.", 72.0, 355.0, 11.0, false, false),
    ];
    let out = run(&build(&[page]), &ExtractOptions::new());
    assert_eq!(
        kinds(&out),
        [
            (BlockKind::Heading { level: 1 }, "Annual report"),
            (BlockKind::Heading { level: 2 }, "Results"),
            (BlockKind::Paragraph, BODY),
            (BlockKind::Heading { level: 3 }, "Regional notes"),
            (BlockKind::Paragraph, BODY),
            (item("•", 0), "Apples sold well in the north and in the south."),
            (item("–", 1), "Green ones most."),
            (item("•", 0), "Pears did not."),
            (item("1.", 0), "Sow early."),
            (item("2.", 0), "Water often."),
        ]
    );
    let ids: Vec<&str> = out.blocks.iter().map(|b| b.id.as_str()).collect();
    assert_eq!(ids.first(), Some(&"p0-b0"));
    assert_eq!(ids.last(), Some(&"p0-b9"));
}

/// Five pages: a running header with the chapter number, a footer, a page number and a
/// pagination artifact, around one paragraph.
fn five_pages() -> Document {
    let numbers = ["1", "2", "3", "4", "5"];
    let headers = [
        "Chapter 1 — Growing",
        "Chapter 1 — Growing",
        "Chapter 2 — Growing",
        "Chapter 2 — Growing",
        "Chapter 3 — Growing",
    ];
    let pages: Vec<Vec<Line<'_>>> = numbers
        .iter()
        .zip(headers)
        .map(|(&n, header)| {
            vec![
                (header, 72.0, 40.0, 9.0, false, false),
                ("Marked as pagination", 400.0, 40.0, 9.0, false, true),
                (BODY, 72.0, 200.0, 11.0, false, false),
                ("Lamina Orchards Ltd.", 72.0, 790.0, 9.0, false, false),
                (n, 290.0, 815.0, 9.0, false, false),
            ]
        })
        .collect();
    build(&pages)
}

#[test]
fn running_headers_footers_page_numbers_and_artifacts_move_to_dropped() {
    let out = run(&five_pages(), &ExtractOptions::new());
    assert_eq!(out.blocks.len(), 5);
    assert!(
        out.blocks
            .iter()
            .all(|b| b.text == BODY && b.kind == BlockKind::Paragraph)
    );
    for page in 0..5 {
        let mut reasons: Vec<DropReason> = out
            .dropped
            .iter()
            .filter(|d| d.page == page)
            .map(|d| d.reason)
            .collect();
        reasons.sort();
        assert_eq!(
            reasons,
            [
                DropReason::Artifact,
                DropReason::RunningHeader,
                DropReason::RunningFooter,
                DropReason::PageNumber
            ],
            "page {page}"
        );
    }
    let pages: Vec<usize> = out.dropped.iter().map(|d| d.page).collect();
    assert!(pages.is_sorted(), "dropped in page order: {pages:?}");
    assert!(out.dropped.iter().all(|d| d.chars > 0 && d.spans.is_empty()));
}

#[test]
fn turning_furniture_off_keeps_it_in_the_blocks() {
    let out = run(&five_pages(), &ExtractOptions::new().with_headers_footers(false));
    assert_eq!(out.dropped.len(), 0);
    let first: Vec<&str> = out
        .blocks
        .iter()
        .filter(|b| b.page == 0)
        .map(|b| b.text.as_str())
        .collect();
    assert!(first.contains(&"Lamina Orchards Ltd.") && first.contains(&"1") && first.contains(&"Marked as pagination"));
}

#[test]
fn a_line_on_two_pages_is_not_a_running_header() {
    let page = |header| {
        vec![
            (header, 72.0, 40.0, 9.0, false, false),
            (BODY, 72.0, 200.0, 11.0, false, false),
        ]
    };
    let out = run(
        &build(&[page("Same top line"), page("Same top line")]),
        &ExtractOptions::new(),
    );
    assert_eq!(out.blocks.len(), 4);
    assert_eq!(out.dropped.len(), 0);
}
