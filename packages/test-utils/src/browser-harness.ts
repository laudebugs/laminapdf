// What both adapters' browser runs share: fixtures fetched as Vite assets and PNGs decoded by
// the browser itself (for C2b's goldens).
import type { Decoded } from "./render-contract.js";

/** A reader over `urls` (an eager `import.meta.glob` of `?url` imports, keyed relative to `prefix`). */
export function fetcher(urls: Record<string, string>, prefix: string): (path: string) => Promise<Uint8Array> {
  return async (path) => {
    const url = urls[`${prefix}${path}`];
    if (url === undefined) throw new Error(`${path} is not among the fixtures this test serves`);
    const response = await fetch(url);
    if (!response.ok) throw new Error(`fetching ${path}: ${response.status}`);
    return new Uint8Array(await response.arrayBuffer());
  };
}

/** Decodes a PNG with the browser's decoder, unpremultiplied and without colour conversion. */
export async function decodePng(png: Uint8Array): Promise<Decoded> {
  const bitmap = await createImageBitmap(new Blob([png as Uint8Array<ArrayBuffer>], { type: "image/png" }), {
    colorSpaceConversion: "none",
    premultiplyAlpha: "none",
  });
  const canvas = new OffscreenCanvas(bitmap.width, bitmap.height);
  const context = canvas.getContext("2d");
  if (context === null) throw new Error("no 2D context");
  context.drawImage(bitmap, 0, 0);
  const data = context.getImageData(0, 0, bitmap.width, bitmap.height);
  bitmap.close();
  return { width: data.width, height: data.height, pixels: data.data };
}
