// Vector graphics and images: rectangles, circles, Bézier curves, dashed round-capped strokes,
// gray/RGB/CMYK/hex colour, transparency and a blend mode, a clip, a rotated transform, a JPEG
// passed through and an RGBA image with a soft mask. Needs only the base module. The TS twin of
// crates/lamina/examples/vectors.rs (fixtures/golden/examples/vectors.pdf).
import { Builder, Image, Matrix } from "@laminapdf/sdk";

/** A 16×16 RGBA gradient whose alpha falls off towards the right. */
function gradient(): Uint8Array {
  const out = new Uint8Array(16 * 16 * 4);
  for (let y = 0; y < 16; y++) {
    for (let x = 0; x < 16; x++) {
      out.set([x * 16, y * 16, 200, 255 - x * 15], (y * 16 + x) * 4);
    }
  }
  return out;
}

/**
 * The document's bytes.
 *
 * @param photo - `fixtures/images/photo-rgb.jpg`.
 */
export function imagesAndVectors(photo: Uint8Array): Uint8Array {
  const b = new Builder({
    id: new TextEncoder().encode("lamina-ex-vector"),
    producer: "lamina examples",
    creationDate: "D:20261030120000Z",
  });
  const jpeg = Image.jpeg(photo);
  const glow = Image.rgba(16, 16, gradient());
  const page = b.page("A4");
  const s = page.surface();
  const dashed = { width: 3, cap: "round", dash: [12, 8] } as const;
  s.fillColor([0.9, 0.3, 0.2])
    .path((p) => p.rect([56, 56, 160, 100]))
    .fill();
  s.fillColor([0.8, 0.1, 0, 0])
    .path((p) => p.circle([330, 106], 50))
    .fill();
  s.strokeColor("#2a9d8f")
    .path((p) => p.moveTo(56, 220).cubicTo(180, 140, 300, 300, 539, 200))
    .stroke(dashed);
  // Translucent overlapping discs, the second multiplied onto the first.
  s.save().opacity(0.6, 1).fillColor([0.3]);
  s.path((p) => p.circle([120, 330], 60)).fill();
  s.blendMode("multiply").fillColor([1, 0.8, 0]);
  s.path((p) => p.circle([190, 330], 60))
    .fill()
    .restore();
  // The photo clipped to an ellipse, and a rotated, alpha-masked gradient.
  s.save()
    .path((p) => p.ellipse([300, 270, 239, 120]))
    .clip();
  s.image(jpeg, [300, 250, 239, 160]).restore();
  s.save().transform(Matrix.after(Matrix.translate(150, 520), Matrix.rotate(30)));
  s.image(glow, [-60, -60, 120, 120]).restore();
  page.finish();
  return b.finish().save();
}
