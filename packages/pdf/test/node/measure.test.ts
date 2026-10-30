// The performance smoke in Node (pdf-library §4; recorded, not asserted): its own file, so it
// runs in a fresh module instance (Vitest isolates files) before any other example, and the
// report's hash check. The `measure` project of vitest.node.config.ts, run by
// `cargo xtask pdf-runtimes node`; `cargo xtask wasm` leaves it out (its four 100-page reports).
import { versions } from "node:process";
import { init } from "@lamina/pdf";
import { expect, it } from "vitest";
import { read } from "../helpers.js";
import { fixturePaths, type Inputs, measure, printMeasurement, REPORT_SHA256, sha256 } from "../suite/examples.js";

it("builds the 100-page report with the same bytes as every runtime, and measures", async () => {
  await init({ text: true });
  const inputs: Inputs = Object.fromEntries(fixturePaths().map((p) => [p, read(p)]));
  const { measurements, report } = await measure(`node-${versions.node.split(".")[0]}`, inputs);
  for (const m of measurements) printMeasurement(m);
  expect(await sha256(report)).toBe(REPORT_SHA256);
});
