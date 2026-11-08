use std::cell::RefCell;
use std::ops::Range;

use super::*;
use crate::font::Font;
use crate::shape::HarfRust;
use crate::shape::{Direction, ShapeOptions, ShapedRun, Shaper};
use crate::test_fonts::font;

fn texts<'t>(text: &'t str, lines: &[Line]) -> Vec<&'t str> {
    lines.iter().map(|l| &text[l.content.clone()]).collect()
}

#[test]
fn short_text_is_one_line() {
    let text = "Hello";
    let lines = break_lines(
        &HarfRust,
        &[font("NotoSans-Regular.ttf")],
        text,
        &LayoutOptions::new(500.0, 12.0),
    )
    .unwrap();
    assert_eq!(texts(text, &lines), ["Hello"]);
}

#[test]
fn breaks_at_spaces_when_the_box_is_narrow() {
    let text = "aaa bbb ccc";
    let fonts = [font("NotoSans-Regular.ttf")];
    let one = HarfRust
        .shape(&fonts[0], "aaa bbb", &ShapeOptions::new())
        .unwrap()
        .width(10.0);
    let one = format!("{one}").parse::<f32>().unwrap() + 0.01;
    let lines = break_lines(&HarfRust, &fonts, text, &LayoutOptions::new(one, 10.0)).unwrap();
    assert_eq!(texts(text, &lines), ["aaa bbb", "ccc"]);
}

#[test]
fn a_line_holding_a_whole_item_has_the_run_shaped_for_it() {
    let fonts = [font("NotoSans-Regular.ttf")];
    let text = "Total € 1,250.00";
    let lines = break_lines(&HarfRust, &fonts, text, &LayoutOptions::new(500.0, 10.0)).unwrap();
    let options = ShapeOptions::new()
        .with_script(crate::shape::Script::LATIN)
        .with_direction(Direction::LeftToRight);
    let fresh = HarfRust.shape_range(&fonts[0], text, 0..text.len(), &options).unwrap();
    assert_eq!(lines[0].runs[0].run, fresh);
}

/// A [`HarfRust`] that records every range it is asked to shape.
#[derive(Default)]
struct Counting(RefCell<Vec<(Range<usize>, Option<Direction>)>>);

impl Shaper for Counting {
    fn shape_range(&self, font: &Font, text: &str, range: Range<usize>, options: &ShapeOptions) -> Result<ShapedRun> {
        self.0.borrow_mut().push((range.clone(), options.direction));
        HarfRust.shape_range(font, text, range, options)
    }
}

#[test]
fn a_single_line_is_shaped_once() {
    let shaper = Counting::default();
    let text = "Product 12";
    let lines = break_lines(
        &shaper,
        &[font("NotoSans-Regular.ttf")],
        text,
        &LayoutOptions::new(500.0, 10.0),
    )
    .unwrap();
    assert_eq!(lines.len(), 1);
    assert_eq!(*shaper.0.borrow(), [(0..text.len(), Some(Direction::LeftToRight))]);
}

#[test]
fn line_breaking_shapes_no_range_twice() {
    // Words, then an overlong one broken between graphemes (whose lines are measured while
    // splitting and kept afterwards).
    let fonts = [font("NotoSans-Regular.ttf")];
    for (text, width) in [
        (
            "The quarter closed with steady growth across every region we track, and shipments rose.",
            120.0,
        ),
        ("abcdefghij klm", 20.0),
    ] {
        let shaper = Counting::default();
        let lines = break_lines(&shaper, &fonts, text, &LayoutOptions::new(width, 11.0)).unwrap();
        assert!(lines.len() > 2, "{text}");
        let calls = shaper.0.borrow();
        let mut unique = calls.clone();
        unique.sort_by_key(|(r, _)| (r.start, r.end));
        unique.dedup();
        assert_eq!(
            unique.len(),
            calls.len(),
            "a range of {text:?} was shaped twice: {calls:?}"
        );
    }
}

#[test]
fn hard_breaks_end_lines() {
    let text = "a\nb";
    let lines = break_lines(
        &HarfRust,
        &[font("NotoSans-Regular.ttf")],
        text,
        &LayoutOptions::new(500.0, 12.0),
    )
    .unwrap();
    assert_eq!(texts(text, &lines), ["a", "b"]);
    assert!(lines[0].hard_break);
    assert!(!lines[1].hard_break);
}

#[test]
fn overlong_words_break_between_graphemes() {
    let text = "abcdefghij";
    let lines = break_lines(
        &HarfRust,
        &[font("NotoSans-Regular.ttf")],
        text,
        &LayoutOptions::new(20.0, 12.0),
    )
    .unwrap();
    assert!(lines.len() > 1);
    assert!(lines.iter().all(|l| l.width <= 20.0));
    assert_eq!(texts(text, &lines).concat(), text);
}

#[test]
fn right_and_centre_alignment_offset_lines() {
    let fonts = [font("NotoSans-Regular.ttf")];
    let right = break_lines(
        &HarfRust,
        &fonts,
        "ab",
        &LayoutOptions::new(100.0, 10.0).with_align(Alignment::Right),
    )
    .unwrap();
    let centre = break_lines(
        &HarfRust,
        &fonts,
        "ab",
        &LayoutOptions::new(100.0, 10.0).with_align(Alignment::Center),
    )
    .unwrap();
    assert!((right[0].x + right[0].width - 100.0).abs() < 1e-9);
    assert!((centre[0].x * 2.0 + centre[0].width - 100.0).abs() < 1e-9);
}

#[test]
fn rtl_paragraphs_start_on_the_right_and_reverse_runs() {
    let fonts = [font("NotoSansHebrew-Regular.ttf"), font("NotoSans-Regular.ttf")];
    let text = "שלום abc";
    let lines = break_lines(&HarfRust, &fonts, text, &LayoutOptions::new(300.0, 10.0)).unwrap();
    let line = &lines[0];
    assert!((line.x + line.width - 300.0).abs() < 1e-9);
    let order: Vec<&str> = line.runs.iter().map(|r| &text[r.run.text_range.clone()]).collect();
    assert_eq!(order, ["abc", "שלום "]);
}
