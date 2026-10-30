// The shared example list (test/suite/examples.ts) in Bun, through the `node` export condition
// (Bun resolves it like Node: the `.wasm` read from @lamina/wasm). Run by
// `cargo xtask pdf-runtimes bun` (`bun test ./test/bun/`); bun.d.ts declares `bun:test` for tsc.

import { beforeAll, expect, test } from "bun:test";
import { readFile } from "node:fs/promises";
import { init } from "@lamina/pdf";
import {
  difference,
  EXAMPLES,
  fixturePaths,
  type Inputs,
  measure,
  printMeasurement,
  REPORT_SHA256,
  sha256,
} from "../suite/examples.js";

const root = new URL("../../../../", import.meta.url);
const read = async (path: string): Promise<Uint8Array> => new Uint8Array(await readFile(new URL(path, root)));

let inputs: Inputs = {};

beforeAll(async () => {
  await init({ text: true });
  inputs = Object.fromEntries(await Promise.all(fixturePaths().map(async (p) => [p, await read(p)])));
});

// First, in this fresh module instance: hello-first is cold and the memory is the hello and
// report runs' alone (measure()).
test("builds the 100-page report with the same bytes as every runtime, and measures", async () => {
  const { measurements, report } = await measure("bun", inputs);
  for (const m of measurements) printMeasurement(m);
  expect(await sha256(report)).toBe(REPORT_SHA256);
});

for (const example of EXAMPLES) {
  test(`${example.name} builds the bytes of the Rust ${example.golden} golden`, async () => {
    const golden = await read(`fixtures/golden/examples/${example.golden}.pdf`);
    expect(await difference(example.build(inputs), golden)).toBeUndefined();
  });
}
