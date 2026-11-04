// The shared example list in workerd (Cloudflare's Workers runtime), through the `workerd` export
// condition: wrangler's test harness bundles and runs test/workerd/worker.ts and the Worker
// example (examples/worker/), and this side (Node) compares their output with the goldens.
// `@cloudflare/vitest-pool-workers` would run the tests inside workerd instead, but it supports
// Vitest 4 only; the examples still run in workerd either way. Run by
// `cargo xtask pdf-runtimes workerd` (vitest.workers.config.ts).
import { copyFile, mkdir } from "node:fs/promises";
import { fileURLToPath } from "node:url";
import { Document, init } from "@laminapdf/sdk";
import { fromRoot } from "@laminapdf/test-utils/root";
import { afterAll, beforeAll, describe, expect, it } from "vitest";
import { createTestHarness, type TestHarness } from "wrangler";
import { read } from "../helpers.js";
import {
  difference,
  EXAMPLES,
  goldenPath,
  MEASURE_TEST,
  type Measurement,
  printMeasurement,
  REPORT_SHA256,
} from "../suite/examples.js";

const config = (path: string): string => fileURLToPath(new URL(path, import.meta.url));

/** More distinct Latin Extended-A letters than Helvetica's 37 spare codes. */
const LATIN_EXTENDED_A = "ĀāĂăĄąĆćČčĎďĐđĒēĖėĘęĚěĞğĢģĪīĮįĶķĹĺĻļĽľŁłŃńŅņŇňŌōŐőŔŕ";

/** The Worker example bundles a font from its own src/fonts/ (gitignored): copy it there. */
async function copyExampleFont(): Promise<void> {
  const dir = config("../../examples/worker/src/fonts/");
  await mkdir(dir, { recursive: true });
  await copyFile(fromRoot("fixtures/fonts/NotoSans-Regular.ttf"), `${dir}NotoSans-Regular.ttf`);
}

let suite: TestHarness;
let example: TestHarness;

beforeAll(async () => {
  await copyExampleFont();
  suite = createTestHarness({ workers: [{ configPath: config("./wrangler.toml") }] });
  example = createTestHarness({ workers: [{ configPath: config("../../examples/worker/wrangler.toml") }] });
  await Promise.all([suite.listen(), example.listen(), init()]);
}, 120_000);

afterAll(() => Promise.all([suite?.close(), example?.close()]));

// The harness's Response type is undici's, not the DOM's.
const bytes = async (response: { status: number; arrayBuffer(): Promise<ArrayBuffer> }): Promise<Uint8Array> => {
  expect(response.status).toBe(200);
  return new Uint8Array(await response.arrayBuffer());
};

describe("workerd", () => {
  // First, so the WebAssembly memory it reports is only the hello and report runs'.
  it(MEASURE_TEST, async () => {
    const { measurements, reportSha256 } = (await (await suite.fetch("/measure")).json()) as {
      measurements: Measurement[];
      reportSha256: string;
    };
    for (const m of measurements) printMeasurement(m);
    expect(reportSha256).toBe(REPORT_SHA256);
  });

  for (const e of EXAMPLES) {
    it(`${e.name} builds the bytes of the Rust ${e.golden} golden`, async () => {
      const golden = read(goldenPath(e));
      expect(await difference(await bytes(await suite.fetch(`/example/${e.name}`)), golden)).toBeUndefined();
    });
  }

  // A name in a script Helvetica lacks (missing-glyph, kind input), and one with more distinct
  // non-WinAnsi Latin letters than its 37 spare codes (encoding-full, kind limit).
  for (const [name, code] of [
    ["Ада 世界", "missing-glyph"],
    [LATIN_EXTENDED_A, "encoding-full"],
  ] as const) {
    it(`answers a name Helvetica can't draw (${code}) with a 400 and the error, not a 500`, async () => {
      const response = await example.fetch(`/?name=${encodeURIComponent(name)}`);
      const body = (await response.json()) as { code?: string };
      expect([response.status, body.code]).toEqual([400, code]);
    });
  }

  // The bundled font (the Noto Sans subset) draws every Latin letter Helvetica ran out of codes
  // for, but has no Chinese.
  it("draws Latin Extended-A in the bundled font, and answers Chinese with a 400", async () => {
    await bytes(await example.fetch(`/embedded?name=${encodeURIComponent(LATIN_EXTENDED_A)}`));
    const response = await example.fetch(`/embedded?name=${encodeURIComponent("世界")}`);
    const body = (await response.json()) as { code?: string };
    expect([response.status, body.code]).toEqual([400, "missing-glyph"]);
  });

  // The Worker leaves the file ID and dates to the builder, so its letters aren't goldens: open
  // them instead. The one in the bundled font carries a subset of it, so it is the larger.
  it("serves the letter in Helvetica and in the bundled font", async () => {
    const name = encodeURIComponent("Zoë Łukasz");
    const standard = await bytes(await example.fetch(`/?name=${name}`));
    const embedded = await bytes(await example.fetch(`/embedded?name=${name}`));
    using a = Document.open(standard);
    using b = Document.open(embedded);
    expect([a.pageCount, b.pageCount, embedded.byteLength > standard.byteLength]).toEqual([1, 1, true]);
  });
});
