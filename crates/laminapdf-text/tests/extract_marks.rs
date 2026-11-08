//! Nonspacing marks in right-to-left runs (bug `rtl-mark-on-wrong-letter`): each mark stays on
//! the letter it was shaped with, whichever side of it the producer draws the mark on.
#![allow(clippy::expect_used, reason = "tests: a panic is a test failure")]

use laminapdf_cos::OpenOptions;
use laminapdf_model::Document;
use laminapdf_text::TextExtractor;

mod common;

use common::extract::{extract, font, page};

/// The Arabic paragraph of `fixtures/golden/tagged-across-pages.pdf`, as
/// `crates/laminapdf-build/tests/common/across_pages.rs`'s `ARABIC` holds it.
const ARABIC: &str = "تُظهر هذه الفقرة أن النص العربي يُشكَّل ويُرتَّب من اليمين إلى اليسار، \
                      وأن الحروف تتصل كما ينبغي في كل سطر من الأسطر.";

#[test]
fn the_golden_arabic_paragraph_comes_out_as_its_source_text() {
    // HarfRust draws each cluster's marks before its base; with Noto Sans Arabic a damma's
    // origin falls just inside the narrow letter drawn before its base (تُ after ظ).
    let doc = Document::open(
        laminapdf_test_fixtures::fixture("golden/tagged-across-pages.pdf"),
        OpenOptions::default(),
    )
    .expect("opens");
    let pages = doc.pages().expect("pages");
    let page = pages.get(2).expect("a third page");
    let text = TextExtractor::new().page(&doc, &page).expect("extracts");
    let arabic: Vec<&str> = text
        .runs
        .iter()
        .map(|r| r.text.as_str())
        .filter(|t| t.chars().any(laminapdf_text::is_right_to_left))
        .collect();
    assert_eq!(arabic.join(" "), ARABIC);
}

/// A Hebrew font: code 1 ש, 2 ל, 3 ו, 4 ם, each 500 thousandths wide, and 6 the qamats (U+05B8,
/// a nonspacing mark) with no width.
fn hebrew_with_qamats() -> laminapdf_syntax::Dict {
    font(
        "<< /Type /Font /Subtype /Type1 /BaseFont /H /FontDescriptor << /Flags 32 >>
           /FirstChar 1 /LastChar 6 /Widths [500 500 500 500 0 0]
           /Encoding << /Differences [1 /uni05E9 /uni05DC /uni05D5 /uni05DD 6 /uni05B8] >> >>",
    )
}

fn lines(content: &[u8]) -> Vec<String> {
    extract(&page(content, vec![hebrew_with_qamats()], OpenOptions::default()))
        .expect("extracts")
        .lines()
}

#[test]
fn a_mark_drawn_before_its_letter_and_just_inside_the_previous_one_stays_on_its_letter() {
    // "שָלום" drawn left to right as a shaper draws it: ם ו ל (x 100 → 115), the qamats moved
    // 0.4 pt back into ל, then ש at 115, the letter the qamats belongs to.
    assert_eq!(
        lines(b"BT /F1 10 Tf 100 700 Td [<040302> 40 <06> -40 <01>] TJ ET"),
        ["שָלום"]
    );
}

#[test]
fn a_mark_drawn_after_its_letter_over_its_middle_stays_on_its_letter() {
    // The same word with the qamats drawn after ש, moved back 2.5 pt to the middle of ש.
    assert_eq!(lines(b"BT /F1 10 Tf 100 700 Td [<04030201> 250 <06>] TJ ET"), ["שָלום"]);
}

#[test]
fn a_mark_drawn_at_the_pen_after_its_letter_stays_on_that_letter() {
    // ם ו ל, the qamats at the pen after ל (x 115, where ש starts), then ש: the qamats belongs to
    // ל, the letter whose advance ends at its origin, and doesn't move to ש.
    assert_eq!(lines(b"BT /F1 10 Tf 100 700 Td [<040302> <06> <01>] TJ ET"), ["שלָום"]);
}
