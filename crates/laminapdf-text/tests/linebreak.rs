//! UAX #14 conformance and box breaking (todo B4b): a subset of the Unicode 17.0.0
//! `LineBreakTest.txt` (`fixtures/unicode/`, every 10th line; `tools/fixtures/unicode_subsets.py`)
//! gives exactly the reference break opportunities, and a paragraph broken into a box gives
//! lines no wider than the box.
#![allow(
    clippy::unwrap_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "tests: a panic is a test failure"
)]

mod common;

use laminapdf_text::{Alignment, HarfRust, LayoutOptions, break_lines, line_breaks};

/// The text of a test line and its expected break offsets (bytes, the end included).
fn parse(line: &str) -> (String, Vec<usize>) {
    let mut text = String::new();
    let mut breaks = Vec::new();
    for token in line.split_whitespace() {
        match token {
            "×" => {}
            "÷" => {
                if !text.is_empty() {
                    breaks.push(text.len());
                }
            }
            hex => text.push(char::from_u32(u32::from_str_radix(hex, 16).unwrap()).unwrap()),
        }
    }
    (text, breaks)
}

#[test]
fn line_break_test_subset_passes() {
    let file = String::from_utf8(laminapdf_test_fixtures::fixture("unicode/LineBreakTest.txt")).unwrap();
    let tests: Vec<&str> = file
        .lines()
        .filter(|l| !l.starts_with('#') && !l.trim().is_empty())
        .collect();
    assert!(tests.len() > 1900, "{} lines", tests.len());
    let failures: Vec<&str> = tests
        .iter()
        .copied()
        .filter(|l| {
            let (text, expected) = parse(l);
            let got: Vec<usize> = line_breaks(&text).iter().map(|b| b.offset).collect();
            got != expected
        })
        .collect();
    assert!(
        failures.is_empty(),
        "{} failures, first: {:?}",
        failures.len(),
        failures.first()
    );
}

const PARAGRAPH: &str = "Shaping maps characters to positioned glyphs using the font's OpenType layout \
tables, and line breaking finds where a line may end: after spaces, between ideographs, never before \
closing punctuation (UAX #14). The office staff fill the affine waffle flask first.";

#[test]
fn paragraph_lines_fit_the_box() {
    let fonts = [common::font("NotoSans-Regular.ttf")];
    for width in [80.0_f32, 150.0, 233.5, 400.0] {
        let lines = break_lines(&HarfRust, &fonts, PARAGRAPH, &LayoutOptions::new(width, 11.0)).unwrap();
        assert!(lines.len() > 1, "{width}");
        for line in &lines {
            assert!(line.width <= f64::from(width), "{width}: {line:?}");
        }
        // Every byte of the text is on exactly one line, in order.
        let mut at = 0;
        for line in &lines {
            assert_eq!(line.range.start, at);
            at = line.range.end;
        }
        assert_eq!(at, PARAGRAPH.len());
    }
}

#[test]
fn greedy_lines_could_not_take_the_next_word() {
    let fonts = [common::font("NotoSans-Regular.ttf")];
    let width = 150.0_f32;
    let lines = break_lines(&HarfRust, &fonts, PARAGRAPH, &LayoutOptions::new(width, 11.0)).unwrap();
    // First fit: adding the next line's first word (up to its first break) must overflow.
    for pair in lines.windows(2) {
        let next = line_breaks(&PARAGRAPH[pair[1].range.clone()])[0].offset + pair[1].range.start;
        let joined = break_lines(
            &HarfRust,
            &fonts,
            &PARAGRAPH[pair[0].range.start..next],
            &LayoutOptions::new(width, 11.0),
        )
        .unwrap();
        assert!(
            joined.len() > 1,
            "{:?} could have taken more",
            &PARAGRAPH[pair[0].range.clone()]
        );
    }
}

#[test]
fn cjk_text_breaks_between_ideographs_and_keeps_closing_punctuation() {
    let fonts = [common::font("NotoSansSC-Regular.otf")];
    let text = "你好世界，这是中文字体子集测试。汉字排版与文本提取";
    let lines = break_lines(&HarfRust, &fonts, text, &LayoutOptions::new(60.0, 12.0)).unwrap();
    assert!(lines.len() > 2);
    for line in &lines {
        assert!(line.width <= 60.0);
        let first = text[line.content.clone()].chars().next().unwrap();
        assert!(
            !"，。".contains(first),
            "a line starts with closing punctuation: {line:?}"
        );
    }
}

#[test]
fn alignment_places_lines_in_the_box() {
    let fonts = [common::font("NotoSans-Regular.ttf")];
    let options = LayoutOptions::new(200.0, 11.0).with_align(Alignment::Center);
    for line in break_lines(&HarfRust, &fonts, PARAGRAPH, &options).unwrap() {
        assert!((line.x - (200.0 - line.width) / 2.0).abs() < 1e-9);
    }
}

#[test]
fn layout_is_deterministic() {
    let fonts = [common::font("NotoSans-Regular.ttf")];
    let options = LayoutOptions::new(150.0, 11.0);
    let a = break_lines(&HarfRust, &fonts, PARAGRAPH, &options).unwrap();
    let b = break_lines(&HarfRust, &fonts, PARAGRAPH, &options).unwrap();
    assert_eq!(a, b);
}
