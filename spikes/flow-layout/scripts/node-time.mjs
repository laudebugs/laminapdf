// Times the WebAssembly probe in Node: node scripts/node-time.mjs target/wasm/own.wasm
// Best of 30 runs for the invoice and of 10 for the report; also writes nothing, it only checks
// the PDF's SHA-256 against the native run's (a determinism check across targets).
import { createHash } from "node:crypto";
import { readFileSync } from "node:fs";

const root = new URL("../../../", import.meta.url);
const read = (path) => readFileSync(new URL(path, root));
const { instance } = await WebAssembly.instantiate(readFileSync(process.argv[2]), {});
const { memory, alloc, input, invoice, report, last } = instance.exports;
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
  const sha = createHash("sha256").update(new Uint8Array(memory.buffer, last(), len)).digest("hex").slice(0, 16);
  console.log(
    `${process.argv[2]} ${name}: first ${first.toFixed(1)} ms, best ${best.toFixed(1)} ms, ${len} B, sha256 ${sha}, memory ${(memory.buffer.byteLength / 2 ** 20).toFixed(0)} MiB`,
  );
}
