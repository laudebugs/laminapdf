//! `Surface::text_box` (todo B5b): no line wider than the box, overflow clipped and reported
//! or refused, alignment, and the lines reading back as the text.
#![allow(
    clippy::unwrap_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "tests: a panic is a test failure"
)]

mod common;

use common::text::lines;
use common::text_and_navigation::{self, LATIN_PARAGRAPH, MULTISCRIPT_BOX_WIDTH, SCRIPTS};
use common::{reopen, save};
use laminapdf_build::{
    Alignment, BuildOptions, Builder, FontHandle, Overflow, PageSize, Standard14, TextBoxOptions, TextLayout,
};
use laminapdf_model::{Pt, RectPt};
use laminapdf_test_fixtures::fixture;

fn rect(x: f64, y: f64, w: f64, h: f64) -> RectPt {
    RectPt::from_xywh(Pt::new(x), Pt::new(y), Pt::new(w), Pt::new(h))
}

/// Lays `text` out in `r` on a fresh page with the font `make` registers; returns the
/// layout, or the error, and the saved document when the page finished.
fn boxed(
    make: impl FnOnce(&mut Builder) -> FontHandle,
    text: &str,
    r: RectPt,
    options: &TextBoxOptions,
) -> (laminapdf_build::Result<TextLayout>, Vec<u8>) {
    let mut b = Builder::new(BuildOptions::default());
    let f = make(&mut b);
    let mut page = b.page(PageSize::A4);
    let layout = page.surface().text_box(text, &f, Pt::new(11.0), r, options);
    page.finish().unwrap();
    (layout, save(b.finish().unwrap()))
}

fn noto(b: &mut Builder) -> FontHandle {
    b.font(text_and_navigation::font("NotoSans-Regular.ttf")).unwrap()
}

fn times(b: &mut Builder) -> FontHandle {
    b.standard_font(Standard14::TimesRoman).unwrap()
}

#[test]
fn no_line_is_wider_than_its_box_in_any_script_or_width() {
    for (file, text) in SCRIPTS {
        for width in [60.0, MULTISCRIPT_BOX_WIDTH, 200.0] {
            let r = rect(50.0, 50.0, width, 700.0);
            let (layout, _) = boxed(
                |b| b.font(text_and_navigation::font(file)).unwrap(),
                text,
                r,
                &TextBoxOptions::default(),
            );
            let layout = layout.unwrap();
            assert!(!layout.overflowed(), "{file} at {width}");
            for line in &layout.lines {
                assert!(line.rect.width().get() <= width + 1e-3, "{file} at {width}: {line:?}");
                assert!(line.rect.x0 >= r.x0 - Pt::new(1e-3) && line.rect.x1 <= r.x1 + Pt::new(1e-3));
            }
        }
    }
}

#[test]
fn standard_14_lines_fit_and_overlong_words_break() {
    let text = "Supercalifragilisticexpialidocious is a word wider than this box.";
    let r = rect(50.0, 50.0, 70.0, 700.0);
    let (layout, bytes) = boxed(times, text, r, &TextBoxOptions::default());
    let layout = layout.unwrap();
    assert!(layout.lines.iter().all(|l| l.rect.width().get() <= 70.0));
    let drawn: String = layout
        .lines
        .iter()
        .map(|l| &text[l.content.clone()])
        .collect::<Vec<_>>()
        .join("");
    assert_eq!(drawn.replace(' ', ""), text.replace(' ', ""));
    assert_eq!(
        lines(&reopen(&bytes), 0).concat().replace(' ', ""),
        text.replace(' ', "")
    );
}

#[test]
fn lines_read_back_as_the_paragraph() {
    let doc = reopen(&fixture("golden/build-text-multiscript.pdf"));
    let got = lines(&doc, 1);
    // Latin first, CJK last: each box's lines joined (spaces hang at line ends).
    let latin = SCRIPTS[0].1;
    let n = latin.split(' ').count();
    let joined: String = got
        .iter()
        .take_while(|l| l.is_ascii())
        .cloned()
        .collect::<Vec<_>>()
        .join(" ");
    assert_eq!((joined.as_str(), joined.split(' ').count()), (latin, n));
    let cjk: String = got.iter().rev().take(3).rev().cloned().collect();
    assert_eq!(cjk, SCRIPTS[4].1);
}

#[test]
fn overflow_is_clipped_and_reported() {
    let r = rect(50.0, 50.0, 120.0, 40.0);
    let (layout, bytes) = boxed(noto, LATIN_PARAGRAPH, r, &TextBoxOptions::default());
    let layout = layout.unwrap();
    let rest = layout.overflow.unwrap();
    assert!(layout.lines.iter().all(|l| l.rect.y1 <= r.y1));
    assert_eq!(layout.lines.last().unwrap().range.end, rest);
    assert_eq!(lines(&reopen(&bytes), 0).len(), layout.lines.len());
}

#[test]
fn overflow_error_draws_nothing() {
    let r = rect(50.0, 50.0, 120.0, 40.0);
    let options = TextBoxOptions::default().with_overflow(Overflow::Error);
    let (layout, bytes) = boxed(noto, LATIN_PARAGRAPH, r, &options);
    assert_eq!(layout.unwrap_err().code(), "text-overflow");
    assert_eq!(lines(&reopen(&bytes), 0), Vec::<String>::new());
}

#[test]
fn alignment_places_lines_in_the_box() {
    let r = rect(100.0, 50.0, 200.0, 300.0);
    for (align, check) in [
        (
            Alignment::Start,
            (|l: &RectPt, r: &RectPt| (l.x0 - r.x0).get().abs()) as fn(&RectPt, &RectPt) -> f64,
        ),
        (Alignment::End, |l, r| (l.x1 - r.x1).get().abs()),
        (Alignment::Center, |l, r| ((l.x0 - r.x0) - (r.x1 - l.x1)).get().abs()),
    ] {
        for font in [noto as fn(&mut Builder) -> FontHandle, times] {
            let (layout, _) = boxed(font, LATIN_PARAGRAPH, r, &TextBoxOptions::default().with_align(align));
            for line in layout.unwrap().lines {
                assert!(check(&line.rect, &r) < 1e-3, "{align:?}: {line:?}");
            }
        }
    }
}

#[test]
fn baselines_step_by_the_line_height() {
    let r = rect(50.0, 50.0, 100.0, 700.0);
    let (layout, _) = boxed(
        times,
        LATIN_PARAGRAPH,
        r,
        &TextBoxOptions::default().with_line_height(2.0),
    );
    let layout = layout.unwrap();
    let first = layout.lines[0].baseline.get();
    // Times' ascent is 683/1000 em.
    assert!((first - (50.0 + 0.683 * 11.0)).abs() < 1e-3, "{first}");
    for w in layout.lines.windows(2) {
        assert!(((w[1].baseline - w[0].baseline).get() - 22.0).abs() < 1e-9);
    }
}

#[test]
fn right_to_left_paragraphs_start_on_the_right() {
    let r = rect(50.0, 50.0, 300.0, 300.0);
    let (layout, _) = boxed(
        |b| b.font(text_and_navigation::font("NotoSansHebrew-Regular.ttf")).unwrap(),
        SCRIPTS[3].1,
        r,
        &TextBoxOptions::default(),
    );
    let line = &layout.unwrap().lines[0];
    assert!((line.rect.x1 - r.x1).get().abs() < 1e-3, "{line:?}");
}

#[test]
fn a_box_without_width_is_invalid_geometry() {
    let (layout, _) = boxed(times, "x", rect(50.0, 50.0, 0.0, 10.0), &TextBoxOptions::default());
    assert_eq!(layout.unwrap_err().code(), "invalid-geometry");
}

/// Measures `text` in `r`, then draws it on the same page; returns both layouts and the saved
/// document, and the document with only the drawing.
fn measured_then_drawn(
    file: Option<&str>,
    text: &str,
    r: RectPt,
    options: &TextBoxOptions,
) -> ((TextLayout, TextLayout), (Vec<u8>, Vec<u8>)) {
    let build = |measure: bool| {
        let mut b = Builder::new(BuildOptions::default());
        let f = match file {
            Some(file) => b.font(text_and_navigation::font(file)).unwrap(),
            None => times(&mut b),
        };
        let mut page = b.page(PageSize::A4);
        let measured = measure.then(|| {
            page.surface()
                .measure_text_box(text, &f, Pt::new(11.0), r, options)
                .unwrap()
        });
        let drawn = page.surface().text_box(text, &f, Pt::new(11.0), r, options).unwrap();
        page.finish().unwrap();
        (measured, drawn, save(b.finish().unwrap()))
    };
    let (measured, drawn, with) = build(true);
    let (_, _, without) = build(false);
    ((measured.unwrap(), drawn), (with, without))
}

#[test]
fn measuring_gives_the_layout_drawing_gives_and_draws_nothing() {
    let centered = TextBoxOptions::default().with_align(Alignment::Center);
    let clipped = TextBoxOptions::default().with_overflow(Overflow::Clip);
    let mut cases: Vec<(Option<&str>, &str, RectPt, &TextBoxOptions)> = SCRIPTS
        .iter()
        .flat_map(|(file, text)| {
            [60.0, MULTISCRIPT_BOX_WIDTH, 200.0].map(|w| (Some(*file), *text, rect(50.0, 50.0, w, 700.0), &centered))
        })
        .collect();
    cases.push((None, LATIN_PARAGRAPH, rect(72.0, 72.0, 120.0, 30.0), &clipped));
    cases.push((
        Some("NotoSans-Regular.ttf"),
        LATIN_PARAGRAPH,
        rect(72.0, 72.0, 90.0, 40.0),
        &clipped,
    ));
    for (file, text, r, options) in cases {
        let ((measured, drawn), (with, without)) = measured_then_drawn(file, text, r, options);
        assert_eq!(measured, drawn, "{file:?} in {r:?}");
        // Not `assert_eq!`: a byte diff of two PDFs is unreadable.
        assert!(with == without, "{file:?} in {r:?}: measuring changed the page");
    }
}

#[test]
fn measuring_refuses_what_drawing_refuses() {
    let mut b = Builder::new(BuildOptions::default());
    let f = times(&mut b);
    let mut page = b.page(PageSize::A4);
    let options = TextBoxOptions::default().with_overflow(Overflow::Error);
    let r = rect(72.0, 72.0, 120.0, 20.0);
    let measured = page
        .surface()
        .measure_text_box(LATIN_PARAGRAPH, &f, Pt::new(11.0), r, &options);
    let drawn = page.surface().text_box(LATIN_PARAGRAPH, &f, Pt::new(11.0), r, &options);
    assert_eq!(
        (measured.unwrap_err().code(), drawn.unwrap_err().code()),
        ("text-overflow", "text-overflow")
    );
}
