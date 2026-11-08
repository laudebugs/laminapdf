// Times the WebAssembly probe in Node and compares its output with the native run's:
//   node scripts/node-time.mjs target/wasm/own.wasm a      (a: experiment A's files, b: B's)
// Best of 30 runs for the invoice and of 10 for the report. The last PDF of each is compared byte
// for byte with target/flow-layout/<prefix>-<name>.pdf, which `cargo run --release` writes; the
// script exits 1 when they differ.
import { existsSync, readFileSync } from "node:fs";

const root = new URL("../../../", import.meta.url);
const read = (path) => readFileSync(new URL(path, root));
const { instance } = await WebAssembly.instantiate(readFileSync(process.argv[2]), {});
const { memory, alloc, input, invoice, report, last } = instance.exports;
const prefix = process.argv[3];
let differs = false;
const inputs = [
  "fixtures/fonts/NotoSans-Regular.ttf",
  "fixtures/fonts/NotoSans-RegularBold.ttc",
  "fixtures/fonts/NotoSansArabic-Regular.ttf",
  "fixtures/fonts/NotoSansDevanagari-Regular.ttf",
  "spikes/flow-layout/inputs/invoice.json",
  "spikes/flow-layout/inputs/invoice.typ",
];
inputs.forEach((path, slot) => {
  const bytes = read(path);
  const ptr = alloc(bytes.length);
  new Uint8Array(memory.buffer, ptr, bytes.length).set(bytes);
  input(slot, ptr, bytes.length);
});
for (const [name, run, n] of [["invoice", invoice, 30], ["report", report, 10]]) {
  let best = Infinity;
  let first = 0;
  let len = 0;
  for (let i = 0; i < n; i++) {
    const start = performance.now();
    len = run();
    const ms = performance.now() - start;
    if (i === 0) first = ms;
    best = Math.min(best, ms);
  }
  const pdf = new Uint8Array(memory.buffer, last(), len);
  const nativePath = new URL(`../target/flow-layout/${prefix}-${name}.pdf`, import.meta.url);
  let same = "no native file";
  if (prefix && existsSync(nativePath)) {
    const native = readFileSync(nativePath);
    same = String(native.length === len && native.every((byte, i) => byte === pdf[i]));
    differs ||= same === "false";
  }
  console.log(
    `${process.argv[2]} ${name}: first ${first.toFixed(1)} ms, best ${best.toFixed(1)} ms, ${len} B, same bytes as native: ${same}, memory ${(memory.buffer.byteLength / 2 ** 20).toFixed(0)} MiB`,
  );
}
if (differs) process.exit(1);
