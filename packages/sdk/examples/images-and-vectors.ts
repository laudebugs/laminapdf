// Images and vector graphics on one A4 page. Shapes, a dashed Bézier curve, gray, RGB, CMYK and
// hex colour, transparency with a blend mode, a JPEG clipped to an ellipse, and a rotated RGBA
// image with a soft mask. Needs only the base module.
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
