//! The spike's content-stream builder behind the frozen goldens (test support in
//! `common/content.rs`):
//! number formatting, operator grouping and escaping.

mod common;

use common::content::{ContentBuilder, fixed};

#[test]
fn builds_hello_world_text_object() {
    let mut b = ContentBuilder::new();
    b.begin_text()
        .font("F1", 24)
        .text_move(72, 720)
        .show_text("Hello, World")
        .end_text();
    assert_eq!(b.as_bytes(), b"BT\n/F1 24 Tf\n72 720 Td\n(Hello, World) Tj\nET");
}

#[test]
fn fixed_precision_keeps_trailing_zeros() {
    let mut b = ContentBuilder::new();
    b.fill_rgb(fixed(0.2, 2), fixed(0.4, 2), fixed(0.8, 2));
    assert_eq!(b.as_bytes(), b"0.20 0.40 0.80 rg");
}

#[test]
fn fixed_rounds_exact_tie_half_to_even_down() {
    let mut b = ContentBuilder::new();
    b.line_width(fixed(0.125, 2));
    assert_eq!(b.as_bytes(), b"0.12 w");
}

#[test]
fn fixed_rounds_exact_tie_half_to_even_up() {
    let mut b = ContentBuilder::new();
    b.line_width(fixed(0.375, 2));
    assert_eq!(b.as_bytes(), b"0.38 w");
}

#[test]
fn fixed_rounds_exact_tie_at_zero_decimals_to_even() {
    let mut b = ContentBuilder::new();
    b.line_width(fixed(2.5, 0));
    assert_eq!(b.as_bytes(), b"2 w");
}

#[test]
fn auto_numbers_are_trimmed() {
    let mut b = ContentBuilder::new();
    b.fill_rgb(0, 0.6, 0.3);
    assert_eq!(b.as_bytes(), b"0 0.6 0.3 rg");
}

#[test]
fn line_joins_operators_with_spaces() {
    let mut b = ContentBuilder::new();
    b.save_state().line(|l| {
        l.rect(1, 2, 3, 4).clip().end_path();
    });
    b.restore_state();
    assert_eq!(b.as_bytes(), b"q\n1 2 3 4 re W n\nQ");
}

#[test]
fn comment_forces_a_newline_even_inside_a_line() {
    let mut b = ContentBuilder::new();
    b.line(|l| {
        l.comment("c").stroke();
    });
    assert_eq!(b.as_bytes(), b"% c\nS");
}

#[test]
fn dash_formats_pattern_array() {
    let mut b = ContentBuilder::new();
    b.dash(&[18.0, 6.0], 0);
    assert_eq!(b.as_bytes(), b"[18 6] 0 d");
}

#[test]
fn show_text_escapes_parentheses() {
    let mut b = ContentBuilder::new();
    b.show_text("a(b)");
    assert_eq!(b.as_bytes(), b"(a\\(b\\)) Tj");
}

#[test]
fn circle_matches_python_reference_first_curve() {
    let mut b = ContentBuilder::new();
    b.circle(430.0, 660.0, 70.0);
    let text = String::from_utf8_lossy(b.as_bytes()).into_owned();
    assert_eq!(
        text.lines().nth(1),
        Some("500.000 698.660 468.660 730.000 430.000 730.000 c")
    );
}
