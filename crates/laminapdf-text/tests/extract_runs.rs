//! Text extraction, geometry and runs (todo B9d): glyph quads in page space against values
//! computed by hand from ISO 32000-2 §9.4.4 and ADR 0027 (rotation, `/UserUnit`, the `CropBox`
//! origin, vertical writing), and runs with fonts, `ActualText` spans, MCIDs and invisible
//! text.
#![allow(
    clippy::expect_used,
    clippy::indexing_slicing,
    reason = "tests: a panic is a test failure"
)]

use laminapdf_cos::OpenOptions;
use laminapdf_model::{PointPt, Pt};
use laminapdf_text::{PageText, Quad};

mod common;

use common::extract::{extract, font, helvetica, page, page_with};

fn text(content: &[u8]) -> PageText {
    extract(&page(content, vec![helvetica(), times()], OpenOptions::default())).expect("extracts")
}

fn times() -> laminapdf_syntax::Dict {
    font("<< /Type /Font /Subtype /Type1 /BaseFont /Times-Roman /Encoding /WinAnsiEncoding >>")
}

/// A quad's corners rounded to 1/100 pt.
fn rounded(quad: &Quad) -> Vec<(f64, f64)> {
    let r = |v: Pt| (v.get() * 100.0).round() / 100.0;
    quad.iter().map(|p: &PointPt| (r(p.x), r(p.y))).collect()
}

/// `A` in Helvetica 10 at (72, 700): width 667, AFM ascent 718 and descent −207, so the box in
/// user space is x 72 → 78.67, y 697.93 → 707.18.
const A_AT_72_700: &[u8] = b"BT /F1 10 Tf 72 700 Td (A) Tj ET";

/// capability: A.10 core read
#[test]
fn a_quad_spans_the_advance_and_the_fonts_extent_in_page_space() {
    // Letter, unrotated: page y = 792 − user y.
    assert_eq!(
        rounded(&text(A_AT_72_700).glyphs[0].quad),
        [(72.0, 94.07), (78.67, 94.07), (78.67, 84.82), (72.0, 84.82)]
    );
}

#[test]
fn rotated_pages_turn_quads_into_the_displayed_page() {
    let doc = page_with(
        A_AT_72_700,
        vec![helvetica()],
        vec![("Rotate", 90.into())],
        OpenOptions::default(),
    );
    // /Rotate 90 (ADR 0027 §3): page (x, y) = (user y − y0, user x − x0).
    assert_eq!(
        rounded(&extract(&doc).expect("extracts").glyphs[0].quad),
        [(697.93, 72.0), (697.93, 78.67), (707.18, 78.67), (707.18, 72.0)]
    );
}

#[test]
fn user_unit_scales_quads() {
    let doc = page_with(
        A_AT_72_700,
        vec![helvetica()],
        vec![("UserUnit", 2.into())],
        OpenOptions::default(),
    );
    assert_eq!(
        rounded(&extract(&doc).expect("extracts").glyphs[0].quad),
        [(144.0, 188.14), (157.34, 188.14), (157.34, 169.64), (144.0, 169.64)]
    );
}

#[test]
fn quads_are_relative_to_the_crop_box() {
    let crop = vec![10.into(), 20.into(), 600.into(), 780.into()];
    let doc = page_with(
        A_AT_72_700,
        vec![helvetica()],
        vec![("CropBox", crop.into())],
        OpenOptions::default(),
    );
    // page x = user x − 10, page y = 780 − user y.
    assert_eq!(
        rounded(&extract(&doc).expect("extracts").glyphs[0].quad),
        [(62.0, 82.07), (68.67, 82.07), (68.67, 72.82), (62.0, 72.82)]
    );
}

#[test]
fn vertical_glyphs_hang_from_their_position_vector() {
    // Identity-V, /DW 1000, /DW2 [880 −1000] by default, descriptor ascent 880 and descent
    // −120: v = (500, 880), so the box around the pen (100, 500) at 10 pt is x 95 → 105 and
    // y 490 → 500 in user space.
    let vertical = font(
        "<< /Type /Font /Subtype /Type0 /BaseFont /V /Encoding /Identity-V /DescendantFonts [<< /Subtype
           /CIDFontType0 /FontDescriptor << /Ascent 880 /Descent -120 >> >>] >>",
    );
    let doc = page(
        b"BT /F1 10 Tf 100 500 Td <0001> Tj ET",
        vec![vertical],
        OpenOptions::default(),
    );
    let glyph = &extract(&doc).expect("extracts").glyphs[0];
    assert_eq!(
        (rounded(&glyph.quad), glyph.advance, glyph.vertical),
        (
            vec![(95.0, 302.0), (105.0, 302.0), (105.0, 292.0), (95.0, 292.0)],
            (0.0, -10.0),
            true
        )
    );
}

#[test]
fn a_run_ends_where_the_font_or_the_line_changes() {
    let text = text(b"BT /F1 10 Tf 72 700 Td (Hello ) Tj /F2 10 Tf (world) Tj 0 -14 Td (again) Tj ET");
    let runs: Vec<(&str, &str)> = text.runs.iter().map(|r| (r.text.as_str(), &*r.font_name)).collect();
    assert_eq!(
        (runs, text.lines()),
        (
            vec![
                ("Hello ", "Helvetica"),
                ("world", "Times-Roman"),
                ("again", "Times-Roman")
            ],
            vec!["Hello world".to_owned(), "again".to_owned()]
        )
    );
}

#[test]
fn run_bounds_cover_their_glyphs() {
    let bounds = text(b"BT /F1 10 Tf 72 700 Td (AA) Tj ET").runs[0].bounds;
    let r = |v: Pt| (v.get() * 100.0).round() / 100.0;
    assert_eq!(
        [r(bounds.x0), r(bounds.y0), r(bounds.x1), r(bounds.y1)],
        [72.0, 84.82, 85.34, 94.07]
    );
}

#[test]
fn actual_text_replaces_a_runs_text_and_keeps_its_glyphs() {
    // UTF-16BE with a byte order mark: U+0915 U+093F, "कि".
    let text = text(b"BT /F1 10 Tf 72 700 Td /Span << /ActualText <FEFF0915093F> >> BDC (AB) Tj EMC (C) Tj ET");
    let run = &text.runs[0];
    assert_eq!(
        (
            run.text.as_str(),
            run.actual_text,
            run.glyphs.clone(),
            text.glyphs[0].text.as_str(),
            text.lines()
        ),
        ("कि", true, 0..2, "A", vec!["किC".to_owned()])
    );
}

#[test]
fn an_actual_text_span_over_several_runs_gives_its_text_once() {
    let text = text(b"BT 72 700 Td /Span << /ActualText (ffi) >> BDC /F1 10 Tf (f) Tj /F2 10 Tf (fi) Tj EMC ET");
    let runs: Vec<&str> = text.runs.iter().map(|r| r.text.as_str()).collect();
    assert_eq!((runs, text.lines()), (vec!["ffi", ""], vec!["ffi".to_owned()]));
}

#[test]
fn glyphs_carry_the_innermost_mcid_and_the_outermost_actual_text() {
    let text = text(
        b"BT /F1 10 Tf 72 700 Td /P << /MCID 1 >> BDC /Span << /MCID 2 /ActualText (x) >> BDC
          /Span << /ActualText (y) >> BDC (A) Tj EMC EMC (B) Tj EMC (C) Tj ET",
    );
    let marks: Vec<(Option<i64>, Option<usize>)> = text.glyphs.iter().map(|g| (g.mcid, g.actual_text)).collect();
    assert_eq!(
        (marks, text.actual_texts),
        (
            vec![(Some(2), Some(0)), (Some(1), None), (None, None)],
            vec!["x".to_owned(), "y".to_owned()]
        )
    );
}

#[test]
fn mcids_split_runs() {
    let text = text(b"BT /F1 10 Tf 72 700 Td /P << /MCID 7 >> BDC (A) Tj EMC (B) Tj ET");
    let runs: Vec<(&str, Option<i64>)> = text.runs.iter().map(|r| (r.text.as_str(), r.mcid)).collect();
    assert_eq!(runs, [("A", Some(7)), ("B", None)]);
}

#[test]
fn invisible_text_is_flagged_and_visible_drops_it() {
    let text = text(b"BT /F1 10 Tf 72 700 Td 3 Tr (OCR) Tj 0 Tr ( drawn) Tj ET");
    let flags: Vec<(&str, bool)> = text.runs.iter().map(|r| (r.text.as_str(), r.invisible)).collect();
    assert_eq!(
        (flags, text.lines(), text.clone().visible().lines()),
        (
            vec![("OCR", true), (" drawn", false)],
            vec!["OCR drawn".to_owned()],
            vec!["drawn".to_owned()]
        )
    );
}

#[test]
fn right_to_left_runs_drawn_in_logical_order_come_out_in_logical_order() {
    // "שלום עולם" as two runs in logical order, each in visual order: "שלום" first, at the
    // right (x 150 → 170: ם ו ל ש), then " עולם" to its left (x 125 → 150: ם ל ו ע, then the
    // space). Every glyph is 500/1000 wide (`/MissingWidth`).
    let hebrew = font(
        "<< /Type /Font /Subtype /Type1 /BaseFont /H /FontDescriptor << /Flags 32 /MissingWidth 500 >>
           /Encoding << /Differences [1 /uni05E9 /uni05DC /uni05D5 /uni05DD /uni05E2 32 /space] >> >>",
    );
    let doc = page(
        b"BT /F1 10 Tf 150 700 Td <04030201> Tj -25 0 Td <0402030520> Tj ET",
        vec![hebrew],
        OpenOptions::default(),
    );
    let text = extract(&doc).expect("extracts");
    let runs: Vec<&str> = text.runs.iter().map(|r| r.text.as_str()).collect();
    assert_eq!(
        (runs, text.lines()),
        (vec!["שלום", " עולם"], vec!["שלום עולם".to_owned()])
    );
}

/// A Hebrew font: code 1 ש, 2 ל, 3 ו, 4 ם, 5 ע, 32 space, each `width` thousandths wide.
fn hebrew(name: &str, width: u32) -> laminapdf_syntax::Dict {
    font(&format!(
        "<< /Type /Font /Subtype /Type1 /BaseFont /{name} /FontDescriptor << /Flags 32 /MissingWidth {width} >>
           /Encoding << /Differences [1 /uni05E9 /uni05DC /uni05D5 /uni05DD /uni05E2 32 /space] >> >>"
    ))
}

#[test]
fn a_right_to_left_run_without_widths_keeps_its_text() {
    // No glyph moves the pen: every glyph is a letter, none is dropped as a mark.
    let doc = page(
        b"BT /F1 10 Tf 100 700 Td <01> Tj 5 0 Td <02> Tj ET",
        vec![hebrew("H", 0)],
        OpenOptions::default(),
    );
    let text = extract(&doc).expect("extracts").lines().concat();
    assert_eq!(text.chars().filter(|c| !c.is_whitespace()).count(), 2, "{text}");
}

#[test]
fn a_gap_after_a_zero_width_mark_is_a_space() {
    let accented = font(
        "<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica
           /Encoding << /BaseEncoding /WinAnsiEncoding /Differences [1 /uni0301] >> >>",
    );
    let doc = page(
        b"BT /F1 10 Tf 72 700 Td [(cafe) <01> -400 (bar)] TJ ET",
        vec![accented],
        OpenOptions::default(),
    );
    assert_eq!(extract(&doc).expect("extracts").lines(), ["cafe\u{301} bar"]);
}

#[test]
fn a_visual_order_right_to_left_line_split_by_fonts_comes_out_in_logical_order() {
    // "שלום עולם" drawn left to right as it is displayed: "עולם" (bold, x 100 → 120: ם ל ו ע),
    // a gap, then "שלום" (regular, x 130 → 150: ם ו ל ש). The font change makes two runs.
    let doc = page(
        b"BT /F2 10 Tf 100 700 Td <04020305> Tj /F1 10 Tf 30 0 Td <04030201> Tj ET",
        vec![hebrew("Regular", 500), hebrew("Bold", 500)],
        OpenOptions::default(),
    );
    assert_eq!(extract(&doc).expect("extracts").lines(), ["שלום עולם"]);
}

#[test]
fn cells_drawn_right_to_left_on_one_line_are_separated() {
    // Three cells of a row drawn last first: "C" at x 200, "B" at 150, "A" at 100.
    let text = text(b"BT /F1 10 Tf 200 700 Td (Gamma) Tj -50 0 Td (Beta) Tj -50 0 Td (Alpha) Tj ET");
    assert_eq!(text.lines(), ["Gamma Beta Alpha"]);
}

#[test]
fn a_small_step_back_between_runs_adds_no_space() {
    // A kerning-sized correction (a TJ number of 100: 1 pt back at 10 pt) before the last
    // letter, drawn in another font: two runs that overlap by 1 pt.
    let text = text(b"BT /F1 10 Tf 100 700 Td [(Wor) 100] TJ /F2 10 Tf (d) Tj ET");
    assert_eq!((text.runs.len(), text.lines()), (2, vec!["Word".to_owned()]));
}

#[test]
fn an_accent_drawn_over_its_letter_in_another_font_adds_no_space() {
    // "e" in Helvetica (556 wide), then back over it with `Td` for a combining acute from
    // another font: the second run lies inside the first.
    let accent = font(
        "<< /Type /Font /Subtype /Type1 /BaseFont /Accents /FontDescriptor << /Flags 32 /MissingWidth 0 >>
           /Encoding << /Differences [1 /uni0301] >> >>",
    );
    let doc = page(
        b"BT /F1 10 Tf 100 700 Td (caf) Tj (e) Tj 15.01 0 Td /F2 10 Tf <01> Tj ET",
        vec![helvetica(), accent],
        OpenOptions::default(),
    );
    assert_eq!(extract(&doc).expect("extracts").lines(), ["cafe\u{301}"]);
}

#[test]
fn a_span_drawn_in_two_runs_gets_no_space_inside_its_actual_text() {
    // "É" as E, then a jump back and an accent from another font, inside one `ActualText`.
    let text = text(b"BT /F1 10 Tf 100 700 Td /Span << /ActualText (\\311) >> BDC (E) Tj -1 0 Td /F2 10 Tf (\\264) Tj EMC (COLE) Tj ET");
    assert_eq!(text.lines(), ["\u{c9}COLE"]);
}

#[test]
fn glyphs_drawn_backwards_one_by_one_get_no_spaces() {
    // Four glyphs, each drawn just left of the previous one (`Td` back by its own width from
    // the previous line start): they abut. D and C are 722 wide, B and A 667.
    let doc = page(
        b"BT /F1 10 Tf 200 700 Td (D) Tj -7.22 0 Td (C) Tj -6.67 0 Td (B) Tj -6.67 0 Td (A) Tj ET",
        vec![helvetica()],
        OpenOptions::default(),
    );
    assert_eq!(
        extract(&doc).expect("extracts").lines().concat().matches(' ').count(),
        0
    );
}
