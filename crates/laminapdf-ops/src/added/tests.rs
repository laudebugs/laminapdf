//! The content ops' wire forms and the checks that run before anything is written.

use laminapdf_model::{PointPt, Pt, RectPt};

use super::*;
use crate::Op;
use crate::annot::{AnnotFont, TextSpan};

fn rect(x0: f64, y0: f64, x1: f64, y1: f64) -> RectPt {
    RectPt::new(Pt::new(x0), Pt::new(y0), Pt::new(x1), Pt::new(y1))
}

#[test]
fn the_ops_read_and_write_their_wire_forms() {
    let blob = BlobId::of(b"png");
    for json in [
        format!(
            r#"{{"op":"addContentObject","args":{{"id":"logo","page":"p3.0","object":{{"image":{{"blob":"{blob}","crop":[0.0,0.0,0.5,1.0]}}}},"matrix":[144.0,0.0,0.0,44.0,72.0,72.0],"opacity":0.5,"layer":"behind","alt":{{"text":"Logo"}},"modified":"D:20261118120000Z"}}}}"#
        ),
        r#"{"op":"addContentObject","args":{"id":"box","page":"p3.0","object":{"shape":{"path":{"rect":{"rect":[72.0,72.0,144.0,144.0],"radius":6.0}},"fill":[1.0,0.0,0.0],"stroke":{"color":[0.0],"width":2.0,"dash":"dotted"}}},"matrix":[1.0,0.0,0.0,1.0,0.0,0.0],"alt":"decorative","modified":"D:20261118120000Z"}}"#.to_owned(),
        r#"{"op":"updateContentObject","args":{"id":"box","page":"p3.0","modified":"D:20261118120000Z","fill":null,"order":-1}}"#.to_owned(),
        r#"{"op":"updateContentObject","args":{"id":"logo","page":"p3.0","modified":"D:20261118120000Z","crop":null}}"#.to_owned(),
        r#"{"op":"deleteContentObject","args":{"id":"logo","page":"p3.0"}}"#.to_owned(),
    ] {
        let op: Op = serde_json::from_str(&json).unwrap();
        assert_eq!(serde_json::to_string(&op).unwrap(), json);
    }
}

#[test]
fn unknown_fields_and_bad_ids_are_refused() {
    for json in [
        r#"{"op":"deleteContentObject","args":{"id":"logo","page":"p3.0","extra":1}}"#,
        r#"{"op":"deleteContentObject","args":{"id":"no spaces","page":"p3.0"}}"#,
        r#"{"op":"deleteContentObject","args":{"id":"","page":"p3.0"}}"#,
        r#"{"op":"addContentObject","args":{"id":"a","page":"p3.0","object":{"shape":{"path":{"line":{"from":{"x":0,"y":0},"to":{"x":1,"y":1},"extra":1}},"stroke":{"color":[0],"width":1}}},"matrix":[1,0,0,1,0,0],"modified":"D:20261118120000Z"}}"#,
        r#"{"op":"addContentObject","args":{"id":"a","page":"p3.0","object":{"circle":{}},"matrix":[1,0,0,1,0,0],"modified":"D:20261118120000Z"}}"#,
    ] {
        assert!(serde_json::from_str::<Op>(json).is_err(), "{json}");
    }
    assert!(ContentId::new("a".repeat(64)).is_some());
    assert!(ContentId::new("a".repeat(65)).is_none());
}

#[test]
fn a_shape_must_draw_something_within_range() {
    let stroke = Some(Stroke::new(AnnotColor::Gray(0.0), 1.0));
    let red = Some(AnnotColor::Rgb(1.0, 0.0, 0.0));
    let shape = |path, fill, stroke| ContentObjectSpec::Shape { path, fill, stroke };
    let line = PathSpec::Line {
        from: PointPt::from_points(0.0, 0.0),
        to: PointPt::from_points(10.0, 0.0),
    };
    let square = PathSpec::Rect {
        rect: rect(0.0, 0.0, 10.0, 10.0),
        radius: 0.0,
    };
    let triangle = |closed| PathSpec::Polygon {
        points: vec![
            PointPt::from_points(0.0, 0.0),
            PointPt::from_points(10.0, 0.0),
            PointPt::from_points(5.0, 8.0),
        ],
        closed,
    };
    let valid = [
        shape(line.clone(), None, stroke.clone()),
        shape(square.clone(), red, None),
        shape(triangle(true), red, stroke.clone()),
        shape(triangle(false), None, stroke),
    ];
    let invalid = [
        // Nothing to draw.
        shape(square.clone(), None, None),
        // A line or an open polygon has no inside to fill.
        shape(line, red, None),
        shape(triangle(false), red, None),
        // A corner rounder than half the side, an empty rectangle, a stroke that isn't one.
        shape(
            PathSpec::Rect {
                rect: rect(0.0, 0.0, 10.0, 10.0),
                radius: 6.0,
            },
            red,
            None,
        ),
        shape(
            PathSpec::Ellipse {
                rect: rect(0.0, 0.0, 0.0, 10.0),
            },
            red,
            None,
        ),
        shape(square, None, Some(Stroke::new(AnnotColor::Gray(0.0), 0.0))),
    ];
    assert!(valid.iter().all(ContentObjectSpec::is_valid));
    for spec in &invalid {
        assert!(!spec.is_valid(), "{spec:?}");
    }
    let crop = |c| ContentObjectSpec::Image {
        blob: BlobId::of(b""),
        crop: Some(c),
    };
    assert!(crop(rect(0.0, 0.0, 0.5, 1.0)).is_valid());
    assert!(!crop(rect(0.0, 0.0, 1.5, 1.0)).is_valid());
    assert!(!crop(rect(0.5, 0.0, 0.5, 1.0)).is_valid());
}

#[test]
fn a_matrix_places_only_when_finite_and_invertible() {
    assert!(placeable(&Matrix::new([2.0, 0.0, 0.0, 3.0, 10.0, 10.0])));
    assert!(!placeable(&Matrix::new([0.0, 0.0, 0.0, 3.0, 10.0, 10.0])));
    assert!(!placeable(&Matrix::new([f64::NAN, 0.0, 0.0, 1.0, 0.0, 0.0])));
}

#[test]
fn bounds_hold_the_box_once_placed() {
    let turned = Matrix::new([0.0, 50.0, -100.0, 0.0, 200.0, 100.0]);
    assert_eq!(
        bounds(&turned, ContentObjectSpec::UNIT),
        rect(100.0, 100.0, 200.0, 150.0)
    );
}

#[test]
fn added_text_and_its_patch_read_and_write_their_wire_forms() {
    for json in [
        r#"{"op":"addText","args":{"id":"note","page":"p3.0","rect":[72.0,72.0,288.0,120.0],"runs":[{"text":"Hi"}],"modified":"D:20261118120000Z"}}"#,
        r#"{"op":"updateContentObject","args":{"id":"note","page":"p3.0","modified":"D:20261118120000Z","text":{"width":216.0,"height":48.0,"runs":[{"text":"Hi","italic":true}],"align":"right","fonts":["times"],"size":10.0}}}"#,
    ] {
        let op: Op = serde_json::from_str(json).unwrap();
        assert_eq!(serde_json::to_string(&op).unwrap(), json);
    }
}

#[test]
fn a_text_block_needs_area_text_a_sensible_size_and_blobs_for_its_fallbacks() {
    let block = |w, h, text: &str| TextBlock::new(w, h, vec![TextSpan::plain(text)]);
    assert!(ContentObjectSpec::Text(block(100.0, 20.0, "a")).is_valid());
    for bad in [
        block(0.0, 20.0, "a"),
        block(100.0, f64::NAN, "a"),
        block(100.0, 20.0, " \n "),
        block(100.0, 20.0, "a").with_fonts(Vec::new(), 0.0),
        block(100.0, 20.0, "a").with_fonts(Vec::new(), 501.0),
        block(100.0, 20.0, "a").with_fonts(vec![AnnotFont::default(), AnnotFont::default()], 12.0),
    ] {
        assert!(!ContentObjectSpec::Text(bad.clone()).is_valid(), "{bad:?}");
    }
    let font = AnnotFont::Font(BlobId::of(b"font"));
    assert!(ContentObjectSpec::Text(block(1.0, 1.0, "a").with_fonts(vec![font, font], 12.0)).is_valid());
}

#[test]
fn a_line_feed_ends_a_paragraph_and_keeps_the_style_of_its_run() {
    let block = TextBlock::new(
        100.0,
        20.0,
        vec![
            TextSpan::plain("one\ntwo"),
            TextSpan::plain("\r\nthree").with_bold(true),
        ],
    );
    let paragraphs = block.paragraphs();
    let texts: Vec<Vec<(String, bool)>> = paragraphs
        .iter()
        .map(|p| p.iter().map(|s| (s.text.clone(), s.bold)).collect())
        .collect();
    assert_eq!(
        texts,
        [
            vec![("one".to_owned(), false)],
            vec![("two".to_owned(), false)],
            vec![("three".to_owned(), true)]
        ]
    );
}
