// The shared example list in workerd (Cloudflare's Workers runtime), through the `workerd` export
// condition: wrangler's test harness bundles and runs test/workerd/worker.ts and the Worker
// example (examples/worker/), and this side (Node) compares their output with the goldens.
// `@cloudflare/vitest-pool-workers` would run the tests inside workerd instead, but it supports
// Vitest 4 only; the examples still run in workerd either way. Run by
// `cargo xtask pdf-runtimes workerd` (vitest.workers.config.ts).
import { fileURLToPath } from "node:url";
import { afterAll, beforeAll, describe, expect, it } from "vitest";
import { createTestHarness, type TestHarness } from "wrangler";
import { read } from "../helpers.js";
import { difference, EXAMPLES, type Measurement, printMeasurement, REPORT_SHA256, sha256 } from "../suite/examples.js";

const config = (path: string): string => fileURLToPath(new URL(path, import.meta.url));

let suite: TestHarness;
let example: TestHarness;

beforeAll(async () => {
  suite = createTestHarness({ workers: [{ configPath: config("./wrangler.toml") }] });
  example = createTestHarness({ workers: [{ configPath: config("../../examples/worker/wrangler.toml") }] });
  await Promise.all([suite.listen(), example.listen()]);
}, 120_000);

afterAll(() => Promise.all([suite?.close(), example?.close()]));

// The harness's Response type is undici's, not the DOM's.
const bytes = async (response: { status: number; arrayBuffer(): Promise<ArrayBuffer> }): Promise<Uint8Array> => {
  expect(response.status).toBe(200);
  return new Uint8Array(await response.arrayBuffer());
};

describe("workerd", () => {
  // First, so the WebAssembly memory it reports is only the hello and report runs'.
  it("builds the 100-page report with the same bytes as every runtime, and measures", async () => {
    const { measurements, reportSha256 } = (await (await suite.fetch("/measure")).json()) as {
      measurements: Measurement[];
      reportSha256: string;
    };
    for (const m of measurements) printMeasurement(m);
    expect(reportSha256).toBe(REPORT_SHA256);
  });

  for (const e of EXAMPLES) {
    it(`${e.name} builds the bytes of the Rust ${e.golden} golden`, async () => {
      const golden = read(`fixtures/golden/examples/${e.golden}.pdf`);
      expect(await difference(await bytes(await suite.fetch(`/example/${e.name}`)), golden)).toBeUndefined();
    });
  }

  it("answers a name Helvetica can't draw with a 400 and the error, not a 500", async () => {
    const response = await example.fetch(`/?name=${encodeURIComponent("Ада 世界")}`);
    const body = (await response.json()) as { code?: string };
    expect([response.status, body.code]).toEqual([400, "missing-glyph"]);
  });

  it("serves a letter and the report from the Worker example", async () => {
    const letter = await bytes(await example.fetch(`/?name=${encodeURIComponent("Zoë Łukasz")}`));
    const report = await bytes(await example.fetch("/report"));
    expect([new TextDecoder().decode(letter.subarray(0, 5)), await sha256(report)]).toEqual(["%PDF-", REPORT_SHA256]);
  });
});
