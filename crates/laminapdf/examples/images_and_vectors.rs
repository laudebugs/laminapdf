//! Images and vector graphics on one A4 page.
//!
//! Shapes, a dashed Bézier curve, gray, RGB, CMYK and hex colour, transparency with a blend
//! mode, a JPEG clipped to an ellipse, and a rotated RGBA image with a soft mask.
//!
//! ```sh
//! cargo run -p laminapdf --example images_and_vectors -- photo.jpg [out-dir]   # writes <out-dir>/images_and_vectors.pdf
//! ```

use std::error::Error;
use std::path::PathBuf;

use laminapdf::{
    BlendMode, BuildOptions, Builder, Color, Dash, FillRule, Image, LineCap, Matrix, PageSize, PathBuilder, PointPt,
    Pt, RectPt, SaveOptions, StrokeStyle, Surface,
};

/// The RGBA gradient's side, in pixels.
const GLOW_SIDE: u8 = 16;

fn main() -> Result<(), Box<dyn Error>> {
    let mut args = std::env::args().skip(1);
    let photo = std::fs::read(args.next().ok_or("usage: images_and_vectors <photo.jpg> [out-dir]")?)?;
    let out = PathBuf::from(args.next().unwrap_or_else(|| "target/examples-out".into()));
    std::fs::create_dir_all(&out)?;
    std::fs::write(out.join("images_and_vectors.pdf"), build(BuildOptions::new(), photo)?)?;
    Ok(())
}

/// The document's bytes, with `photo` (a JPEG) on the page. `options` can fix the file ID, the
/// producer and the creation date.
pub(crate) fn build(options: BuildOptions, photo: Vec<u8>) -> Result<Vec<u8>, Box<dyn Error>> {
    let mut b = Builder::new(options);
    let photo = Image::jpeg(photo)?;
    let glow = Image::rgba(u32::from(GLOW_SIDE), u32::from(GLOW_SIDE), &gradient())?;
    let mut page = b.page(PageSize::A4);
    let s = page.surface();
    shapes(s)?;
    overlapping_discs(s);
    images(s, &photo, &glow);
    page.finish()?;
    let mut doc = b.finish()?;
    Ok(doc.save(&SaveOptions::new())?.bytes)
}

/// A square gradient whose alpha falls off towards the right.
fn gradient() -> Vec<u8> {
    (0..GLOW_SIDE)
        .flat_map(|y| (0..GLOW_SIDE).flat_map(move |x| [x * 16, y * 16, 200, 255 - x * 15]))
        .collect()
}

/// A rectangle, a circle and a dashed, round-capped Bézier curve.
fn shapes(s: &mut Surface) -> Result<(), Box<dyn Error>> {
    let at = PointPt::from_points;
    s.fill_color(Color::rgb(0.9, 0.3, 0.2))
        .path(|p| _ = p.rect(RectPt::from_points(56.0, 56.0, 160.0, 100.0)))
        .fill(FillRule::NonZero);
    s.fill_color(Color::cmyk(0.8, 0.1, 0.0, 0.0))
        .path(|p| _ = p.circle(at(330.0, 106.0), Pt::new(50.0)))
        .fill(FillRule::NonZero);
    let dashed = StrokeStyle {
        width: Pt::new(3.0),
        cap: LineCap::Round,
        dash: Some(Dash {
            array: vec![Pt::new(12.0), Pt::new(8.0)],
            phase: Pt::ZERO,
        }),
        ..StrokeStyle::default()
    };
    s.stroke_color(Color::hex("#2a9d8f")?)
        .path(|p| {
            p.move_to(at(56.0, 220.0))
                .cubic_to(at(180.0, 140.0), at(300.0, 300.0), at(539.0, 200.0));
        })
        .stroke(&dashed);
    Ok(())
}

/// Two translucent discs, the second multiplied onto the first.
fn overlapping_discs(s: &mut Surface) {
    let disc = |x: f64| move |p: &mut PathBuilder| _ = p.circle(PointPt::from_points(x, 330.0), Pt::new(60.0));
    s.save().opacity(0.6, 1.0).fill_color(Color::gray(0.3));
    s.path(disc(120.0)).fill(FillRule::NonZero);
    s.blend_mode(BlendMode::Multiply).fill_color(Color::rgb(1.0, 0.8, 0.0));
    s.path(disc(190.0)).fill(FillRule::NonZero).restore();
}

/// The photo clipped to an ellipse, and the gradient rotated by 30 degrees.
fn images(s: &mut Surface, photo: &Image, glow: &Image) {
    s.save()
        .path(|p| _ = p.ellipse(RectPt::from_points(300.0, 270.0, 239.0, 120.0)))
        .clip(FillRule::NonZero);
    s.image(photo, RectPt::from_points(300.0, 250.0, 239.0, 160.0))
        .restore();
    let placement = Matrix::translate(Pt::new(150.0), Pt::new(520.0)).after(&Matrix::rotate(30.0));
    s.save().transform(placement);
    s.image(glow, RectPt::from_points(-60.0, -60.0, 120.0, 120.0)).restore();
}
