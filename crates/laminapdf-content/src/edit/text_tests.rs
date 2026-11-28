use laminapdf_cos::Limits;

use super::super::{ContentOps, ContentOwner, EncodedPart, NotEditable};
use super::super::{FillColour, LineShift, Segment, SegmentStyle};
use super::ShownGlyph;
use crate::state::GraphicsState;
use laminapdf_syntax::Name;

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
    text.bytes()
        .map(|b| ShownGlyph::new(u32::from(b), 1, 500.0, b == b' '))
        .collect()
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

#[test]
fn a_gap_should_be_written_as_a_tj_number() {
    let mut ops = ir(b"BT /F1 10 Tf (AB) Tj ET");
    let id = ops.objects()[0].id();
    let new = [glyphs("A")[0], ShownGlyph::gap(250.0), glyphs("B")[0]];
    let shift = ops.rewrite_text(id, &glyphs("AB"), 0..2, &new).unwrap();
    assert_eq!(written(&ops), "BT\n/F1 10 Tf\n[(A) -250 (B)] TJ\nET\n");
    assert!((shift - 2.5).abs() < 1e-9);
}

fn styled(content: &[u8], shown: &str, range: std::ops::Range<usize>, new: &str, style: SegmentStyle) -> (String, f64) {
    let mut ops = ir(content);
    let id = ops.objects()[0].id();
    let segment = Segment::new(glyphs(new)).styled(style);
    let shift = ops
        .rewrite_segments(id, &glyphs(shown), range, &[segment])
        .unwrap()
        .shift();
    (written(&ops), shift)
}

#[test]
fn a_styled_segment_should_set_its_font_and_colour_and_put_the_object_s_back() {
    let style = SegmentStyle::new()
        .with_font(Name::new("F2"))
        .with_scale(2.0)
        .with_fill(FillColour::Rgb(1.0, 0.0, 0.0));
    let (out, shift) = styled(
        b"BT /F1 10 Tf (Hello world) Tj ET",
        "Hello world",
        6..11,
        "there",
        style,
    );
    assert_eq!(
        out,
        "BT\n/F1 10 Tf\n(Hello ) Tj\n/F2 20 Tf\n1 0 0 rg\n(there) Tj\n/F1 10 Tf\n0 g\nET\n"
    );
    // Five glyphs of 500/1000 at 20 points replace five at 10.
    assert!((shift - 25.0).abs() < 1e-9);
}

#[test]
fn a_styled_segment_should_leave_the_text_after_it_in_the_object_s_style() {
    let style = SegmentStyle::new().with_rise(0.4);
    let (out, _) = styled(
        b"BT /F1 10 Tf 2 Ts (Hello world) Tj ET",
        "Hello world",
        0..5,
        "Howdy",
        style,
    );
    assert_eq!(out, "BT\n/F1 10 Tf\n2 Ts\n4 Ts\n(Howdy) Tj\n2 Ts\n( world) Tj\nET\n");
}

#[test]
fn a_styled_segment_should_keep_the_line_move_of_a_quote_operator() {
    let style = SegmentStyle::new().with_font(Name::new("F2"));
    let (out, _) = styled(b"BT /F1 10 Tf 12 TL (one) Tj (two) ' ET", "onetwo", 3..6, "TWO", style);
    // The quote operator's line move stays, shown empty, before the styled text.
    assert!(out.contains("() '\n/F2 10 Tf\n(TWO) Tj\n/F1 10 Tf"), "{out}");
}

#[test]
fn a_style_that_cannot_be_put_back_should_be_refused() {
    let mut ops = ir(b"BT (Hello) Tj ET");
    let id = ops.objects()[0].id();
    let style = SegmentStyle::new().with_font(Name::new("F2"));
    let segment = Segment::new(glyphs("J")).styled(style);
    let result = ops.rewrite_segments(id, &glyphs("Hello"), 0..1, &[segment]);
    assert_eq!(result, Err(NotEditable::ContentUnreadable));
}

#[test]
fn lines_should_move_by_text_space_amounts_and_move_back_after() {
    let mut ops = ir(b"BT /F1 10 Tf 0 0 Td (one) Tj 0 -12 Td (two) Tj ET");
    let id = ops.objects()[0].id();
    // Glyphs drawn at 20 points where the font size is 10: text space is half of user space.
    ops.shift_lines(id, &[LineShift::new(1, 1, 20.0, 8.0, -4.0)]).unwrap();
    assert_eq!(
        written(&ops),
        "BT\n/F1 10 Tf\n0 0 Td\n(one) Tj\n0 -12 Td\n4 -2 Td\n(two) Tj\n-4 2 Td\nET\n"
    );
}

#[test]
fn a_line_shift_naming_a_missing_show_should_be_refused() {
    let mut ops = ir(b"BT /F1 10 Tf (one) Tj ET");
    let id = ops.objects()[0].id();
    let result = ops.shift_lines(id, &[LineShift::new(3, 3, 10.0, 1.0, 0.0)]);
    assert_eq!(result, Err(NotEditable::ContentUnreadable));
}

#[test]
fn restyling_a_range_should_keep_its_glyphs_and_numbers_and_move_the_line_by_the_new_width() {
    let mut ops = ir(b"BT /F1 10 Tf [(AB) -200 (CD)] TJ ET");
    let id = ops.objects()[0].id();
    let style = SegmentStyle::new().with_scale(2.0).with_fill(FillColour::Gray(0.5));
    let shift = ops.restyle_text(id, &glyphs("ABCD"), 0..4, &style).unwrap().shift();
    assert_eq!(
        written(&ops),
        "BT\n/F1 10 Tf\n/F1 20 Tf\n0.5 g\n[(AB) -200 (CD)] TJ\n/F1 10 Tf\n0 g\nET\n"
    );
    // Four glyphs of 5 points and a 2-point gap each grow by their own width and gap again.
    assert!((shift - 22.0).abs() < 1e-9);
}

#[test]
fn restyling_a_range_across_two_show_operators_should_be_refused() {
    let mut ops = ir(b"BT /F1 10 Tf (AB) Tj (CD) Tj ET");
    let id = ops.objects()[0].id();
    let style = SegmentStyle::new().with_scale(2.0);
    let result = ops.restyle_text(id, &glyphs("ABCD"), 0..4, &style);
    assert_eq!(result, Err(NotEditable::ContentUnreadable));
}

#[test]
fn a_line_that_starts_without_a_line_move_should_not_be_shifted() {
    // The second show continues the first one's line: a `Td` would restart from the line's start.
    let mut ops = ir(b"BT /F1 10 Tf (one) Tj (two) Tj ET");
    let id = ops.objects()[0].id();
    let result = ops.shift_lines(id, &[LineShift::new(1, 1, 10.0, 5.0, 0.0)]);
    assert_eq!(result, Err(NotEditable::ContentUnreadable));
}

#[test]
fn text_that_follows_a_shifted_line_without_a_line_move_should_be_refused() {
    let mut ops = ir(b"BT /F1 10 Tf (one) Tj (two) Tj ET");
    let id = ops.objects()[0].id();
    let result = ops.shift_lines(id, &[LineShift::new(0, 0, 10.0, 5.0, 0.0)]);
    assert_eq!(result, Err(NotEditable::ContentUnreadable));
}

#[test]
fn widths_should_include_character_spacing_and_horizontal_scaling() {
    let mut ops = ir(b"BT /F1 10 Tf 2 Tc 50 Tz (Hi) Tj ET");
    let id = ops.objects()[0].id();
    let widths = ops
        .rewrite_segments(id, &glyphs("Hi"), 0..2, &[Segment::new(glyphs("Hi"))])
        .unwrap();
    // Two glyphs of (5 + 2) × 0.5 points, put back as the same two.
    assert!((widths.removed - 7.0).abs() < 1e-9, "{widths:?}");
    assert!(widths.shift().abs() < 1e-9);
}

#[test]
fn a_tm_inside_a_shifted_line_should_be_refused() {
    let mut ops = ir(b"BT /F1 10 Tf (one) Tj 1 0 0 1 50 0 Tm (two) Tj ET");
    let id = ops.objects()[0].id();
    let result = ops.shift_lines(id, &[LineShift::new(0, 1, 10.0, 5.0, 0.0)]);
    assert_eq!(result, Err(NotEditable::ContentUnreadable));
}
