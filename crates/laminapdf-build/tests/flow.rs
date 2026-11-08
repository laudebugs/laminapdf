//! Flowing layout (todo T4b): paragraphs continue across pages and read back as written, widows
//! and orphans, a heading kept with the next block, keep groups and figures, the two codes, the
//! structure across pages, the header and footer artifacts and their fields.
#![allow(
    clippy::unwrap_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "tests: a panic is a test failure"
)]

mod common;

use common::{reopen, save};
use laminapdf_build::flow::{
    Figure, Flow, FlowBlock, Margins, PageRegion, PageTemplate, Paragraph, ParagraphStyle, TextStyle,
};
use laminapdf_build::{BuildOptions, Builder, Error, FontHandle, Image, PageSize, Warning};
use laminapdf_model::{PageId, StructKid};
use laminapdf_test_fixtures::fixture;
use laminapdf_text::{Font, TextExtractor};

const SIZE: f64 = 10.0;

fn noto(b: &mut Builder) -> FontHandle {
    b.font(Font::from_bytes(fixture("fonts/NotoSans-Regular.ttf")).unwrap())
        .unwrap()
}

/// A line's height at 10 pt in Noto Sans: its ascent to its descent, taller than 1.2 × 10.
fn line_height(font: &FontHandle) -> f64 {
    let info = font.info();
    f64::from(info.ascent - info.descent) * SIZE / f64::from(info.units_per_em)
}

/// A page whose body is `lines` lines of 10 pt text tall, 300 points wide.
fn template(font: &FontHandle, lines: f64) -> PageTemplate {
    let height = 100.0 + lines * line_height(font) + 0.01;
    PageTemplate::new(PageSize::new(
        laminapdf_model::Pt::new(400.0),
        laminapdf_model::Pt::new(height),
    ))
    .with_margins(Margins::new(50.0, 50.0, 50.0, 50.0))
}

/// `n` lines of words, each line ending in a hard break so its count is exact.
fn lines(n: usize, style: &TextStyle) -> Paragraph {
    let mut p = Paragraph::new(ParagraphStyle::new().with_space(0.0, 0.0));
    for i in 0..n {
        p = p.text(format!("Line {i} of the paragraph."), style);
        if i + 1 < n {
            p = p.line_break();
        }
    }
    p
}

fn texts(bytes: &[u8]) -> Vec<Vec<String>> {
    let doc = reopen(bytes);
    let mut extractor = TextExtractor::new();
    doc.pages()
        .unwrap()
        .iter()
        .map(|page| extractor.page(&doc, &page).unwrap().lines())
        .collect()
}

fn build(flow_of: impl FnOnce(&FontHandle) -> Flow) -> (Vec<u8>, Vec<Warning>) {
    let mut b = Builder::new(BuildOptions::new().with_id(*b"lamina-flow-test").with_compress(false));
    let font = noto(&mut b);
    let report = b.flow(&flow_of(&font)).unwrap();
    (save(b.finish().unwrap()), report.warnings)
}

#[test]
fn a_paragraph_continues_on_the_next_page_and_reads_back_as_written() {
    let words = "Every quarter the team measured how fast the reader opened files, and it got faster. ";
    let text = words.repeat(12);
    let (bytes, _) = build(|font| {
        let style = TextStyle::new(font, SIZE);
        Flow::new(template(font, 6.0)).block(FlowBlock::Paragraph(Paragraph::plain(text.trim_end(), &style)))
    });
    let pages = texts(&bytes);
    assert!(pages.len() > 1);
    assert_eq!(pages.concat().join(" "), text.trim_end());
}

#[test]
fn one_line_left_moves_the_whole_paragraph_to_the_next_page() {
    let (bytes, _) = build(|font| {
        let style = TextStyle::new(font, SIZE);
        Flow::new(template(font, 10.0))
            .block(FlowBlock::Paragraph(lines(9, &style)))
            .block(FlowBlock::Paragraph(lines(10, &style)))
    });
    let counts: Vec<usize> = texts(&bytes).iter().map(Vec::len).collect();
    assert_eq!(counts, [9, 10]);
}

#[test]
fn nine_lines_left_split_a_ten_line_paragraph_eight_and_two() {
    let (bytes, _) = build(|font| {
        let style = TextStyle::new(font, SIZE);
        Flow::new(template(font, 10.0))
            .block(FlowBlock::Paragraph(lines(1, &style)))
            .block(FlowBlock::Paragraph(lines(10, &style)))
    });
    let counts: Vec<usize> = texts(&bytes).iter().map(Vec::len).collect();
    assert_eq!(counts, [9, 2]);
}

#[test]
fn a_heading_with_room_for_one_more_line_moves_with_its_paragraph() {
    let (bytes, _) = build(|font| {
        let style = TextStyle::new(font, SIZE);
        Flow::new(template(font, 10.0))
            .block(FlowBlock::Paragraph(lines(8, &style)))
            .block(FlowBlock::Heading(
                1,
                Paragraph::plain("Results", &style).with_style(ParagraphStyle::new().with_space(0.0, 0.0)),
            ))
            .block(FlowBlock::Paragraph(lines(4, &style)))
    });
    let pages = texts(&bytes);
    assert_eq!(pages[0].len(), 8);
    assert_eq!(pages[1][0], "Results");
}

#[test]
fn a_keep_group_taller_than_a_page_is_split_with_a_warning() {
    let (bytes, warnings) = build(|font| {
        let style = TextStyle::new(font, SIZE);
        let group = vec![
            FlowBlock::Paragraph(lines(6, &style)),
            FlowBlock::Paragraph(lines(6, &style)),
        ];
        Flow::new(template(font, 10.0))
            .block(FlowBlock::Paragraph(lines(2, &style)))
            .block(FlowBlock::Keep(group))
    });
    assert_eq!(warnings, [Warning::FlowKeepBroken { page: 1 }]);
    assert_eq!(texts(&bytes).concat().len(), 14);
}

#[test]
fn a_figure_taller_than_the_body_fails() {
    let mut b = Builder::new(BuildOptions::new());
    let font = noto(&mut b);
    let image = Image::rgba(1, 10, &[0; 40]).unwrap();
    let flow = Flow::new(template(&font, 5.0)).block(FlowBlock::Figure(Figure::new(image, "A tall bar")));
    assert_eq!(
        b.flow(&flow).unwrap_err(),
        Error::FlowContentTooLarge { what: "figure" }
    );
}

#[test]
fn a_split_paragraph_is_one_element_with_content_on_both_pages() {
    let (bytes, _) = build(|font| {
        let style = TextStyle::new(font, SIZE);
        Flow::new(template(font, 10.0)).block(FlowBlock::Paragraph(lines(14, &style)))
    });
    let doc = reopen(&bytes);
    let pages: Vec<PageId> = doc.pages().unwrap().iter().map(|p| p.id().clone()).collect();
    let tree = doc.structure().unwrap().unwrap();
    let paragraphs: Vec<_> = tree.elements().into_iter().filter(|e| e.kind == b"P").collect();
    assert_eq!(paragraphs.len(), 1);
    let on: Vec<usize> = paragraphs[0]
        .kids
        .iter()
        .filter_map(|k| match k {
            StructKid::Mcid { page, .. } => pages.iter().position(|p| Some(p) == page.as_ref()),
            _ => None,
        })
        .collect();
    assert_eq!(on, [0, 1]);
}

#[test]
fn headers_and_footers_carry_their_fields_and_subtypes() {
    let (bytes, _) = build(|font| {
        let style = TextStyle::new(font, SIZE);
        let small = style.clone().with_size(8.0);
        let header = PageRegion::new().with_end("{section}");
        let footer = PageRegion::new().with_center("Page {page} of {pages}");
        let pages = template(font, 10.0).with_furniture(Some(header), Some(footer), &small);
        let cover = template(font, 10.0);
        Flow::new(pages.with_first_page(cover))
            .with_title("Report")
            .block(FlowBlock::Paragraph(lines(10, &style)))
            .block(FlowBlock::Heading(1, Paragraph::plain("Findings", &style)))
            .block(FlowBlock::Paragraph(lines(14, &style)))
    });
    let pages = texts(&bytes);
    assert_eq!(pages.len(), 3);
    assert!(
        !pages[0].iter().any(|l| l.starts_with("Page")),
        "the cover has no footer"
    );
    // The header names the section that started on or before the page.
    assert_eq!(pages[1].first().map(String::as_str), Some("Findings"));
    assert_eq!(pages[1].last().map(String::as_str), Some("Page 2 of 3"));
    assert_eq!(pages[2].first().map(String::as_str), Some("Findings"));
    assert_eq!(pages[2].last().map(String::as_str), Some("Page 3 of 3"));
    let count = |marker: &[u8]| bytes.windows(marker.len()).filter(|w| *w == marker).count();
    let header: &[u8] = b"/Artifact <<\n  /Type /Pagination\n  /Subtype /Header\n  /Attached [/Top]\n>> BDC";
    let footer: &[u8] = b"/Artifact <<\n  /Type /Pagination\n  /Subtype /Footer\n  /Attached [/Bottom]\n>> BDC";
    assert_eq!((count(header), count(footer)), (2, 2));
}
