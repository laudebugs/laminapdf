use laminapdf_model::{UserSpacePoint, UserSpaceRect};
use laminapdf_ops::{
    AnnotColor, AnnotSpec, Border, Ink, InkStroke, Leader, LineEnding, LineMark, Poly, PolyKind, Shape, ShapeKind,
};

use super::super::{Paint, draw as draw_any};
use super::*;

fn p(x: f64, y: f64) -> UserSpacePoint {
    UserSpacePoint::new(x, y)
}

const PAINT: Paint = Paint {
    color: None,
    opacity: 1.0,
    intent: None,
};

fn drawn(spec: &AnnotSpec) -> Drawn {
    draw(spec, &PAINT).unwrap()
}

fn text(spec: &AnnotSpec) -> String {
    String::from_utf8(drawn(spec).content()).unwrap()
}

fn rect(r: UserSpaceRect) -> [f64; 4] {
    r.to_array().map(|v| (v * 1000.0).round() / 1000.0)
}

#[test]
fn a_square_is_boxed_by_its_outline_and_half_the_stroke() {
    let spec = AnnotSpec::Shape(
        Shape::new(ShapeKind::Square, UserSpaceRect::from_corners(10.0, 10.0, 50.0, 30.0))
            .with_border(Border::solid(4.0)),
    );
    assert_eq!(rect(drawn(&spec).bbox), [8.0, 8.0, 52.0, 32.0]);
}

#[test]
fn a_filled_circle_fills_and_strokes() {
    let spec = AnnotSpec::Shape(
        Shape::new(ShapeKind::Circle, UserSpaceRect::from_corners(0.0, 0.0, 20.0, 10.0))
            .with_interior(Some(AnnotColor::Gray(0.5))),
    );
    let t = text(&spec);
    assert!(t.contains("0.5 g") && t.trim_end().ends_with('B'), "{t}");
}

#[test]
fn a_dashed_border_sets_its_pattern() {
    let spec = AnnotSpec::Shape(
        Shape::new(ShapeKind::Square, UserSpaceRect::from_corners(0.0, 0.0, 20.0, 10.0))
            .with_border(Border::dashed(1.0, vec![3.0, 2.0])),
    );
    assert!(text(&spec).contains("[3 2] 0 d"));
}

#[test]
fn a_cloudy_square_reaches_past_its_box_by_half_a_bump() {
    let spec = AnnotSpec::Shape(
        Shape::new(ShapeKind::Square, UserSpaceRect::from_corners(0.0, 0.0, 48.0, 48.0))
            .with_border(Border::cloudy(1.0, 1.0)),
    );
    // 192 points round, 16 bumps of 12: each reaches 6 out, plus half the stroke.
    let b = drawn(&spec).bbox;
    assert!((b.x0 - (-6.5)).abs() < 0.01 && (b.x1 - 54.5).abs() < 0.01, "{b:?}");
}

#[test]
fn every_ending_draws_and_grows_the_box_past_the_line() {
    for end in LineEnding::ALL.into_iter().filter(|e| *e != LineEnding::None) {
        let spec = AnnotSpec::Line(LineMark::new(p(0.0, 0.0), p(100.0, 0.0)).with_ends(end, end));
        let b = drawn(&spec).bbox;
        assert!(b.y1 > 0.5 && b.y0 < -0.5, "{end:?}: {b:?}");
    }
}

#[test]
fn a_closed_arrow_is_filled_with_the_interior_colour() {
    let line = LineMark::new(p(0.0, 0.0), p(100.0, 0.0))
        .with_ends(LineEnding::None, LineEnding::ClosedArrow)
        .with_interior(Some(AnnotColor::Rgb(0.0, 0.0, 1.0)));
    assert!(text(&AnnotSpec::Line(line)).contains("0 0 1 rg"));
}

#[test]
fn leader_lines_move_the_line_to_their_far_end() {
    let line = LineMark::new(p(0.0, 0.0), p(100.0, 0.0)).with_leader(Some(Leader::new(20.0)));
    // To the left of a line going right: up, 20 points.
    assert_eq!(rect(drawn(&AnnotSpec::Line(line)).bbox), [-0.5, -0.5, 100.5, 20.5]);
}

#[test]
fn a_polyline_draws_its_endings_along_its_end_segments() {
    let poly = Poly::new(PolyKind::PolyLine, vec![p(0.0, 0.0), p(50.0, 50.0), p(100.0, 0.0)])
        .with_ends(LineEnding::None, LineEnding::Square);
    let b = drawn(&AnnotSpec::Poly(poly)).bbox;
    assert!(b.x1 > 100.5, "{b:?}");
}

#[test]
fn a_polyline_ending_on_a_repeated_vertex_keeps_its_arrow() {
    let ends = |vertices| {
        let poly = Poly::new(PolyKind::PolyLine, vertices).with_ends(LineEnding::None, LineEnding::ClosedArrow);
        drawn(&AnnotSpec::Poly(poly)).bbox
    };
    let once = ends(vec![p(0.0, 0.0), p(50.0, 50.0), p(100.0, 0.0)]);
    let twice = ends(vec![p(0.0, 0.0), p(50.0, 50.0), p(100.0, 0.0), p(100.0, 0.0)]);
    assert_eq!(rect(twice), rect(once));
}

#[test]
fn a_pressure_stroke_is_filled_not_stroked() {
    let stroke = InkStroke::new(vec![p(0.0, 0.0), p(10.0, 5.0), p(20.0, 0.0)]).with_widths(vec![1.0, 4.0, 1.0]);
    let t = text(&AnnotSpec::Ink(Ink::new(vec![stroke])));
    let ops: Vec<&str> = t.lines().filter_map(|l| l.split_whitespace().last()).collect();
    assert!(ops.contains(&"f") && !ops.contains(&"S"), "{t}");
}

#[test]
fn a_pressure_stroke_reaches_half_its_widest_point_and_no_further() {
    let stroke = InkStroke::new(vec![p(0.0, 0.0), p(20.0, 0.0)]).with_widths(vec![2.0, 6.0]);
    let b = drawn(&AnnotSpec::Ink(Ink::new(vec![stroke]))).bbox;
    // The round cap at the wide end: 3 past the point, and nothing added for a pen.
    assert!((b.y1 - 3.0).abs() < 1e-9 && (b.x1 - 23.0).abs() < 1e-9, "{b:?}");
}

#[test]
fn ink_smoothing_draws_curves_and_without_it_lines() {
    let points = vec![p(0.0, 0.0), p(10.0, 10.0), p(20.0, 0.0), p(30.0, 10.0)];
    let smooth = text(&AnnotSpec::Ink(Ink::new(vec![InkStroke::new(points.clone())])));
    let straight = text(&AnnotSpec::Ink(
        Ink::new(vec![InkStroke::new(points)]).with_smoothing(false),
    ));
    assert!(smooth.contains(" c\n") && !straight.contains(" c\n"));
}

#[test]
fn a_one_point_stroke_is_a_dot() {
    let spec = AnnotSpec::Ink(Ink::new(vec![InkStroke::new(vec![p(5.0, 5.0)])]).with_width(4.0));
    assert_eq!(rect(drawn(&spec).bbox), [1.0, 1.0, 9.0, 9.0]);
}

#[test]
fn a_drawing_mark_is_red_without_a_colour() {
    let spec = AnnotSpec::Line(LineMark::new(p(0.0, 0.0), p(1.0, 1.0)));
    assert!(text(&spec).contains("1 0 0 RG"));
}

#[test]
fn the_same_mark_draws_the_same_bytes() {
    let spec = AnnotSpec::Shape(
        Shape::new(ShapeKind::Circle, UserSpaceRect::from_corners(0.0, 0.0, 80.0, 50.0))
            .with_border(Border::cloudy(1.5, 2.0)),
    );
    assert_eq!(
        text(&spec),
        String::from_utf8(draw_any(&spec, &PAINT).content()).unwrap()
    );
}
