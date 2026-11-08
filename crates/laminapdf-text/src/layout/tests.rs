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

mod spans {
    use super::*;
    use crate::error::Error;
    use crate::test_fonts::bold;

    fn latin() -> [Font; 2] {
        [font("NotoSans-Regular.ttf"), bold()]
    }

    #[test]
    fn one_span_gives_the_lines_break_lines_gives() {
        let fonts = [font("NotoSans-Regular.ttf"), font("NotoSansHebrew-Regular.ttf")];
        let options = LayoutOptions::new(90.0, 11.0);
        for text in [
            "The quarter closed with steady growth across every region, שלום עולם, and more.",
            "a\nb\n\nabcdefghijklmnop",
            "",
        ] {
            let lines = break_lines(&HarfRust, &fonts, text, &options).unwrap();
            let spans = break_spans(&HarfRust, &fonts, text, &[Span::new(0..text.len())], &options).unwrap();
            assert_eq!(lines, spans, "{text:?}");
        }
    }

    #[test]
    fn equal_spans_are_shaped_as_one_run() {
        let shaper = Counting::default();
        let text = "AVAV";
        let spans = [Span::new(0..2), Span::new(2..4)];
        let lines = break_spans(&shaper, &latin(), text, &spans, &LayoutOptions::new(500.0, 12.0)).unwrap();
        assert_eq!(lines[0].runs.len(), 1);
        assert_eq!(lines[0].runs[0].spans, 0..2);
        // Kerning across the boundary is kept: the run is the whole text shaped once.
        assert_eq!(*shaper.0.borrow(), [(0..4, Some(Direction::LeftToRight))]);
    }

    #[test]
    fn a_line_takes_its_height_from_its_tallest_span() {
        let text = "small Big small";
        let spans = [
            Span::new(0..6).with_size(11.0),
            Span::new(6..9).with_size(16.0),
            Span::new(9..15).with_size(11.0),
        ];
        let fonts = latin();
        let line = &break_spans(&HarfRust, &fonts, text, &spans, &LayoutOptions::new(500.0, 11.0)).unwrap()[0];
        let info = fonts[0].info();
        let at = |size: f64| f64::from(info.ascent) * size / f64::from(info.units_per_em);
        assert!((line.ascent - at(16.0)).abs() < 1e-9);
        assert!(line.descent < 0.0);
        let sizes: Vec<f32> = line.runs.iter().map(|r| r.size).collect();
        assert_eq!(sizes, [11.0, 16.0, 11.0]);
    }

    #[test]
    fn a_bold_word_takes_its_span_s_font_and_width() {
        let text = "a bold word";
        let fonts = latin();
        let spans = [Span::new(0..2), Span::new(2..6).with_fonts([1]), Span::new(6..11)];
        let line = &break_spans(&HarfRust, &fonts, text, &spans, &LayoutOptions::new(500.0, 12.0)).unwrap()[0];
        let runs: Vec<(&str, usize)> = line
            .runs
            .iter()
            .map(|r| (&text[r.run.text_range.clone()], r.font))
            .collect();
        assert_eq!(runs, [("a ", 0), ("bold", 1), (" word", 0)]);
        let regular = HarfRust
            .shape(&fonts[0], "bold", &ShapeOptions::new())
            .unwrap()
            .width(12.0);
        assert!(line.runs[1].run.width(12.0) > regular);
        let sum: f64 = line.runs.iter().map(|r| r.run.width(r.size)).sum();
        assert!((line.width - sum).abs() < 1e-9);
    }

    #[test]
    fn a_line_can_break_where_a_span_ends_at_a_space() {
        let text = "aaaa bbbb";
        let fonts = latin();
        let spans = [Span::new(0..5), Span::new(5..9).with_fonts([1])];
        let widest = HarfRust
            .shape(&fonts[1], "bbbb", &ShapeOptions::new())
            .unwrap()
            .width(10.0);
        let width = format!("{widest}").parse::<f32>().unwrap() + 1.0;
        let lines = break_spans(&HarfRust, &fonts, text, &spans, &LayoutOptions::new(width, 10.0)).unwrap();
        assert_eq!(texts(text, &lines), ["aaaa", "bbbb"]);
        assert_eq!(lines[1].runs[0].font, 1);
    }

    #[test]
    fn a_span_boundary_inside_a_word_is_no_break_opportunity() {
        // "un" + bold "believ" + "able": one word, so a box too narrow for it still keeps it
        // whole on one line before breaking between graphemes, never at the span boundaries.
        let text = "x unbelievable";
        let fonts = latin();
        let spans = [Span::new(0..4), Span::new(4..10).with_fonts([1]), Span::new(10..14)];
        let whole = break_spans(&HarfRust, &fonts, text, &spans, &LayoutOptions::new(500.0, 10.0)).unwrap();
        let width = format!("{}", whole[0].width).parse::<f32>().unwrap() - 1.0;
        let lines = break_spans(&HarfRust, &fonts, text, &spans, &LayoutOptions::new(width, 10.0)).unwrap();
        assert_eq!(texts(text, &lines), ["x", "unbelievable"]);
        assert_eq!(lines[1].runs.len(), 3);
    }

    #[test]
    fn a_rise_is_carried_on_the_run() {
        let text = "x2";
        let spans = [Span::new(0..1), Span::new(1..2).with_size(7.0).with_rise(4.0)];
        let line = &break_spans(&HarfRust, &latin(), text, &spans, &LayoutOptions::new(500.0, 12.0)).unwrap()[0];
        assert_eq!((line.runs[1].size, line.runs[1].rise), (7.0, 4.0));
        assert_eq!(line.runs[0].rise, 0.0);
    }

    #[test]
    fn a_subscript_lowers_the_line_s_descent() {
        let text = "H2O";
        let fonts = latin();
        let plain = &break_lines(&HarfRust, &fonts, text, &LayoutOptions::new(500.0, 12.0)).unwrap()[0];
        let spans = [Span::new(0..1), Span::new(1..2).with_rise(-6.0), Span::new(2..3)];
        let line = &break_spans(&HarfRust, &fonts, text, &spans, &LayoutOptions::new(500.0, 12.0)).unwrap()[0];
        assert!((line.descent - (plain.descent - 6.0)).abs() < 1e-9);
        assert!((line.ascent - plain.ascent).abs() < 1e-9);
    }

    #[test]
    fn a_span_s_language_and_features_reach_the_shaper() {
        let text = "fi fi";
        let fonts = latin();
        let off: Feature = "liga=0".parse().unwrap();
        let spans = [Span::new(0..3), Span::new(3..5).with_feature(off)];
        let line = &break_spans(&HarfRust, &fonts, text, &spans, &LayoutOptions::new(500.0, 12.0)).unwrap()[0];
        let glyphs: Vec<usize> = line.runs.iter().map(|r| r.run.glyphs.len()).collect();
        // The first "fi" is a ligature (one glyph, plus the space); the second is two glyphs.
        assert_eq!(glyphs, [2, 2]);
    }

    #[test]
    fn spans_must_cover_the_text_once_in_order() {
        let fonts = latin();
        let options = LayoutOptions::new(500.0, 12.0);
        let text = "Hello, world";
        for (spans, at) in [
            (vec![Span::new(0..5), Span::new(6..12)], 5),
            (vec![Span::new(0..6), Span::new(5..12)], 6),
            (vec![Span::new(0..5)], 5),
            (vec![Span::new(0..13)], 0),
            (vec![Span::new(1..12)], 0),
            (vec![], 0),
        ] {
            let err = break_spans(&HarfRust, &fonts, text, &spans, &options).unwrap_err();
            assert_eq!(err, Error::InvalidSpans { at }, "{spans:?}");
        }
        let err = break_spans(&HarfRust, &fonts, "é", &[Span::new(0..1), Span::new(1..2)], &options).unwrap_err();
        assert_eq!(err, Error::InvalidSpans { at: 0 });
        assert_eq!(
            break_spans(&HarfRust, &fonts, "", &[], &options).unwrap(),
            break_lines(&HarfRust, &fonts, "", &options).unwrap()
        );
    }

    #[test]
    fn a_span_s_fonts_decide_which_fonts_its_text_may_use() {
        let fonts = [font("NotoSans-Regular.ttf"), font("NotoSansHebrew-Regular.ttf")];
        let options = LayoutOptions::new(500.0, 12.0);
        let text = "ab שלום";
        let only_latin = [Span::new(0..text.len()).with_fonts([0])];
        assert!(matches!(
            break_spans(&HarfRust, &fonts, text, &only_latin, &options),
            Err(Error::MissingGlyph { ch: 'ש', .. })
        ));
        let hebrew_first = [Span::new(0..3), Span::new(3..text.len()).with_fonts([1, 0])];
        let line = &break_spans(&HarfRust, &fonts, text, &hebrew_first, &options).unwrap()[0];
        assert!(line.runs.iter().any(|r| r.font == 1));
    }
}
