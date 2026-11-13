---
title: "Shapes, colour and images"
description: "Draw lines, shapes and images on a page, in any colour, with transparency, clipping and transforms."
version: "0.2.0"
url: "https://docs.laminapdf.com/guides/images-and-vectors/"
---

# Shapes, colour and images

Draw lines, shapes and images on a page, in any colour, with transparency, clipping and transforms.

You'll draw lines, shapes and images on a page, in colour and with transparency. You need this for anything visual in a PDF: a logo, a chart, the rules of a table or a photo.

Shapes and images need only the base module (`init()`).

## Draw a shape

`surface.path(p => …)` returns the shape unpainted. Paint it with `fill`, `stroke` or `fillStroke`, or call `clip` to limit what comes after to its inside. A value that makes no shape, such as NaN or a negative line width, fails `page.finish()` with `invalid-geometry`.

## Colour it

A colour is gray `[g]`, RGB `[r, g, b]`, CMYK `[c, m, y, k]` or hex. Opacity and blend modes set how a drawing mixes with what's already on the page: in the example, `multiply` darkens where a shape overlaps what's below it.

## Place an image

- `Image.jpeg(bytes)` embeds a JPEG as it is, without decoding it.
- `Image.png(bytes)` keeps the PNG's transparency.
- `Image.rgba(width, height, pixels)` takes raw pixels.

## Move, turn and scale

A `Matrix` rotates, scales or moves what you draw. It works in [page space](../../pages/reference/glossary.md#page-space), like every other position.

This example draws rectangles, circles, curves and dashed strokes in several colours, with transparency, a clip and a rotation, and places a JPEG and a transparent image:

**TypeScript**

`packages/sdk/examples/images-and-vectors.ts`:

```ts
// Images and vector graphics on one A4 page. It draws shapes and a dashed Bézier curve in gray,
// RGB, CMYK and hex colour, overlaps two translucent discs with a blend mode, clips a JPEG to an
// ellipse and rotates an RGBA image with a soft mask. Needs only the base module.
import { Builder, type BuilderOptions, Image, Matrix, type Surface } from "@laminapdf/sdk";

/** The RGBA gradient's side, in pixels. */
const GLOW_SIDE = 16;

/**
 * The document's bytes, with `photo` (a JPEG) on the page.
 *
 * @param options - can fix the file ID, the producer and the creation date.
 */
export function imagesAndVectors(photo: Uint8Array, options: BuilderOptions = {}): Uint8Array<ArrayBuffer> {
  using b = new Builder(options);
  const jpeg = Image.jpeg(photo);
  const glow = Image.rgba(GLOW_SIDE, GLOW_SIDE, gradient());
  const page = b.page("A4");
  const s = page.surface();
  shapes(s);
  overlappingDiscs(s);
  images(s, jpeg, glow);
  page.finish();
  using doc = b.finish();
  return doc.save();
}

/** A square gradient whose alpha falls off towards the right. */
function gradient(): Uint8Array {
  const out = new Uint8Array(GLOW_SIDE * GLOW_SIDE * 4);
  for (let y = 0; y < GLOW_SIDE; y++) {
    for (let x = 0; x < GLOW_SIDE; x++) out.set([x * 16, y * 16, 200, 255 - x * 15], (y * GLOW_SIDE + x) * 4);
  }
  return out;
}

/** A rectangle, a circle and a dashed, round-capped Bézier curve. */
function shapes(s: Surface): void {
  s.fillColor([0.9, 0.3, 0.2])
    .path((p) => p.rect([56, 56, 160, 100]))
    .fill();
  s.fillColor([0.8, 0.1, 0, 0])
    .path((p) => p.circle([330, 106], 50))
    .fill();
  s.strokeColor("#2a9d8f")
    .path((p) => p.moveTo(56, 220).cubicTo(180, 140, 300, 300, 539, 200))
    .stroke({ width: 3, cap: "round", dash: [12, 8] });
}

/** Two translucent discs, the second multiplied onto the first. */
function overlappingDiscs(s: Surface): void {
  s.save().opacity(0.6, 1).fillColor([0.3]);
  s.path((p) => p.circle([120, 330], 60)).fill();
  s.blendMode("multiply").fillColor([1, 0.8, 0]);
  s.path((p) => p.circle([190, 330], 60))
    .fill()
    .restore();
}

/** The photo clipped to an ellipse, and the gradient rotated by 30 degrees. */
function images(s: Surface, photo: Image, glow: Image): void {
  s.save()
    .path((p) => p.ellipse([300, 270, 239, 120]))
    .clip();
  s.image(photo, [300, 250, 239, 160]).restore();
  s.save().transform(Matrix.after(Matrix.translate(150, 520), Matrix.rotate(30)));
  s.image(glow, [-60, -60, 120, 120]).restore();
}
```

**Rust**

`crates/laminapdf/examples/images_and_vectors.rs`:

````rust
//! Images and vector graphics on one A4 page.
//!
//! It draws shapes and a dashed Bézier curve in gray, RGB, CMYK and hex colour, overlaps two
//! translucent discs with a blend mode, clips a JPEG to an ellipse and rotates an RGBA image
//! with a soft mask.
//!
//! ```sh
//! cargo run -p laminapdf --example images_and_vectors -- photo.jpg [out-dir]   # writes images_and_vectors.pdf
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
        dash: Some(Dash::new(vec![12.0, 8.0], 0.0)),
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
    let placement = Matrix::translate(150.0, 520.0).after(&Matrix::rotate(30.0));
    s.save().transform(placement);
    s.image(glow, RectPt::from_points(-60.0, -60.0, 120.0, 120.0)).restore();
}
````

**Result:** [images_and_vectors.pdf](https://docs.laminapdf.com/examples/pdf/images_and_vectors.pdf)

[See the output](../../pages/recipes/images-and-vectors.md).

> **Details:** A PNG's alpha channel, and an RGBA image's, becomes a soft mask: a second image that holds the transparency.

Next: [links, outlines and metadata](../../pages/guides/links-outlines-and-metadata.md).
