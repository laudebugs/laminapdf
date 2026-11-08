//! `TextRun::bottom` and `TextLayout::bottom` (todo B10a): where the next element can start
//! below text. Expectations come from Helvetica's AFM metrics (ascender 718, descender -207 per
//! 1000 units), not from the builder, so a first-line or box-bottom answer fails.
#![allow(
    clippy::unwrap_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "tests: a panic is a test failure"
)]

mod common;

use common::text::shown;
use common::{reopen, save};
use laminapdf_build::{
    BuildOptions, Builder, FontHandle, Matrix, Overflow, PageSize, PointPt, Pt, RectPt, Standard14, TextBoxOptions,
    TextLayout,
};

const SIZE: f64 = 12.0;
/// Helvetica's AFM ascender and descender at `SIZE`, in points.
const ASCENT: f64 = 718.0 * SIZE / 1000.0;
const DESCENT: f64 = 207.0 * SIZE / 1000.0;
/// A run's leading, and a text box's default line height, at `SIZE`.
const LEADING: f64 = 1.2 * SIZE;

fn close(a: f64, b: f64) -> bool {
    (a - b).abs() < 1e-6
}

/// The bottom of a run of `text` with its first baseline at (72, 100).
fn run_bottom(text: &str) -> Option<f64> {
    let mut b = Builder::new(BuildOptions::default());
    let f = b.standard_font(Standard14::Helvetica).unwrap();
    let mut page = b.page(PageSize::A4);
    let s = page.surface();
    let run = s
        .text(text, &f, Pt::new(SIZE), PointPt::from_points(72.0, 100.0))
        .unwrap();
    run.bottom().map(Pt::get)
}

/// `text` laid out in a box at (72, 100), `width` wide and `height` high.
fn layout(text: &str, width: f64, height: f64, options: &TextBoxOptions) -> TextLayout {
    let mut builder = Builder::new(BuildOptions::default());
    let font: FontHandle = builder.standard_font(Standard14::Helvetica).unwrap();
    let mut page = builder.page(PageSize::A4);
    let area = RectPt::from_xywh(Pt::new(72.0), Pt::new(100.0), Pt::new(width), Pt::new(height));
    page.surface()
        .text_box(text, &font, Pt::new(SIZE), area, options)
        .unwrap()
}

/// The descent edge of line `n` (0-based) of a box at y = 100 with line height `leading`.
fn line_bottom(n: usize, leading: f64) -> f64 {
    #[expect(clippy::cast_precision_loss, reason = "a handful of lines")]
    let n = n as f64;
    100.0 + ASCENT + n * leading + DESCENT
}

const PARAGRAPH: &str = "Revenue rose twelve per cent over the quarter, led by the new reader and its offline mode.";

#[test]
fn a_runs_bottom_is_the_descent_below_its_baseline() {
    assert!(close(run_bottom("Results").unwrap(), 100.0 + DESCENT));
}

#[test]
fn a_run_with_line_breaks_ends_below_its_last_line() {
    assert!(close(
        run_bottom("one\ntwo\nthree").unwrap(),
        100.0 + 2.0 * LEADING + DESCENT
    ));
}

#[test]
fn blank_lines_after_the_last_drawn_one_dont_count() {
    assert!(close(run_bottom("one\n\n  \n").unwrap(), 100.0 + DESCENT));
}

#[test]
fn a_run_that_draws_nothing_has_no_bottom() {
    for text in ["", "   ", "\n", "\n\n", " \n \n"] {
        assert_eq!(run_bottom(text), None, "{text:?}");
    }
}

#[test]
fn a_wrapped_box_ends_at_its_last_lines_descent_not_the_box_bottom() {
    let l = layout(PARAGRAPH, 160.0, 400.0, &TextBoxOptions::default());
    assert!(l.lines.len() >= 3, "the paragraph should wrap: {} lines", l.lines.len());
    let bottom = l.bottom().unwrap().get();
    assert!(close(bottom, line_bottom(l.lines.len() - 1, LEADING)), "{bottom}");
    assert!(bottom < 500.0);
}

#[test]
fn a_boxs_bottom_follows_its_line_height() {
    let l = layout(
        "one\ntwo",
        400.0,
        400.0,
        &TextBoxOptions::default().with_line_height(2.0),
    );
    assert!(close(l.bottom().unwrap().get(), line_bottom(1, 2.0 * SIZE)));
}

#[test]
fn a_clipped_box_ends_at_its_last_drawn_line_and_keeps_its_overflow() {
    // Room for two lines: the first baseline, one line height, the descent.
    let two_lines = ASCENT + LEADING + DESCENT + 1.0;
    let l = layout(
        PARAGRAPH,
        160.0,
        two_lines,
        &TextBoxOptions::default().with_overflow(Overflow::Clip),
    );
    assert_eq!(l.lines.len(), 2);
    assert!(l.overflow.is_some());
    assert!(close(l.bottom().unwrap().get(), line_bottom(1, LEADING)));
}

#[test]
fn a_box_that_fits_no_line_has_no_bottom_and_overflows_from_the_start() {
    let l = layout(
        PARAGRAPH,
        160.0,
        4.0,
        &TextBoxOptions::default().with_overflow(Overflow::Clip),
    );
    assert_eq!((l.bottom(), l.overflow), (None, Some(0)));
}

#[test]
fn a_box_of_only_blanks_has_no_bottom() {
    let l = layout("  \n \n", 160.0, 400.0, &TextBoxOptions::default());
    assert_eq!(l.bottom(), None);
}

#[test]
fn the_bottom_is_measured_before_the_surfaces_transform() {
    let mut b = Builder::new(BuildOptions::default());
    let f = b.standard_font(Standard14::Helvetica).unwrap();
    let mut page = b.page(PageSize::A4);
    let s = page.surface();
    s.transform(Matrix::translate(0.0, 50.0));
    let run = s
        .text("Moved", &f, Pt::new(SIZE), PointPt::from_points(72.0, 100.0))
        .unwrap();
    assert!(close(run.bottom().unwrap().get(), 100.0 + DESCENT));
}

/// The next paragraph, drawn a gap below the first one's bottom, starts there when the file is
/// read back: its baseline is the bottom, the gap and the font's ascent below the first's top.
#[test]
fn a_paragraph_placed_below_the_bottom_reads_back_there() {
    let mut b = Builder::new(BuildOptions::default());
    let f = b.standard_font(Standard14::Helvetica).unwrap();
    let mut page = b.page(PageSize::A4);
    let s = page.surface();
    let first = RectPt::from_xywh(Pt::new(72.0), Pt::new(100.0), Pt::new(160.0), Pt::new(400.0));
    let l = s
        .text_box(PARAGRAPH, &f, Pt::new(SIZE), first, &TextBoxOptions::default())
        .unwrap();
    let gap = 10.0;
    let next_baseline = l.bottom().unwrap().get() + gap + ASCENT;
    s.text("Next", &f, Pt::new(SIZE), PointPt::from_points(72.0, next_baseline))
        .unwrap();
    page.finish().unwrap();
    let doc = reopen(&save(b.finish().unwrap()));
    let (strings, _) = shown(&doc, 0);
    let next = strings.iter().find(|s| s.text == "Next").unwrap();
    // User space has y up from the bottom of the A4 page (841.89 pt high).
    let expected = 841.89 - line_bottom(l.lines.len() - 1, LEADING) - gap - ASCENT;
    assert!(
        (next.origin.1 - expected).abs() < 0.01,
        "{} vs {expected}",
        next.origin.1
    );
}

/// Each font the two layout paths take: a standard 14 font (AFM widths) and an embedded one
/// (shaped through `laminapdf-text`), whose trimming of a line differs.
fn fonts() -> [fn(&mut Builder) -> FontHandle; 2] {
    [
        |b| b.standard_font(Standard14::Helvetica).unwrap(),
        |b| {
            b.font(common::text_and_navigation::font("NotoSans-Regular.ttf"))
                .unwrap()
        },
    ]
}

/// A run's and a box's bottom for `text` in the font `make` registers.
fn both(make: fn(&mut Builder) -> FontHandle, text: &str) -> (Option<Pt>, Option<Pt>) {
    let mut builder = Builder::new(BuildOptions::default());
    let font = make(&mut builder);
    let mut page = builder.page(PageSize::A4);
    let s = page.surface();
    let run = s
        .text(text, &font, Pt::new(SIZE), PointPt::from_points(72.0, 100.0))
        .unwrap()
        .bottom();
    let area = RectPt::from_xywh(Pt::new(72.0), Pt::new(100.0), Pt::new(300.0), Pt::new(400.0));
    let boxed = s
        .text_box(text, &font, Pt::new(SIZE), area, &TextBoxOptions::default())
        .unwrap()
        .bottom();
    (run, boxed)
}

#[test]
fn white_space_and_controls_draw_nothing_in_either_layout_path() {
    for make in fonts() {
        for text in ["", " ", "\u{a0}", "\u{1c}", "\u{2029}", "\n \u{a0}\n"] {
            assert_eq!(both(make, text), (None, None), "{text:?}");
        }
    }
}

#[test]
fn trailing_separators_dont_move_the_bottom_in_either_layout_path() {
    for make in fonts() {
        assert_eq!(both(make, "one\n\u{1c}\n\u{a0}"), both(make, "one"));
    }
}
