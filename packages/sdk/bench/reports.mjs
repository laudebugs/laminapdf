// Times the 100-page reports of pdf-library §4 (≤ 500 ms after init()) in Node over the built
// dist/ (todo B5d): a timing script, not a test (ADR 0018 keeps wall-clock asserts out of tests).
// - `report`: examples/report.ts, the report `cargo xtask pdf-runtimes` times in every runtime;
// - `text-boxes`: B6b's first measurement, a Helvetica-Bold heading and six 4-sentence Noto Sans
//   text boxes per page.
// Both have native twins in crates/lamina/benches/report.rs (`cargo bench -p lamina --bench
// report`), so a gap between the two is binding cost.
//
//   cargo xtask wasm && node packages/sdk/bench/reports.mjs [runs]   # Node ≥ 22.18 (type stripping)
//
// Prints the minimum and median of `runs` builds (default 10) after one warm-up build.
import { readFileSync } from "node:fs";
import { argv } from "node:process";
import { Builder, Font, init } from "@laminapdf/sdk";
import { REPORT_PAGES, report } from "../examples/report.ts";

const root = new URL("../../../", import.meta.url);
const read = (path) => new Uint8Array(readFileSync(new URL(path, root)));
const notoSans = read("fixtures/fonts/NotoSans-Regular.ttf");
const notoSansSC = read("fixtures/fonts/NotoSansSC-Regular.otf");

const SENTENCES =
  "The quarter closed with steady growth across every region we track. Shipments rose in " +
  "the north while the south held level against a strong previous year. Margins improved " +
  "as freight costs eased and the new warehouse came fully online. The outlook for the " +
  "next quarter remains cautious but positive.";

/** B6b's report: a Helvetica-Bold heading and six text boxes per page. */
function textBoxes() {
  const b = new Builder({
    id: new TextEncoder().encode("lamina-bench-b6b"),
    producer: "lamina examples",
    creationDate: "D:20261030120000Z",
  });
  try {
    const bold = Font.standard("Helvetica-Bold");
    const font = Font.fromBytes(notoSans);
    b.font(bold);
    b.font(font);
    for (let n = 1; n <= REPORT_PAGES; n++) {
      const page = b.page("A4");
      const s = page.surface();
      s.text(`Section ${n}`, { font: bold, size: 18, at: [56, 72] });
      for (let k = 0; k < 6; k++) {
        s.textBox(SENTENCES, { font, size: 11, box: [56, 96 + 110 * k, 483, 100] });
      }
      page.finish();
    }
    const doc = b.finish();
    try {
      return doc.save();
    } finally {
      doc.free();
    }
  } finally {
    b.free();
  }
}

function time(name, runs, build) {
  build();
  const ms = [];
  for (let i = 0; i < runs; i++) {
    const start = performance.now();
    build();
    ms.push(performance.now() - start);
  }
  ms.sort((a, b) => a - b);
  const mid = Math.floor(runs / 2);
  const median = runs % 2 === 1 ? ms[mid] : (ms[mid - 1] + ms[mid]) / 2;
  console.info(`${name}: min ${ms[0].toFixed(1)} ms, median ${median.toFixed(1)} ms (${runs} runs)`);
}

const runs = Number(argv[2] ?? 10);
if (!Number.isInteger(runs) || runs < 1) {
  throw new Error(`runs must be a positive integer, not ${argv[2]}`);
}
await init({ text: true });
time("report", runs, () => report(notoSans, notoSansSC));
time("text-boxes", runs, textBoxes);
