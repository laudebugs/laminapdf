//! UAX #9 conformance (todo B4b): a subset of the Unicode 17.0.0 `BidiCharacterTest.txt`
//! (`fixtures/unicode/`, every 100th line; `tools/fixtures/unicode_subsets.py`) gives the
//! same paragraph level, resolved levels (after rule L1) and visual order (L2) as the
//! reference, and a mixed English/Arabic/Hebrew paragraph is reordered as UAX #9 says.
#![allow(
    clippy::unwrap_used,
    clippy::indexing_slicing,
    reason = "tests: a panic is a test failure"
)]

mod common;

use icu_properties::CodePointMapData;
use icu_properties::props::BidiClass;
use lamina_text::{BidiParagraph, Direction};

/// Characters rule X9 removes: the reference gives them no level (`x`) and no place in the
/// visual order.
fn removed_by_x9(ch: char) -> bool {
    matches!(
        CodePointMapData::<BidiClass>::new().get(ch),
        BidiClass::BoundaryNeutral
            | BidiClass::LeftToRightEmbedding
            | BidiClass::RightToLeftEmbedding
            | BidiClass::PopDirectionalFormat
            | BidiClass::LeftToRightOverride
            | BidiClass::RightToLeftOverride
    )
}

/// The character indices of `text` in visual order (removed characters left out).
fn visual_order(para: &BidiParagraph<'_>, text: &str) -> Vec<usize> {
    let chars: Vec<(usize, char)> = text.char_indices().collect();
    let mut order = Vec::new();
    for run in para.visual_runs(0..text.len()).unwrap() {
        let mut indices: Vec<usize> = (0..chars.len()).filter(|&i| run.range.contains(&chars[i].0)).collect();
        if run.direction().is_rtl() {
            indices.reverse();
        }
        order.extend(indices.into_iter().filter(|&i| !removed_by_x9(chars[i].1)));
    }
    order
}

/// Checks one test line (code points; direction 0/1/2; paragraph level; levels; order).
fn check(line: &str) -> Result<(), String> {
    let fields: Vec<&str> = line.split(';').collect();
    let text: String = fields[0]
        .split_whitespace()
        .map(|h| char::from_u32(u32::from_str_radix(h, 16).unwrap()).unwrap())
        .collect();
    let base = match fields[1] {
        "0" => Some(Direction::LeftToRight),
        "1" => Some(Direction::RightToLeft),
        _ => None, // 2: auto, left to right without a strong character (P2–P3)
    };
    let para = BidiParagraph::new(&text, base);
    let level = if para.base_direction().is_rtl() { "1" } else { "0" };
    let levels: Vec<String> = para
        .line_levels(0..text.len())
        .unwrap()
        .iter()
        .zip(text.chars())
        .map(|(l, ch)| {
            if removed_by_x9(ch) {
                "x".to_owned()
            } else {
                l.to_string()
            }
        })
        .collect();
    let expected_levels: Vec<&str> = fields[3].split_whitespace().collect();
    let expected_order: Vec<usize> = fields[4].split_whitespace().map(|s| s.parse().unwrap()).collect();
    if level != fields[2] {
        return Err(format!("paragraph level {level}"));
    }
    if levels != expected_levels {
        return Err(format!("levels {levels:?}"));
    }
    let order = visual_order(&para, &text);
    if order != expected_order {
        return Err(format!("order {order:?}"));
    }
    Ok(())
}

#[test]
fn bidi_character_test_subset_passes() {
    let file = String::from_utf8(common::fixture("unicode/BidiCharacterTest.txt")).unwrap();
    let tests: Vec<&str> = file
        .lines()
        .filter(|l| !l.starts_with('#') && !l.trim().is_empty())
        .collect();
    assert!(tests.len() > 900, "{} lines", tests.len());
    let failures: Vec<String> = tests
        .iter()
        .filter_map(|l| check(l).err().map(|e| format!("{l}: {e}")))
        .collect();
    assert!(
        failures.is_empty(),
        "{} failures, first: {:?}",
        failures.len(),
        failures.first()
    );
}

#[test]
fn mixed_english_arabic_hebrew_paragraph_is_in_visual_order() {
    // Logical: "The words " + Hebrew "שלום עולם" + " and " + Arabic "مرحبا بالعالم" + " mean hello."
    // In a left-to-right paragraph each right-to-left word sequence (with the space between
    // its words, rule N1) is one level-1 run, drawn reversed in place (L2).
    let text = "The words שלום עולם and مرحبا بالعالم mean hello.";
    let para = BidiParagraph::new(text, None);
    assert_eq!(para.base_direction(), Direction::LeftToRight);
    let runs: Vec<(&str, u8)> = para
        .visual_runs(0..text.len())
        .unwrap()
        .iter()
        .map(|r| (&text[r.range.clone()], r.level))
        .collect();
    assert_eq!(
        runs,
        [
            ("The words ", 0),
            ("שלום עולם", 1),
            (" and ", 0),
            ("مرحبا بالعالم", 1),
            (" mean hello.", 0),
        ]
    );
}

#[test]
fn rtl_paragraph_with_an_embedded_english_phrase_and_number() {
    // Arabic paragraph: the English phrase and the number are level 2, the whole line is
    // reversed, so the last logical run comes first on the left.
    let text = "قرأت ABC 2024 مرة";
    let para = BidiParagraph::new(text, None);
    assert_eq!(para.base_direction(), Direction::RightToLeft);
    let visual: Vec<(&str, u8)> = para
        .visual_runs(0..text.len())
        .unwrap()
        .iter()
        .map(|r| (&text[r.range.clone()], r.level))
        .collect();
    assert_eq!(visual, [(" مرة", 1), ("ABC 2024", 2), ("قرأت ", 1)]);
}
