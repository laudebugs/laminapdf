// Heron in onnxruntime-web, through @laminapdf/layout-heron, on two example pages rendered by
// hayro, against the regions crates/laminapdf-layout-onnx/tests/reference/heron.json recorded
// (ONNX Runtime's Python package and PDFium): every region the reference scored >= 0.5 must be
// found with the same label family and boxes overlapping by >= 0.8, the Rust test's rule.
//
// Run by hand, after `cargo xtask wasm`, `pnpm --filter @laminapdf/render --filter
// @laminapdf/layout-heron build` and `cargo xtask models fetch docling-layout-heron`:
//
//   pnpm --dir tools/heron-web install --frozen-lockfile
//   node tools/heron-web/check.mjs target/models/docling-layout-heron/<sha256>/model.onnx
import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import * as ort from "onnxruntime-web";

const root = join(dirname(fileURLToPath(import.meta.url)), "../..");
const { HeronLayout } = await import(join(root, "packages/layout-heron/dist/index.js"));
const { initHayro } = await import(join(root, "packages/render-hayro/dist/index.js"));
const { bytesSource } = await import(join(root, "packages/render/dist/index.js"));

const MIN_IOU = 0.8;
const STABLE = 0.5;
const family = (label) => (label === "title" || label === "section-header" ? "heading" : label);
const iou = ([ax, ay, aw, ah], [x0, y0, x1, y1]) => {
  const w = Math.max(Math.min(ax + aw, x1) - Math.max(ax, x0), 0);
  const h = Math.max(Math.min(ay + ah, y1) - Math.max(ay, y0), 0);
  const union = aw * ah + (x1 - x0) * (y1 - y0) - w * h;
  return union > 0 ? (w * h) / union : 0;
};

const weightsPath = process.argv[2];
if (weightsPath === undefined) throw new Error("usage: node tools/heron-web/check.mjs <model.onnx>");
// One thread, no worker: the same WebAssembly build a browser runs, without a server.
ort.env.wasm.numThreads = 1;
const heron = await HeronLayout.load(new Uint8Array(readFileSync(weightsPath)), ort);
const hayro = await initHayro(readFileSync(join(root, "packages/wasm/dist/lamina_pdf_render_hayro.wasm")));
const reference = JSON.parse(
  readFileSync(join(root, "crates/laminapdf-layout-onnx/tests/reference/heron.json"), "utf8"),
);

let failures = 0;
for (const [name, expected] of Object.entries(reference.pages)) {
  // Node 22 has no `using` yet: close the document by hand.
  const doc = hayro.open(bytesSource(new Uint8Array(readFileSync(join(root, "fixtures/golden/examples", name)))));
  let page;
  let tile;
  try {
    page = doc.page(0);
    tile = doc.render({ page: 0, scale: reference.dpi / 72 });
  } finally {
    doc.close();
  }
  const found = await heron.detect({
    pixels: tile.pixels,
    width: tile.width,
    height: tile.height,
    widthPt: page.width,
    heightPt: page.height,
  });
  for (const region of expected.filter((r) => r.score >= STABLE)) {
    const best = Math.max(
      0,
      ...found.filter((r) => family(r.label) === family(region.label)).map((r) => iou(r.bbox, region.bbox)),
    );
    const ok = best >= MIN_IOU;
    if (!ok) failures++;
    console.log(
      `${ok ? "ok  " : "FAIL"} ${name}: ${region.label} (reference ${region.score}) best IoU ${best.toFixed(3)}`,
    );
  }
}
if (failures > 0) {
  console.error(`${failures} region(s) not found`);
  process.exit(1);
}
console.log("heron-web: every stable region of the reference found");
