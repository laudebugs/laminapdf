// The pdf.js renderer of `cargo xtask annots-parity`: every page of a PDF to a binary PPM.
//
//   node tools/render/pdfjs-render.mjs <file.pdf> <out-dir> <dpi>
//
// Writes <out-dir>/<page>.ppm (0-based page index) and prints one JSON line
// `{ tool, version, pages }`. Settings match pdfium.py and hayro_render.rs: the page as shown
// (CropBox with /Rotate), floor(points × dpi / 72) pixels per axis, a white background.
// Annotations are drawn from their appearance streams (`AnnotationMode.ENABLE`: no form layer,
// no annotation editor), which is what the parity suite compares. The file is only read: no
// scripting, no system fonts; pdf.js's own copies of the standard 14 fonts stand in for the
// unembedded ones, as they do in a browser.
import { mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { createRequire } from "node:module";
import { dirname, join } from "node:path";
import { argv, exit } from "node:process";

import { AnnotationMode, getDocument, version } from "pdfjs-dist/legacy/build/pdf.mjs";

const require = createRequire(import.meta.url);
const STANDARD_FONTS = `${join(dirname(require.resolve("pdfjs-dist/package.json")), "standard_fonts")}/`;

/** RGBA pixels (straight alpha) over white, as packed RGB. */
function overWhite(rgba) {
  const rgb = new Uint8Array((rgba.length / 4) * 3);
  for (let i = 0, j = 0; i < rgba.length; i += 4, j += 3) {
    const a = rgba[i + 3] / 255;
    for (let c = 0; c < 3; c++) {
      rgb[j + c] = Math.round(rgba[i + c] * a + 255 * (1 - a));
    }
  }
  return rgb;
}

async function renderPage(doc, index, dpi) {
  const page = await doc.getPage(index + 1);
  const viewport = page.getViewport({ scale: dpi / 72 });
  const width = Math.floor(viewport.width);
  const height = Math.floor(viewport.height);
  const { canvas, context } = doc.canvasFactory.create(width, height);
  context.fillStyle = "#ffffff";
  context.fillRect(0, 0, width, height);
  await page.render({ canvas, canvasContext: context, viewport, annotationMode: AnnotationMode.ENABLE }).promise;
  const pixels = overWhite(context.getImageData(0, 0, width, height).data);
  page.cleanup();
  return { width, height, pixels };
}

async function main() {
  const [file, out, dpiText] = argv.slice(2);
  const dpi = Number(dpiText);
  if (!file || !out || !Number.isFinite(dpi) || dpi <= 0) {
    throw new Error("usage: pdfjs-render.mjs <file.pdf> <out-dir> <dpi>");
  }
  const task = getDocument({
    data: new Uint8Array(readFileSync(file)),
    isEvalSupported: false,
    useSystemFonts: false,
    standardFontDataUrl: STANDARD_FONTS,
    verbosity: 0,
  });
  const doc = await task.promise;
  mkdirSync(out, { recursive: true });
  for (let index = 0; index < doc.numPages; index++) {
    const { width, height, pixels } = await renderPage(doc, index, dpi);
    const header = new TextEncoder().encode(`P6\n${width} ${height}\n255\n`);
    const ppm = new Uint8Array(header.length + pixels.length);
    ppm.set(header);
    ppm.set(pixels, header.length);
    writeFileSync(join(out, `${index}.ppm`), ppm);
  }
  console.log(JSON.stringify({ tool: "pdfjs", version, pages: doc.numPages }));
  await task.destroy();
}

main().catch((error) => {
  console.error(error);
  exit(1);
});
