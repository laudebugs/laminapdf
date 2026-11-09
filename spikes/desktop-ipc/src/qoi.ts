// A QOI decoder (qoiformat.org spec), since no webview decodes QOI natively. Runs on the
// main thread, so its cost shows up in the long-task and frame numbers.
export function decodeQoi(buffer: ArrayBuffer): ImageData {
  const bytes = new Uint8Array(buffer);
  const view = new DataView(buffer);
  if (view.getUint32(0) !== 0x716f6966) throw new Error("not a QOI image");
  const width = view.getUint32(4);
  const height = view.getUint32(8);
  const out = new Uint8ClampedArray(width * height * 4);
  const index = new Uint8Array(256);
  let [r, g, b, a] = [0, 0, 0, 255];
  let p = 14;
  let run = 0;
  for (let o = 0; o < out.length; o += 4) {
    if (run > 0) {
      run--;
    } else {
      const b1 = bytes[p++];
      if (b1 === 0xfe) {
        r = bytes[p++]; g = bytes[p++]; b = bytes[p++];
      } else if (b1 === 0xff) {
        r = bytes[p++]; g = bytes[p++]; b = bytes[p++]; a = bytes[p++];
      } else {
        const tag = b1 >> 6;
        if (tag === 0) {
          const i = b1 * 4;
          r = index[i]; g = index[i + 1]; b = index[i + 2]; a = index[i + 3];
        } else if (tag === 1) {
          r = (r + ((b1 >> 4) & 3) - 2) & 255;
          g = (g + ((b1 >> 2) & 3) - 2) & 255;
          b = (b + (b1 & 3) - 2) & 255;
        } else if (tag === 2) {
          const b2 = bytes[p++];
          const vg = (b1 & 0x3f) - 32;
          r = (r + vg - 8 + ((b2 >> 4) & 15)) & 255;
          g = (g + vg) & 255;
          b = (b + vg - 8 + (b2 & 15)) & 255;
        } else {
          run = b1 & 0x3f;
        }
      }
      const h = ((r * 3 + g * 5 + b * 7 + a * 11) % 64) * 4;
      index[h] = r; index[h + 1] = g; index[h + 2] = b; index[h + 3] = a;
    }
    out[o] = r; out[o + 1] = g; out[o + 2] = b; out[o + 3] = a;
  }
  return new ImageData(out, width, height);
}
