// The page image as Heron reads it: 640 × 640, planar RGB (all reds, then greens, then blues),
// one byte per channel. The model's graph scales and normalises itself, so the bytes go in as they
// are. A port of crates/laminapdf-layout-onnx/src/pre.rs: the same filter, so both runtimes see
// the same pixels.

/** The side of the square image Heron reads. */
export const SIDE = 640;

/** One output pixel's inputs along an axis: the first input index and each input's weight. */
interface Taps {
  readonly start: number;
  readonly weights: Float32Array;
}

/**
 * Pillow's bilinear filter along one axis: a triangle whose width grows with the reduction, so a
 * page shrunk to 640 pixels averages its strokes instead of skipping them.
 */
function taps(from: number, to: number): Taps[] {
  const scale = from / to;
  const support = Math.max(scale, 1);
  const out: Taps[] = [];
  for (let o = 0; o < to; o++) {
    const centre = (o + 0.5) * scale;
    const lo = Math.min(Math.max(Math.floor(centre - support), 0), from);
    const hi = Math.min(Math.max(Math.ceil(centre + support), 0), from);
    const raw: number[] = [];
    for (let i = lo; i < hi; i++) raw.push(Math.max(1 - Math.abs((i + 0.5 - centre) / support), 0));
    const sum = raw.reduce((a, b) => a + b, 0);
    out.push({ start: lo, weights: Float32Array.from(raw, (w) => (sum > 0 ? w / sum : 0)) });
  }
  return out;
}

/** The RGBA pixels over white, as three planes of floats. */
function overWhite(pixels: Uint8Array | Uint8ClampedArray, count: number): Float32Array[] {
  const planes = [new Float32Array(count), new Float32Array(count), new Float32Array(count)];
  for (let i = 0; i < count; i++) {
    const alpha = (pixels[i * 4 + 3] ?? 255) / 255;
    planes.forEach((plane, c) => {
      plane[i] = (pixels[i * 4 + c] ?? 255) * alpha + 255 * (1 - alpha);
    });
  }
  return planes;
}

/** One weighted sum of `source` at `start + k * stride`. */
function weigh(t: Taps, source: Float32Array, start: number, stride: number): number {
  let sum = 0;
  for (let k = 0; k < t.weights.length; k++)
    sum += (t.weights[k] ?? 0) * (source[start + (t.start + k) * stride] ?? 255);
  return sum;
}

/** One plane (`w × h`) resized to 640 × 640 into `out` at `offset`: across first, then down. */
function resizePlane(plane: Float32Array, w: number, h: number, out: Uint8Array, offset: number): void {
  const across = taps(w, SIDE);
  const down = taps(h, SIDE);
  const row = new Float32Array(SIDE * h);
  for (let y = 0; y < h; y++) {
    across.forEach((t, x) => {
      row[y * SIDE + x] = weigh(t, plane, y * w, 1);
    });
  }
  down.forEach((t, y) => {
    for (let x = 0; x < SIDE; x++) {
      out[offset + y * SIDE + x] = Math.min(Math.max(Math.round(weigh(t, row, x, SIDE)), 0), 255);
    }
  });
}

/**
 * Straight RGBA rows (`width × 4` bytes each) resized to 640 × 640 by Pillow's bilinear filter,
 * over white: a transparent pixel reads as paper, as on screen.
 */
export function chw640(pixels: Uint8Array | Uint8ClampedArray, width: number, height: number): Uint8Array {
  const w = Math.max(width, 1);
  const h = Math.max(height, 1);
  const out = new Uint8Array(3 * SIDE * SIDE);
  overWhite(pixels, w * h).forEach((plane, c) => {
    resizePlane(plane, w, h, out, c * SIDE * SIDE);
  });
  return out;
}
