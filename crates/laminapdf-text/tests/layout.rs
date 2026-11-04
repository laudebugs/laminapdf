//! Line layout: each bidi paragraph finds its own direction, tabs shape as spaces, no line is
//! only hanging spaces, fallback fonts don't stick, mixed-direction lines fit, breaks and
//! ignorables need no font, alignment applies per paragraph, and visual runs follow a line slice.
#![allow(
    clippy::unwrap_used,
    clippy::indexing_slicing,
    clippy::panic,
    reason = "tests: a panic is a test failure"
)]

mod common;

use common::{font, latin};
use laminapdf_text::{Alignment, BidiParagraph, HarfRust, LayoutOptions, ShapeOptions, Shaper, break_lines, itemize};

/// A line feed ends a bidi paragraph (UAX #9 P1); the next paragraph finds its own
/// direction, so a Latin paragraph after a Hebrew one starts on the left.
#[test]
fn each_paragraph_resolves_its_own_direction() {
    let fonts = [font("NotoSansHebrew-Regular.ttf"), latin()];
    let lines = break_lines(&HarfRust, &fonts, "שלום\nabc", &LayoutOptions::new(300.0, 10.0)).unwrap();
    assert_eq!(lines.len(), 2);
    assert!(
        (lines[0].x + lines[0].width - 300.0).abs() < 1e-9,
        "Hebrew starts on the right"
    );
    assert_eq!(lines[1].x, 0.0, "Latin starts on the left");
    assert_eq!(lines[1].runs[0].level, 0);
}

/// An isolate left open in one paragraph does not leak into the next (X8 closes it at
/// the paragraph end).
#[test]
fn open_isolates_end_with_their_paragraph() {
    let fonts = [latin(), font("NotoSansHebrew-Regular.ttf")];
    let text = "a\u{2067}b\nc";
    let lines = break_lines(&HarfRust, &fonts, text, &LayoutOptions::new(300.0, 10.0)).unwrap();
    assert_eq!(lines[1].runs.iter().map(|r| r.level).collect::<Vec<_>>(), [0]);
}

/// A tab needs no glyph of its own: it is shaped as a space.
#[test]
fn tabs_are_shaped_as_spaces() {
    let font = latin();
    let run = HarfRust.shape(&font, "a\tb", &ShapeOptions::new()).unwrap();
    assert!(!run.has_notdef());
    assert_eq!(run.glyphs[1].gid, font.glyph_for(' ').unwrap());
    let lines = break_lines(&HarfRust, &[font], "a\tb", &LayoutOptions::new(300.0, 10.0)).unwrap();
    assert_eq!(lines.len(), 1);
}

/// A cluster wider than the box followed by a space never leaves a line holding only
/// the hanging space.
#[test]
fn no_line_is_only_hanging_spaces() {
    let text = "ab c";
    let lines = break_lines(&HarfRust, &[latin()], text, &LayoutOptions::new(5.0, 12.0)).unwrap();
    for line in &lines {
        assert!(!line.content.is_empty(), "{lines:?}");
    }
    let drawn: String = lines.iter().map(|l| &text[l.content.clone()]).collect();
    assert_eq!(drawn, "abc");
}

/// After a fallback font, characters the primary font covers go back to it.
#[test]
fn fallback_fonts_do_not_stick() {
    let fonts = [latin(), font("NotoSansSC-Regular.otf")];
    let text = "中a";
    let items = itemize(&BidiParagraph::new(text, None), &fonts).unwrap();
    let got: Vec<(&str, usize)> = items.iter().map(|i| (&text[i.range.clone()], i.font)).collect();
    assert_eq!(got, [("中", 1), ("a", 0)]);
}

/// Lines are measured as they are drawn, so mixed-direction text with separators never
/// exceeds the box.
#[test]
fn drawn_lines_fit_for_mixed_direction_text() {
    let fonts = [
        latin(),
        font("NotoSansHebrew-Regular.ttf"),
        font("NotoSansArabic-Regular.ttf"),
    ];
    let text = "One שלום\tעולם two مرحبا\tبالعالم three שלום עולם four, five six seven.";
    for width in [40.0_f32, 61.0, 77.5, 90.0, 133.0, 170.0] {
        for line in break_lines(&HarfRust, &fonts, text, &LayoutOptions::new(width, 10.0)).unwrap() {
            assert!(line.width <= f64::from(width) + 1e-9, "{width}: {line:?}");
        }
    }
}

/// Text with nothing to draw lays out with no fonts.
#[test]
fn breaks_and_ignorables_need_no_font() {
    assert!(itemize(&BidiParagraph::new("\n\u{200D}", None), &[]).is_ok());
    assert!(break_lines(&HarfRust, &[], "\n", &LayoutOptions::new(100.0, 10.0)).is_ok());
}

/// Justification is reserved: asking for it is an error, not silently ignored.
#[test]
fn justify_is_refused() {
    let mut options = LayoutOptions::new(100.0, 10.0);
    options.justify = true;
    let err = break_lines(&HarfRust, &[latin()], "a b", &options).unwrap_err();
    assert_eq!(err.code(), "unsupported-layout-option");
}

/// Visual runs of a sub-range of a line match the reference reordering of that range alone.
#[test]
fn visual_runs_of_a_line_slice() {
    let text = "abc שלום def עולם";
    let para = BidiParagraph::new(text, None);
    let line = 4..text.len();
    let runs: Vec<(&str, u8)> = para
        .visual_runs(line)
        .unwrap()
        .iter()
        .map(|r| (&text[r.range.clone()], r.level))
        .collect();
    assert_eq!(runs, [("שלום", 1), (" def ", 0), ("עולם", 1)]);
}

/// Alignment still works with paragraph splitting: centred lines of both paragraphs.
#[test]
fn alignment_applies_per_paragraph() {
    let lines = break_lines(
        &HarfRust,
        &[latin()],
        "ab\ncd",
        &LayoutOptions::new(100.0, 10.0).with_align(Alignment::Center),
    )
    .unwrap();
    for line in &lines {
        assert!((line.x * 2.0 + line.width - 100.0).abs() < 1e-9);
    }
}
