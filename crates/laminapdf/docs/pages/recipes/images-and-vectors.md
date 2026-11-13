---
title: "Images and vectors"
description: "Shapes, a dashed curve, gray, RGB, CMYK and hex colour, transparency, a blend mode, a clip, a rotation, a JPEG and an RGBA image on one page."
version: "0.2.0"
url: "https://docs.laminapdf.com/recipes/images-and-vectors/"
---

# Images and vectors

Shapes, a dashed curve, gray, RGB, CMYK and hex colour, transparency, a blend mode, a clip, a rotation, a JPEG and an RGBA image on one page.

You'll draw shapes and images on one A4 page: a rectangle, a circle, a dashed curve, two see-through discs, a photo cut to an ellipse and a rotated gradient that fades out. Start from it for a logo, a chart, the rules of a table or a page with photos.

The function takes a JPEG's bytes, draws the page and returns the PDF's bytes. It draws no text, so in TypeScript it needs only the base module, `init()`. Both languages write the same bytes:

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

The result is one A4 page with every shape and both images. The Result tab shows it, with a link to the PDF.

## How it works

- `path(p => …)` builds a shape and returns it unpainted. `fill` and `stroke` paint it, and `clip` limits what's drawn next to its inside. See [Shapes, colour and images](../../pages/guides/images-and-vectors.md).
- A colour of one, three or four numbers from 0 to 1 is gray, RGB or CMYK, and a string such as `"#2a9d8f"` is hex. Rust has `Color::gray`, `rgb`, `cmyk` and `hex`.
- `save()` and `restore()` bracket each group of drawings, so the opacity, blend mode, clip and rotation set inside stop at `restore()`.
- Both discs are 60% opaque, and `multiply` darkens the area where the second disc covers the first.
- `Image.jpeg` embeds the photo as it is, without decoding it. `Image.rgba` takes raw pixels, and their alpha becomes a soft mask: a second image that holds the transparency.
- The matrix turns the gradient 30 degrees clockwise, then moves it to (150, 520). The gradient is drawn centred on the origin, so it turns about its own centre. Like every position, the matrix works in [page space](../../pages/reference/glossary.md#page-space).

## What to change

- **A PNG instead of the JPEG:** `Image.png(bytes)` (`Image::png` in Rust) keeps the PNG's transparency.
- **A shape with a hole:** draw both outlines in one path and fill it with `"evenOdd"` (`FillRule::EvenOdd`), so the inner shape stays empty.
- **A filled shape with an outline:** `fillStroke(rule, style)` (`fill_stroke` in Rust) paints both at once.
- **Another blend mode:** `"screen"` lightens where `"multiply"` darkens. `BlendMode` lists all sixteen.
- **Alternate text:** pass a description as the third argument of `image` (`figure` in Rust), and a screen reader reads it out. See [Accessible PDFs](../../pages/guides/accessible-pdfs.md).

Next: [links and outlines](../../pages/recipes/links-and-outlines.md), with four pages joined by links, an outline and page labels.
