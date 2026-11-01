//! Every Annex A operator parses with its operand signature; unknown and malformed operators
//! warn and interpretation goes on (ADR 0023 §3).

#![allow(
    clippy::unwrap_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "test helpers outside #[test] fns: a panic here is a test failure"
)]

mod common;

use common::{Event, run, run_content, text_doc};
use laminapdf_content::{ContentIter, GraphicsState, Lexed, Operand, Operator, PathPaint, Signature, Visitor};

/// Valid operands for `signature`, as content bytes; `skip` leaves out the first ones.
fn operands(signature: Signature, skip: usize) -> String {
    match signature {
        Signature::Fixed(kinds) => kinds
            .iter()
            .skip(skip)
            .map(|kind| match kind {
                Operand::Number => "1",
                Operand::Name => "/F1",
                Operand::String => "(a)",
                Operand::Array => "[1 2]",
                Operand::NameOrDict => "<< /MCID 0 >>",
            })
            .collect::<Vec<_>>()
            .join(" "),
        Signature::Color | Signature::ColorOrPattern if skip == 0 => "0.5".to_owned(),
        Signature::Color | Signature::ColorOrPattern => String::new(),
    }
}

/// The content for one operator with valid operands (inline images read whole).
fn valid(op: Operator<'_>) -> String {
    match op {
        Operator::BeginInlineImage => "BI /W 1 /H 1 /BPC 8 /CS /G ID x EI".to_owned(),
        _ => format!(
            "{} {}",
            operands(op.signature(), 0),
            String::from_utf8_lossy(op.keyword())
        ),
    }
}

/// Inline-image data operators only make sense inside `BI … EI`.
fn standalone(op: Operator<'_>) -> bool {
    !matches!(op, Operator::InlineImageData | Operator::EndInlineImage)
}

#[test]
fn every_annex_a_operator_is_read_with_its_operands() {
    for op in Operator::ALL.iter().copied().filter(|op| standalone(*op)) {
        let content = valid(op);
        let ops: Vec<_> = ContentIter::new(content.as_bytes())
            .map(|item| match item.unwrap() {
                Lexed::Op(read) => (read.operator, read.operands.len()),
                Lexed::Issue(issue) => panic!("{content}: {issue:?}"),
            })
            .collect();
        let expected = if op == Operator::BeginInlineImage {
            0
        } else {
            op.signature().min()
        };
        assert_eq!(ops, [(op, expected)], "{content}");
    }
}

#[test]
fn every_annex_a_operator_runs_without_an_operand_warning() {
    for op in Operator::ALL.iter().copied().filter(|op| standalone(*op)) {
        // Inside a text object, so text operators are where they belong.
        let content = format!("BT {} ET", valid(op));
        let (result, rec) = run(&text_doc(content.as_bytes()));
        result.unwrap();
        let codes = rec.codes();
        assert!(
            !codes.contains(&"content-bad-operands") && !codes.contains(&"content-unknown-operator"),
            "{content}: {codes:?}"
        );
    }
}

#[test]
fn every_operator_with_operands_warns_when_one_is_missing() {
    for op in Operator::ALL.iter().copied().filter(|op| op.signature().min() > 0) {
        let fewer = operands(op.signature(), 1);
        let content = format!("BT {fewer} {} ET", String::from_utf8_lossy(op.keyword()));
        let (result, rec) = run(&text_doc(content.as_bytes()));
        result.unwrap();
        assert_eq!(rec.codes(), ["content-bad-operands"], "{content}");
    }
}

#[test]
fn a_mistyped_operand_skips_the_operator() {
    let (_, rec) = run_content(b"/a 2 m 3 4 l S");
    let segments = rec.events.iter().filter(|e| matches!(e, Event::Segment(_))).count();
    assert_eq!((rec.codes(), segments), (vec!["content-bad-operands"], 1));
}

/// Records the line width at each stroke.
#[derive(Default)]
struct Widths(Vec<f64>);

impl Visitor for Widths {
    fn paint_path(&mut self, paint: &PathPaint<'_>) {
        self.0.push(paint.state.line_width);
    }
}

#[test]
fn extra_operands_are_ignored_and_the_last_ones_used() {
    let doc = text_doc(b"");
    let mut widths = Widths::default();
    laminapdf_content::interpret_content(&doc, b"9 2 w S", None, &mut widths).unwrap();
    assert_eq!(widths.0, [2.0]);
}

#[test]
fn extra_operands_warn() {
    let (_, rec) = run_content(b"9 2 w S");
    assert_eq!(rec.codes(), ["content-bad-operands"]);
}

#[test]
fn an_unknown_operator_warns_and_reading_goes_on() {
    let (_, rec) = run_content(b"1 2 foo 0 0 m S");
    assert_eq!((rec.codes(), rec.events.len()), (vec!["content-unknown-operator"], 2));
}

#[test]
fn an_unknown_operator_inside_bx_ex_is_silent() {
    let (_, rec) = run_content(b"BX 1 foo EX");
    assert_eq!(rec.warnings, []);
}

#[test]
fn an_unknown_operator_after_ex_warns_again() {
    let (_, rec) = run_content(b"BX EX foo");
    assert_eq!(rec.codes(), ["content-unknown-operator"]);
}

#[test]
fn junk_bytes_are_skipped_with_content_syntax() {
    let (result, rec) = run_content(b"0 0 m ) 1 1 l S");
    result.unwrap();
    assert_eq!((rec.codes(), rec.events.len()), (vec!["content-syntax"], 3));
}

#[test]
fn a_stray_ei_is_content_syntax() {
    let (_, rec) = run_content(b"EI");
    assert_eq!(rec.codes(), ["content-syntax"]);
}

#[test]
fn an_inline_image_is_reported_with_the_ctm() {
    let (_, rec) = run_content(b"q 20 0 0 10 5 5 cm BI /W 2 /H 1 /BPC 8 /CS /G ID ab EI Q");
    assert_eq!(
        rec.events,
        [Event::Image {
            inline: true,
            ctm: laminapdf_content::Matrix::new(20.0, 0.0, 0.0, 10.0, 5.0, 5.0)
        }]
    );
}

#[test]
fn an_unterminated_inline_image_ends_the_stream_with_a_warning() {
    let (result, rec) = run_content(b"BI /W 9 /H 9 /F /DCT ID \x01\x02 0 0 m");
    result.unwrap();
    assert_eq!((rec.codes(), rec.events.len()), (vec!["inline-image-unterminated"], 0));
}

#[test]
fn every_operator_is_reported_to_the_visitor_in_order() {
    let (_, rec) = run_content(b"q 1 w foo Q");
    let keywords: Vec<_> = rec.operators.iter().map(|(k, _)| k.as_slice()).collect();
    assert_eq!(keywords, [&b"q"[..], b"w", b"foo", b"Q"]);
}

#[test]
fn the_initial_graphics_state_follows_table_51() {
    let state = GraphicsState::default();
    assert_eq!(
        (
            state.line_width,
            state.miter_limit,
            state.flatness,
            state.fill.components
        ),
        (1.0, 10.0, 1.0, vec![0.0])
    );
}

#[test]
fn path_operators_report_segments_then_the_paint() {
    let (_, rec) = run_content(b"0 0 m 1 1 l 1 1 2 2 v 2 2 3 3 y h 0 0 5 5 re W* n");
    let kinds: Vec<_> = rec
        .events
        .iter()
        .map(|e| match e {
            Event::Segment(_) => "segment",
            Event::Paint(p) if p.clip == Some(laminapdf_content::FillRule::EvenOdd) => "clip",
            _ => "other",
        })
        .collect();
    assert_eq!(
        kinds,
        ["segment", "segment", "segment", "segment", "segment", "segment", "clip"]
    );
}

// Review round 1.

/// Records each segment with the CTM it came with, and each paint.
#[derive(Default)]
struct Segments(
    Vec<(laminapdf_content::PathSegment, f64)>,
    Vec<laminapdf_content::Paint>,
);

impl Visitor for Segments {
    fn path_segment(&mut self, segment: &laminapdf_content::PathSegment, ctm: &laminapdf_content::Matrix) {
        self.0.push((*segment, ctm.a));
    }

    fn paint_path(&mut self, paint: &PathPaint<'_>) {
        self.1.push(paint.paint);
    }
}

fn segments(content: &[u8]) -> Segments {
    let doc = text_doc(b"");
    let mut segments = Segments::default();
    laminapdf_content::interpret_content(&doc, content, None, &mut segments).unwrap();
    segments
}

#[test]
fn v_and_y_are_expanded_to_full_curves() {
    use laminapdf_content::PathSegment::CurveTo;
    let curves: Vec<_> = segments(b"1 2 m 3 4 5 6 v 7 8 9 10 y S")
        .0
        .into_iter()
        .filter_map(|(s, _)| match s {
            CurveTo { c1, c2, end } => Some((c1, c2, end)),
            _ => None,
        })
        .collect();
    assert_eq!(
        curves,
        [
            ((1.0, 2.0), (3.0, 4.0), (5.0, 6.0)),
            ((7.0, 8.0), (9.0, 10.0), (9.0, 10.0))
        ]
    );
}

#[test]
fn each_segment_carries_the_ctm_it_was_built_under() {
    let ctms: Vec<_> = segments(b"0 0 m 2 0 0 2 0 0 cm 1 1 l S")
        .0
        .iter()
        .map(|(_, a)| *a)
        .collect();
    assert_eq!(ctms, [1.0, 2.0]);
}

#[test]
fn an_unpainted_path_is_discarded_at_the_end_of_the_stream() {
    assert_eq!(segments(b"0 0 m 1 1 l").1, [laminapdf_content::Paint::Discarded]);
}

#[test]
fn an_unpainted_path_warns() {
    let (_, rec) = run_content(b"0 0 m 1 1 l");
    assert_eq!(rec.codes(), ["content-path-unpainted"]);
}
