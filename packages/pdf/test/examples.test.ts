// The TS examples produce the Rust examples' bytes (fixtures/golden/examples/, ADR 0018 §5), and
// the 100-page report its hash, in Node: the shared list (test/suite/examples.ts) every runtime
// of the B6c matrix runs. The performance smoke's figures go to stdout for
// `cargo xtask pdf-runtimes` (recorded, not asserted).
import { versions } from "node:process";
import { Builder, Font, init } from "@lamina/pdf";
import { beforeAll, describe, expect, it } from "vitest";
import { read } from "./helpers.js";
import {
  difference,
  EXAMPLES,
  fixturePaths,
  type Inputs,
  measure,
  printMeasurement,
  REPORT_SHA256,
  sha256,
} from "./suite/examples.js";

const inputs: Inputs = Object.fromEntries(fixturePaths().map((p) => [p, read(p)]));

beforeAll(() => init({ text: true }));

describe("examples", () => {
  for (const example of EXAMPLES) {
    it(`${example.name} builds the bytes of the Rust ${example.golden} golden`, async () => {
      const golden = read(`fixtures/golden/examples/${example.golden}.pdf`);
      expect(await difference(example.build(inputs), golden)).toBeUndefined();
    });
  }

  it("builds the 100-page report with the same bytes as every runtime, and measures", async () => {
    const { measurements, report } = measure(`node-${versions.node.split(".")[0]}`, inputs);
    for (const m of measurements) printMeasurement(m);
    expect(await sha256(report)).toBe(REPORT_SHA256);
  });

  it("uses one PDF font for equal fonts created per call", () => {
    const build = (font: () => Font) => {
      const b = new Builder({ id: "000102030405060708090a0b0c0d0e0f", creationDate: "D:20261030120000Z" });
      const page = b.page("A6");
      page.surface().text("one", { font: font(), size: 12, at: [10, 20] });
      page.surface().text("two", { font: font(), size: 12, at: [10, 40] });
      const noto = Font.fromBytes(read("fixtures/fonts/NotoSans-Regular.ttf"));
      page.surface().text("three", { font: noto, size: 12, at: [10, 60] });
      page
        .surface()
        .text("four", { font: Font.fromBytes(read("fixtures/fonts/NotoSans-Regular.ttf")), size: 12, at: [10, 80] });
      page.finish();
      return b.finish().save();
    };
    const helvetica = Font.standard("Helvetica");
    expect(build(() => Font.standard("Helvetica"))).toEqual(build(() => helvetica));
  });
});
