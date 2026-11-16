use laminapdf_cos::Limits;

use super::super::{ContentOps, ContentOwner, EncodedPart, NotEditable};
use super::ShownGlyph;
use crate::state::GraphicsState;

/// An IR of `content` whose objects start in the default state, as a recorder leaves them.
fn ir(content: &[u8]) -> ContentOps {
    let owner = ContentOwner::Page("p3.0".parse().unwrap());
    let mut ops = ContentOps::from_parts(owner, vec![(None, content.to_vec())], &Limits::default()).unwrap();
    for object in &mut ops.objects {
        object.state = Some(Box::new(GraphicsState::default()));
        object.end_state = Some(Box::new(GraphicsState::default()));
    }
    ops
}

fn written(ops: &ContentOps) -> String {
    let encoded = ops.encode_every_part();
    let parts: Vec<&[u8]> = encoded.parts.iter().filter_map(EncodedPart::bytes).collect();
    String::from_utf8(parts.join(&b'\n')).unwrap()
}

/// One-byte glyphs for `text`, each 500 thousandths wide; a space is a word space.
fn glyphs(text: &str) -> Vec<ShownGlyph> {
    text.bytes().map(|b| ShownGlyph::new(u32::from(b), 1, 500.0, b == b' ')).collect()
}

fn rewrite(content: &[u8], shown: &str, range: std::ops::Range<usize>, new: &str) -> (String, f64) {
    let mut ops = ir(content);
    let id = ops.objects()[0].id();
    let shift = ops.rewrite_text(id, &glyphs(shown), range, &glyphs(new)).unwrap();
    (written(&ops), shift)
}

#[test]
fn replacing_a_word_should_write_the_new_codes_in_place() {
    let (out, shift) = rewrite(b"BT /F1 10 Tf (Hello world) Tj ET", "Hello world", 6..11, "there");
    assert_eq!(out, "BT\n/F1 10 Tf\n(Hello there) Tj\nET\n");
    assert!(shift.abs() < 1e-9);
}

#[test]
fn a_longer_word_should_report_the_width_it_adds() {
    let (_, shift) = rewrite(b"BT /F1 10 Tf (Hi) Tj ET", "Hi", 0..2, "Hello");
    // Three more glyphs of 500/1000 × 10 points.
    assert!((shift - 15.0).abs() < 1e-9);
}

#[test]
fn an_empty_replacement_should_delete_the_range() {
    let (out, _) = rewrite(b"BT /F1 10 Tf (Hello world) Tj ET", "Hello world", 5..11, "");
    assert_eq!(out, "BT\n/F1 10 Tf\n(Hello) Tj\nET\n");
}

#[test]
fn numbers_inside_the_range_should_go_and_the_rest_stay() {
    let (out, shift) = rewrite(b"BT /F1 10 Tf [(AB) -200 (CD) 100 (EF)] TJ ET", "ABCDEF", 1..4, "x");
    assert_eq!(out, "BT\n/F1 10 Tf\n[(Ax) 100 (EF)] TJ\nET\n");
    // Three glyphs (15) and a 2-point kern out, one glyph (5) in.
    assert!((shift + 12.0).abs() < 1e-9);
}

#[test]
fn the_next_move_on_the_line_should_take_the_difference_and_the_next_line_take_it_back() {
    let content = b"BT /F1 10 Tf 0 0 Td (Hi) Tj 30 0 Td (there) Tj 0 -12 Td (next) Tj ET";
    let (out, _) = rewrite(content, "Hitherenext", 0..2, "Hello");
    assert_eq!(
        out,
        "BT\n/F1 10 Tf\n0 0 Td\n(Hello) Tj\n45 0 Td\n(there) Tj\n-15 -12 Td\n(next) Tj\nET\n"
    );
}

#[test]
fn a_line_change_without_operands_should_get_a_move_back_first() {
    let content = b"BT /F1 10 Tf 12 TL (Hi) Tj 30 0 Td (there) Tj T* (next) Tj ET";
    let (out, _) = rewrite(content, "Hitherenext", 0..2, "Hello");
    assert_eq!(
        out,
        "BT\n/F1 10 Tf\n12 TL\n(Hello) Tj\n45 0 Td\n(there) Tj\n-15 0 Td\nT*\n(next) Tj\nET\n"
    );
}

#[test]
fn a_tj_left_with_numbers_should_become_a_tj_array() {
    let content = b"BT /F1 10 Tf [(AB) -200 (CD)] TJ ET";
    let (out, _) = rewrite(content, "ABCD", 0..1, "");
    assert_eq!(out, "BT\n/F1 10 Tf\n[(B) -200 (CD)] TJ\nET\n");
}

#[test]
fn codes_that_do_not_match_the_glyphs_should_be_refused() {
    let mut ops = ir(b"BT /F1 10 Tf (Hello) Tj ET");
    let id = ops.objects()[0].id();
    let result = ops.rewrite_text(id, &glyphs("Jello"), 0..1, &glyphs("M"));
    assert_eq!(result, Err(NotEditable::ContentUnreadable));
}

#[test]
fn clipping_text_should_be_refused() {
    let mut ops = ir(b"BT /F1 10 Tf 7 Tr (Hello) Tj ET");
    let id = ops.objects()[0].id();
    let mut state = GraphicsState::default();
    state.text.render_mode = 7;
    ops.objects[0].state = Some(Box::new(state));
    let result = ops.rewrite_text(id, &glyphs("Hello"), 0..1, &glyphs("J"));
    assert_eq!(result, Err(NotEditable::ClippingPath));
}
