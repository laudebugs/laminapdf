//! Vector graphics and images: rectangles, circles, Bézier curves, dashed and round-capped
//! strokes, gray/RGB/CMYK/hex colour, transparency and a blend mode, a clip, a rotated
//! transform, a JPEG passed through and an RGBA image with a soft mask.
//!
//! ```sh
//! cargo run -p laminapdf --example vectors -- [out-dir]   # writes <out-dir>/vectors.pdf
//! ```

use std::error::Error;
use std::path::PathBuf;

use laminapdf::{
    BlendMode, BuildOptions, Builder, CancelToken, Color, Dash, FillRule, Image, LineCap, Matrix, PageSize, PointPt,
    Pt, RectPt, SaveOptions, StrokeStyle,
};

fn main() -> Result<(), Box<dyn Error>> {
    let out = PathBuf::from(std::env::args().nth(1).unwrap_or_else(|| "target/examples-out".into()));
    std::fs::create_dir_all(&out)?;
    std::fs::write(out.join("vectors.pdf"), build()?)?;
    Ok(())
}

fn rect(x: f64, y: f64, w: f64, h: f64) -> RectPt {
    RectPt::from_xywh(Pt::new(x), Pt::new(y), Pt::new(w), Pt::new(h))
}

/// A 16×16 RGBA gradient whose alpha falls off towards the right.
fn gradient() -> Vec<u8> {
    (0_u8..16)
        .flat_map(|y| (0_u8..16).flat_map(move |x| [x * 16, y * 16, 200, 255 - x * 15]))
        .collect()
}

/// The document's bytes.
pub(crate) fn build() -> Result<Vec<u8>, Box<dyn Error>> {
    let at = PointPt::from_points;
    let options = BuildOptions::default()
        .with_id(*b"lamina-ex-vector")
        .with_producer("lamina examples")
        .with_creation_date("D:20261030120000Z".parse()?);
    let mut b = Builder::new(options);
    let photo = Image::jpeg(include_bytes!("../../../fixtures/images/photo-rgb.jpg").to_vec())?;
    let glow = Image::rgba(16, 16, &gradient())?;
    let mut page = b.page(PageSize::A4);
    let s = page.surface();
    let dashed = StrokeStyle {
        width: Pt::new(3.0),
        cap: LineCap::Round,
        dash: Some(Dash {
            array: vec![Pt::new(12.0), Pt::new(8.0)],
            phase: Pt::ZERO,
        }),
        ..StrokeStyle::default()
    };
    s.fill_color(Color::rgb(0.9, 0.3, 0.2))
        .path(|p| {
            p.rect(rect(56.0, 56.0, 160.0, 100.0));
        })
        .fill(FillRule::NonZero);
    s.fill_color(Color::cmyk(0.8, 0.1, 0.0, 0.0))
        .path(|p| {
            p.circle(at(330.0, 106.0), Pt::new(50.0));
        })
        .fill(FillRule::NonZero);
    s.stroke_color(Color::hex("#2a9d8f")?)
        .path(|p| {
            p.move_to(at(56.0, 220.0))
                .cubic_to(at(180.0, 140.0), at(300.0, 300.0), at(539.0, 200.0));
        })
        .stroke(&dashed);
    // Translucent overlapping discs, the second multiplied onto the first.
    s.save().opacity(0.6, 1.0).fill_color(Color::gray(0.3));
    s.path(|p| {
        p.circle(at(120.0, 330.0), Pt::new(60.0));
    })
    .fill(FillRule::NonZero);
    s.blend_mode(BlendMode::Multiply).fill_color(Color::rgb(1.0, 0.8, 0.0));
    s.path(|p| {
        p.circle(at(190.0, 330.0), Pt::new(60.0));
    })
    .fill(FillRule::NonZero)
    .restore();
    // The photo clipped to an ellipse, and a rotated, alpha-masked gradient.
    s.save()
        .path(|p| {
            p.ellipse(rect(300.0, 270.0, 239.0, 120.0));
        })
        .clip(FillRule::NonZero);
    s.image(&photo, rect(300.0, 250.0, 239.0, 160.0)).restore();
    s.save()
        .transform(Matrix::translate(Pt::new(150.0), Pt::new(520.0)).after(&Matrix::rotate(30.0)));
    s.image(&glow, rect(-60.0, -60.0, 120.0, 120.0)).restore();
    page.finish()?;
    let mut doc = b.finish()?;
    Ok(doc.save(&SaveOptions::new(), &CancelToken::new())?.bytes)
}
