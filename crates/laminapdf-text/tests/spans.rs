//! Styled spans on a page (todo T4g): a paragraph broken by `break_spans` with a bold word, a
//! larger word and a superscript, and an Arabic paragraph with a bold Latin word, drawn by the
//! test page writer and read back with `TextExtractor`.
#![allow(
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "tests: a panic is a test failure"
)]

use laminapdf_cos::OpenOptions;
use laminapdf_model::Document;
use laminapdf_text::{HarfRust, LayoutOptions, PageText, StyledSpan, TextExtractor, break_spans};

mod common;

use common::{TestPdf, bold, font};

const LATIN: &str = "Revenue rose 12% in the second QUARTER, as E = mc2 predicts.";
const ARABIC: &str = "مرحبا Lamina بالعالم";

/// The byte range of `part` in `text`.
fn at(text: &str, part: &str) -> std::ops::Range<usize> {
    let start = text.find(part).expect("part");
    start..start + part.len()
}

/// Spans covering `text`: `styled` ranges in their own style, the rest plain.
fn spans(text: &str, styled: Vec<StyledSpan>) -> Vec<StyledSpan> {
    let mut out = Vec::new();
    let mut end = 0;
    for span in styled {
        if span.range.start > end {
            out.push(StyledSpan::new(end..span.range.start));
        }
        end = span.range.end;
        out.push(span);
    }
    if end < text.len() {
        out.push(StyledSpan::new(end..text.len()));
    }
    out
}

fn latin_spans() -> Vec<StyledSpan> {
    spans(
        LATIN,
        vec![
            StyledSpan::new(at(LATIN, "12%")).with_fonts([1]),
            StyledSpan::new(at(LATIN, "QUARTER")).with_size(16.0),
            StyledSpan::new(at(LATIN, "2 ")).with_size(7.0).with_rise(4.0),
        ],
    )
}

fn arabic_spans() -> Vec<StyledSpan> {
    spans(ARABIC, vec![StyledSpan::new(at(ARABIC, "Lamina")).with_fonts([2])])
}

/// The page: both paragraphs, and the text of each line drawn.
fn page() -> (Vec<u8>, Vec<String>) {
    let mut pdf = TestPdf::new();
    let regular = pdf.add_font(font("NotoSans-Regular.ttf")).expect("font");
    let bold = pdf.add_font(bold()).expect("font");
    let arabic = pdf.add_font(font("NotoSansArabic-Regular.ttf")).expect("font");
    let mut drawn = pdf
        .spans(
            &[regular, bold],
            LATIN,
            &latin_spans(),
            &LayoutOptions::new(400.0, 11.0),
        )
        .expect("latin");
    drawn.extend(
        pdf.spans(
            &[arabic, regular, bold],
            ARABIC,
            &arabic_spans(),
            &LayoutOptions::new(400.0, 16.0),
        )
        .expect("arabic"),
    );
    (pdf.finish().expect("saves").0, drawn)
}

fn extracted(bytes: Vec<u8>) -> PageText {
    let doc = Document::open(bytes, OpenOptions::default()).expect("opens");
    let pages = doc.pages().expect("pages");
    let page = pages.iter().next().expect("one page");
    TextExtractor::new().page(&doc, &page).expect("extracts")
}

#[test]
fn each_paragraph_fits_one_line() {
    assert_eq!(page().1, [LATIN, ARABIC]);
}

#[test]
fn a_bold_latin_word_keeps_the_arabic_paragraph_s_visual_order() {
    let fonts = [font("NotoSansArabic-Regular.ttf"), font("NotoSans-Regular.ttf"), bold()];
    let lines = break_spans(
        &HarfRust,
        &fonts,
        ARABIC,
        &arabic_spans(),
        &LayoutOptions::new(400.0, 16.0),
    )
    .expect("lines");
    let visual: Vec<(&str, usize)> = lines[0]
        .runs
        .iter()
        .map(|r| (ARABIC[r.run.text_range.clone()].trim(), r.font))
        .collect();
    // Right to left: the last Arabic word on the left, "Lamina" in the middle, in bold.
    assert_eq!(visual, [("بالعالم", 0), ("Lamina", 2), ("مرحبا", 0)]);
}

#[test]
fn the_page_extracts_in_logical_order() {
    let (bytes, _) = page();
    let text = extracted(bytes);
    assert_eq!(text.lines(), [LATIN, ARABIC]);
    assert_eq!(text.unmapped, 0);
}

#[test]
fn the_superscript_is_drawn_with_a_rise_and_extracts_as_its_text() {
    let text = extracted(page().0);
    let glyph = |t: &str| text.glyphs.iter().rfind(|g| g.text == t).expect("glyph");
    let (c, two) = (glyph("c"), glyph("2"));
    // 4 pt above the line's baseline (user space: y up), at 7 pt.
    assert!(
        (two.origin.y - c.origin.y - 4.0).abs() < 1e-3,
        "{} vs {}",
        two.origin.y,
        c.origin.y
    );
    assert!((two.size - 7.0).abs() < 1e-9);
}
