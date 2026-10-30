// The shared example list (test/suite/examples.ts) in a browser (Chromium, Firefox, WebKit:
// vitest.browser.config.ts), through the `browser` export condition. The fixtures are fetched
// as Vite assets (`?url` imports), not read with a Node API.
import { init } from "@lamina/pdf";
import { beforeAll, describe, expect, it } from "vitest";
import { server } from "vitest/browser";
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

// Every fixture the suite may read, as the URL the Vite server serves it at.
const urls = import.meta.glob<string>(
  [
    "../../../../fixtures/golden/examples/*.pdf",
    "../../../../fixtures/fonts/*.{ttf,otf}",
    "../../../../fixtures/images/photo-rgb.jpg",
  ],
  { query: "?url", import: "default", eager: true },
);

async function read(path: string): Promise<Uint8Array> {
  const url = urls[`../../../../${path}`];
  if (url === undefined) throw new Error(`${path} is not among the fixtures this test serves`);
  const response = await fetch(url);
  if (!response.ok) throw new Error(`fetching ${path}: ${response.status}`);
  return new Uint8Array(await response.arrayBuffer());
}

let inputs: Inputs = {};

beforeAll(async () => {
  await init({ text: true });
  inputs = Object.fromEntries(await Promise.all(fixturePaths().map(async (p) => [p, await read(p)])));
});

describe(`examples in ${server.browser}`, () => {
  // First, in this file's fresh module instance: hello-first is cold and the memory is the
  // hello and report runs' alone (measure()).
  it("builds the 100-page report with the same bytes as every runtime, and measures", async () => {
    const { measurements, report } = await measure(server.browser, inputs);
    for (const m of measurements) printMeasurement(m);
    expect(await sha256(report)).toBe(REPORT_SHA256);
  });

  for (const example of EXAMPLES) {
    it(`${example.name} builds the bytes of the Rust ${example.golden} golden`, async () => {
      const golden = await read(`fixtures/golden/examples/${example.golden}.pdf`);
      expect(await difference(example.build(inputs), golden)).toBeUndefined();
    });
  }
});
