use laminapdf_cos::{Dict, Object};
use laminapdf_model::{UserSpacePoint, UserSpaceRect};

use super::super::AnnotSpec;
use super::super::drawing_patch::{InkPatch, LinePatch};
use super::*;

fn p(x: f64, y: f64) -> UserSpacePoint {
    UserSpacePoint::new(x, y)
}

/// `spec`'s keys written over a dictionary with `/Rect`, then read back.
fn round_trip(spec: &AnnotSpec, subtype: &str, rect: UserSpaceRect) -> Option<AnnotSpec> {
    let mut dict = Dict::new();
    dict.insert("Subtype", Object::name(subtype));
    dict.insert("Rect", super::super::add::rect_object(&rect));
    super::super::add::write_spec(&mut dict, spec, None);
    read(subtype.as_bytes(), &dict, Some(rect))
}

#[test]
fn ink_reads_from_the_wire_with_its_defaults() {
    let spec: AnnotSpec =
        serde_json::from_str(r#"{ "type": "ink", "strokes": [ { "points": [ { "x": 1, "y": 2 } ] } ] }"#).unwrap();
    assert_eq!(spec, AnnotSpec::Ink(Ink::new(vec![InkStroke::new(vec![p(1.0, 2.0)])])));
}

#[test]
fn a_border_style_reads_from_the_wire() {
    let border: Border =
        serde_json::from_str(r#"{ "width": 2, "style": { "type": "dashed", "pattern": [3, 2] } }"#).unwrap();
    assert_eq!(border, Border::dashed(2.0, vec![3.0, 2.0]));
}

#[test]
fn a_line_refuses_unknown_fields() {
    let json = r#"{ "type": "line", "from": { "x": 0, "y": 0 }, "to": { "x": 1, "y": 1 }, "arrow": true }"#;
    assert!(serde_json::from_str::<AnnotSpec>(json).is_err());
}

#[test]
fn thinning_drops_points_within_a_quarter_point_and_keeps_the_last() {
    let stroke = InkStroke::new(vec![p(0.0, 0.0), p(0.1, 0.0), p(1.0, 0.0), p(1.1, 0.0)])
        .with_widths(vec![1.0, 2.0, 3.0, 4.0])
        .thinned();
    assert_eq!(
        (stroke.points, stroke.widths),
        (vec![p(0.0, 0.0), p(1.1, 0.0)], Some(vec![1.0, 4.0]))
    );
}

#[test]
fn thinning_keeps_the_first_point_and_changes_a_thinned_stroke_no_more() {
    // The last point lands near the first once the middle one goes.
    let once = InkStroke::new(vec![p(0.0, 0.0), p(0.3, 0.0), p(0.1, 0.0)]).thinned();
    assert_eq!(once.points, vec![p(0.0, 0.0), p(0.1, 0.0)]);
    assert_eq!(once.thinned(), once);
    // A last point near several kept ones replaces them all, so no two stay too close.
    let tail = InkStroke::new(vec![p(0.0, 0.0), p(5.0, 0.0), p(5.2, 0.0), p(5.1, 0.0)]).thinned();
    assert_eq!(tail.points, vec![p(0.0, 0.0), p(5.1, 0.0)]);
    assert_eq!(tail.thinned(), tail);
}

#[test]
fn widths_must_match_the_points() {
    let stroke = InkStroke::new(vec![p(0.0, 0.0), p(5.0, 5.0)]).with_widths(vec![1.0]);
    assert!(!AnnotSpec::Ink(Ink::new(vec![stroke])).is_valid());
}

#[test]
fn a_cloudy_line_is_invalid() {
    let line = LineMark {
        border: Border::cloudy(1.0, 1.0),
        ..LineMark::new(p(0.0, 0.0), p(10.0, 0.0))
    };
    assert!(!AnnotSpec::Line(line).is_valid());
}

#[test]
fn a_polygon_needs_three_vertices() {
    let poly = Poly::new(PolyKind::Polygon, vec![p(0.0, 0.0), p(5.0, 5.0)]);
    assert!(!AnnotSpec::Poly(poly).is_valid());
}

#[test]
fn a_polygon_has_no_endings() {
    let mut poly = Poly::new(PolyKind::Polygon, vec![p(0.0, 0.0), p(5.0, 5.0), p(9.0, 0.0)]);
    poly.ends = [LineEnding::None, LineEnding::OpenArrow];
    assert!(!AnnotSpec::Poly(poly).is_valid());
}

#[test]
fn a_shape_with_no_area_is_invalid() {
    let shape = Shape::new(ShapeKind::Square, UserSpaceRect::from_corners(0.0, 0.0, 10.0, 0.0));
    assert!(!AnnotSpec::Shape(shape).is_valid());
}

#[test]
fn ink_keys_read_back_with_widths_and_smoothing() {
    let ink = Ink::new(vec![
        InkStroke::new(vec![p(1.0, 1.0), p(9.0, 9.0)]).with_widths(vec![1.5, 3.0]),
        InkStroke::new(vec![p(4.0, 0.0)]),
    ])
    .with_width(2.0)
    .with_smoothing(false);
    let spec = AnnotSpec::Ink(ink);
    assert_eq!(
        round_trip(&spec, "Ink", UserSpaceRect::from_corners(0.0, 0.0, 10.0, 10.0)),
        Some(spec)
    );
}

#[test]
fn a_cloudy_filled_square_reads_back_inside_its_rect() {
    let shape = Shape {
        interior: Some(AnnotColor::Rgb(1.0, 1.0, 0.0)),
        border: Border::cloudy(2.0, 1.0),
        ..Shape::new(ShapeKind::Square, UserSpaceRect::from_corners(10.0, 10.0, 50.0, 30.0))
    };
    let spec = AnnotSpec::Shape(shape);
    let rect = UserSpaceRect::from_corners(4.0, 4.0, 56.0, 36.0);
    assert_eq!(round_trip(&spec, "Square", rect), Some(spec));
}

#[test]
fn a_dashed_arrow_with_leader_lines_reads_back() {
    let line = LineMark {
        interior: Some(AnnotColor::Gray(0.5)),
        border: Border::dashed(1.5, vec![4.0, 2.0]),
        leader: Some(Leader {
            length: 12.0,
            extension: 3.0,
            offset: 1.0,
        }),
        ..LineMark::new(p(0.0, 0.0), p(100.0, 50.0)).with_ends(LineEnding::Butt, LineEnding::ClosedArrow)
    };
    let spec = AnnotSpec::Line(line);
    assert_eq!(
        round_trip(&spec, "Line", UserSpaceRect::from_corners(-5.0, -5.0, 105.0, 55.0)),
        Some(spec)
    );
}

#[test]
fn a_polyline_with_endings_reads_back() {
    let mut poly = Poly::new(PolyKind::PolyLine, vec![p(0.0, 0.0), p(5.0, 5.0), p(9.0, 0.0)]);
    poly.ends = [LineEnding::Slash, LineEnding::RClosedArrow];
    let spec = AnnotSpec::Poly(poly);
    assert_eq!(
        round_trip(&spec, "PolyLine", UserSpaceRect::from_corners(0.0, 0.0, 9.0, 5.0)),
        Some(spec)
    );
}

#[test]
fn every_ending_name_reads_back_as_itself() {
    let names: Vec<_> = LineEnding::ALL
        .iter()
        .map(|e| LineEnding::from_name(e.name().as_bytes()))
        .collect();
    assert_eq!(names, LineEnding::ALL);
}

#[test]
fn another_apps_old_border_array_gives_the_width() {
    let mut dict = Dict::new();
    dict.insert("Border", Object::Array(vec![0.into(), 0.into(), 3.into()]));
    assert_eq!(read_border(&dict, false), Border::solid(3.0));
}

#[test]
fn erasing_a_stroke_and_adding_one_keeps_the_rest_in_order() {
    let ink = Ink::new(vec![
        InkStroke::new(vec![p(0.0, 0.0)]),
        InkStroke::new(vec![p(1.0, 1.0)]),
        InkStroke::new(vec![p(2.0, 2.0)]),
    ]);
    let patch = InkPatch {
        remove: vec![1],
        add: vec![InkStroke::new(vec![p(3.0, 3.0)])],
        ..InkPatch::default()
    };
    let firsts: Vec<_> = patch
        .applied_to(&ink)
        .map(|i| i.strokes.iter().map(|s| s.points[0]).collect())
        .unwrap_or_default();
    assert_eq!(firsts, [p(0.0, 0.0), p(2.0, 2.0), p(3.0, 3.0)]);
}

#[test]
fn erasing_a_stroke_the_mark_lacks_is_refused() {
    let ink = Ink::new(vec![InkStroke::new(vec![p(0.0, 0.0)])]);
    let patch = InkPatch {
        remove: vec![4],
        ..InkPatch::default()
    };
    assert_eq!(patch.applied_to(&ink), None);
}

#[test]
fn a_line_patch_can_remove_the_leader_lines() {
    let line = LineMark {
        leader: Some(Leader::new(10.0)),
        ..LineMark::new(p(0.0, 0.0), p(10.0, 0.0))
    };
    let patch: LinePatch = serde_json::from_str(r#"{ "leader": null }"#).unwrap();
    assert_eq!(patch.applied_to(&line).leader, None);
}
