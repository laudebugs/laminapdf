// The shared example list (test/suite/examples.ts) in Deno, through the `deno` export condition
// (dist/index.js: the glue fetches its `.wasm` next to itself, a file: URL). Run by
// `cargo xtask pdf-runtimes deno`: `deno test --sloppy-imports --no-check --allow-read` (the
// suite imports `.ts` files by their `.js` paths, as tsc wants; tsc checks the types, with
// deno.d.ts standing in for Deno's).
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
const read = (path: string): Promise<Uint8Array> => Deno.readFile(new URL(path, root));

await init({ text: true });
const inputs: Inputs = Object.fromEntries(await Promise.all(fixturePaths().map(async (p) => [p, await read(p)])));

function check(problem: string | undefined): void {
  if (problem !== undefined) throw new Error(problem);
}

for (const example of EXAMPLES) {
  Deno.test(`${example.name} builds the bytes of the Rust ${example.golden} golden`, async () => {
    check(await difference(example.build(inputs), await read(`fixtures/golden/examples/${example.golden}.pdf`)));
  });
}

Deno.test("builds the 100-page report with the same bytes as every runtime, and measures", async () => {
  const { measurements, report } = measure("deno", inputs);
  for (const m of measurements) printMeasurement(m);
  const hash = await sha256(report);
  check(hash === REPORT_SHA256 ? undefined : `the report's sha256 is ${hash}, not ${REPORT_SHA256}`);
});
