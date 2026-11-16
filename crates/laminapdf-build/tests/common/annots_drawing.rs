//! `fixtures/golden/annots/drawing.pdf`: every drawing mark `AnnotationAppearances` draws, on one
//! blank A4 page. From the top: a line with each line ending at both ends, one ending per row;
//! squares and circles, solid, dashed, cloudy and filled translucent; lines with leader lines
//! above and below, and a dashed arrow with a filled head; a filled polygon, a cloudy polygon and
//! a polyline with arrows; ink smoothed, ink in straight segments, a pressure stroke and a dot.

use laminapdf_build::{AnnotationAppearances, BuildOptions, Builder, PageSize};
use laminapdf_model::{UserSpacePoint, UserSpaceRect};
use laminapdf_ops::{
    AddAnnot, AnnotColor, AnnotSpec, Border, Ink, InkStroke, Leader, LineEnding, LineMark, Op, Opacity, Origin, Poly,
    PolyKind, Shape, ShapeKind,
};

use super::save;

/// The page's height: user-space y is this minus the page-space y.
const HEIGHT: f64 = 841.89;

/// A point in user space from page-space coordinates.
fn at(x: f64, y: f64) -> UserSpacePoint {
    UserSpacePoint::new(x, HEIGHT - y)
}

/// A box in user space from a page-space box's left, top, width and height.
fn area(left: f64, top: f64, width: f64, height: f64) -> UserSpaceRect {
    UserSpaceRect::from_corners(left, HEIGHT - top - height, left + width, HEIGHT - top)
}

fn blank_page() -> laminapdf_ops::Document {
    let mut b = Builder::new(
        BuildOptions::default()
            .with_id(*b"lamina-annots-4d")
            .with_producer("lamina-build golden")
            .with_creation_date("D:20261115120000Z".parse().unwrap())
            .with_compress(false),
    );
    b.page(PageSize::A4).finish().unwrap();
    b.finish().unwrap()
}

fn endings() -> Vec<AnnotSpec> {
    LineEnding::ALL
        .into_iter()
        .zip(0_u8..)
        .map(|(end, i)| {
            let y = 50.0 + 22.0 * f64::from(i);
            AnnotSpec::Line(
                LineMark::new(at(60.0, y), at(220.0, y))
                    .with_ends(end, end)
                    .with_interior(Some(AnnotColor::Rgb(1.0, 0.8, 0.8))),
            )
        })
        .collect()
}

fn shapes() -> Vec<AnnotSpec> {
    let borders = [
        Border::solid(2.0),
        Border::dashed(2.0, vec![6.0, 3.0]),
        Border::cloudy(1.0, 1.0),
        Border::cloudy(1.0, 2.0),
    ];
    let mut specs = Vec::new();
    for (border, i) in borders.into_iter().zip(0_u8..) {
        let x = 280.0 + 70.0 * f64::from(i);
        specs.push(AnnotSpec::Shape(
            Shape::new(ShapeKind::Square, area(x, 60.0, 50.0, 40.0)).with_border(border.clone()),
        ));
        specs.push(AnnotSpec::Shape(
            Shape::new(ShapeKind::Circle, area(x, 140.0, 50.0, 40.0)).with_border(border),
        ));
    }
    specs
}

fn lines() -> Vec<AnnotSpec> {
    vec![
        AnnotSpec::Line(
            LineMark::new(at(60.0, 330.0), at(260.0, 330.0))
                .with_leader(Some(Leader::new(30.0).with_extension(6.0).with_offset(4.0))),
        ),
        AnnotSpec::Line(LineMark::new(at(60.0, 380.0), at(260.0, 380.0)).with_leader(Some(Leader::new(-20.0)))),
        AnnotSpec::Line(
            LineMark::new(at(300.0, 330.0), at(500.0, 420.0))
                .with_ends(LineEnding::Circle, LineEnding::ClosedArrow)
                .with_interior(Some(AnnotColor::Rgb(0.1, 0.4, 0.9)))
                .with_border(Border::dashed(3.0, vec![9.0, 4.0])),
        ),
    ]
}

fn polys() -> Vec<AnnotSpec> {
    let star = |cx: f64, cy: f64| {
        (0_u8..10)
            .map(|i| {
                let r = if i % 2 == 0 { 45.0 } else { 20.0 };
                let a = std::f64::consts::FRAC_PI_2 + std::f64::consts::PI * f64::from(i) / 5.0;
                at(cx + r * a.cos(), cy - r * a.sin())
            })
            .collect::<Vec<_>>()
    };
    vec![
        AnnotSpec::Poly(
            Poly::new(PolyKind::Polygon, star(110.0, 510.0)).with_interior(Some(AnnotColor::Rgb(1.0, 0.9, 0.2))),
        ),
        AnnotSpec::Poly(
            Poly::new(
                PolyKind::Polygon,
                vec![at(200.0, 470.0), at(320.0, 480.0), at(290.0, 550.0), at(210.0, 540.0)],
            )
            .with_border(Border::cloudy(1.5, 1.0)),
        ),
        AnnotSpec::Poly(
            Poly::new(
                PolyKind::PolyLine,
                vec![at(370.0, 550.0), at(420.0, 470.0), at(470.0, 540.0), at(520.0, 480.0)],
            )
            .with_ends(LineEnding::OpenArrow, LineEnding::RClosedArrow)
            .with_border(Border::solid(2.0)),
        ),
    ]
}

fn wave(left: f64, top: f64) -> Vec<UserSpacePoint> {
    (0_u8..=24)
        .map(|i| {
            let t = f64::from(i);
            at(left + 6.0 * t, top + 18.0 * (t / 2.5).sin())
        })
        .collect()
}

fn ink() -> Vec<AnnotSpec> {
    let pressure: Vec<f32> = (0_u8..=24)
        .map(|i| 1.0 + 5.0 * (std::f32::consts::PI * f32::from(i) / 24.0).sin())
        .collect();
    vec![
        AnnotSpec::Ink(Ink::new(vec![InkStroke::new(wave(60.0, 640.0))]).with_width(2.0)),
        AnnotSpec::Ink(
            Ink::new(vec![InkStroke::new(wave(60.0, 700.0))])
                .with_width(2.0)
                .with_smoothing(false),
        ),
        AnnotSpec::Ink(Ink::new(vec![InkStroke::new(wave(280.0, 640.0)).with_widths(pressure)])),
        AnnotSpec::Ink(Ink::new(vec![InkStroke::new(vec![at(300.0, 720.0)])]).with_width(8.0)),
    ]
}

pub fn drawing() -> Vec<u8> {
    let mut doc = blank_page().with_appearances(AnnotationAppearances);
    let page = doc.pages().unwrap().get(0).unwrap().id().clone();
    let mut ops: Vec<Op> = [endings(), shapes(), lines(), polys(), ink()]
        .into_iter()
        .flatten()
        .map(|spec| Op::from(AddAnnot::new(page.clone(), spec)))
        .collect();
    // A translucent filled square over the circles' row.
    let mut filled = AddAnnot::new(
        page,
        AnnotSpec::Shape(
            Shape::new(ShapeKind::Square, area(300.0, 200.0, 220.0, 60.0))
                .with_interior(Some(AnnotColor::Rgb(0.2, 0.7, 0.3)))
                .with_border(Border::solid(1.0)),
        ),
    );
    filled.common.opacity = Opacity::new(0.5).unwrap();
    ops.push(filled.into());
    doc.transact(Origin::Local, |tx| {
        for op in ops {
            tx.apply(op)?;
        }
        Ok(())
    })
    .unwrap();
    save(doc)
}
