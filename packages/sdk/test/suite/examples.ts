// The one example list every runtime runs (B6c): Node and Vite (Vitest), Chromium, Firefox and
// WebKit (Vitest browser mode), Deno, Bun and workerd. Runtime-neutral: no Node API, no test
// framework; each runner supplies the fixture bytes its own way (`runSuite`'s `read`, or the
// Worker's bundled `INPUT_PATHS`) and the output is compared with the Rust golden, so every
// runtime produces the same bytes.
import { init, wasmMemory } from "@laminapdf/sdk";
import { generateThenEdit } from "../../examples/generate-then-edit.js";
import { hello } from "../../examples/hello.js";
import { imagesAndVectors } from "../../examples/images-and-vectors.js";
import { invoice } from "../../examples/invoice.js";
import { linksAndOutlines } from "../../examples/links-and-outlines.js";
import { multilingual } from "../../examples/multilingual.js";
import { pdfAArchive } from "../../examples/pdf-a-archive.js";
import { pdfA3InvoiceWithXml } from "../../examples/pdf-a3-invoice-with-xml.js";
import { pdfUaReport } from "../../examples/pdf-ua-report.js";
import { report } from "../../examples/report.js";

/** The fixture files an example reads, by path from the repository root. */
export type Inputs = Readonly<Record<string, Uint8Array>>;

/** One example: the files it reads, how it builds its bytes, and the golden they must equal. */
export interface Example {
  readonly name: string;
  /** `fixtures/golden/examples/<golden>.pdf`. */
  readonly golden: string;
  readonly inputs: readonly string[];
  build(inputs: Inputs): Uint8Array;
}

const NOTO = "fixtures/fonts/NotoSans-Regular.ttf";
const SCRIPT_FONTS = [
  NOTO,
  "fixtures/fonts/NotoSansArabic-Regular.ttf",
  "fixtures/fonts/NotoSansDevanagari-Regular.ttf",
  "fixtures/fonts/NotoSansHebrew-Regular.ttf",
  "fixtures/fonts/NotoSansSC-Regular.otf",
] as const;
const PHOTO = "fixtures/images/photo-rgb.jpg";

/** An input the runner was asked for (`input` says which), else an error naming it. */
function input(inputs: Inputs, path: string): Uint8Array {
  const bytes = inputs[path];
  if (bytes === undefined) throw new Error(`the runner did not supply ${path}`);
  return bytes;
}

/** The TS examples with Rust goldens (`fixtures/golden/examples/`, ADR 0018 §5). */
export const EXAMPLES: readonly Example[] = [
  { name: "hello", golden: "hello", inputs: [], build: () => hello() },
  { name: "invoice", golden: "invoice", inputs: [NOTO], build: (i) => invoice(input(i, NOTO)) },
  {
    name: "multilingual",
    golden: "multilingual",
    inputs: SCRIPT_FONTS,
    build: (i) => multilingual(SCRIPT_FONTS.map((f) => input(i, f))),
  },
  {
    name: "images-and-vectors",
    golden: "vectors",
    inputs: [PHOTO],
    build: (i) => imagesAndVectors(input(i, PHOTO)),
  },
  { name: "links-and-outlines", golden: "links_and_outlines", inputs: [], build: () => linksAndOutlines() },
  { name: "generate-then-edit", golden: "generate_then_edit", inputs: [], build: () => generateThenEdit() },
  { name: "pdf-ua-report", golden: "pdf_ua_report", inputs: [NOTO], build: (i) => pdfUaReport(input(i, NOTO)) },
  {
    name: "pdf-a-archive-2b",
    golden: "pdf_a_archive-2b",
    inputs: [NOTO, PHOTO],
    build: (i) => pdfAArchive("2b", input(i, NOTO), input(i, PHOTO)),
  },
  {
    name: "pdf-a-archive-2u",
    golden: "pdf_a_archive-2u",
    inputs: [NOTO, PHOTO],
    build: (i) => pdfAArchive("2u", input(i, NOTO), input(i, PHOTO)),
  },
  {
    name: "pdf-a3-invoice-with-xml",
    golden: "pdf_a3_invoice_with_xml",
    inputs: [NOTO],
    build: (i) => pdfA3InvoiceWithXml(input(i, NOTO)),
  },
];

/** The fonts of the 100-page report. */
export const REPORT_INPUTS = [NOTO, "fixtures/fonts/NotoSansSC-Regular.otf"] as const;

/**
 * The SHA-256 (hex) of the 100-page report's bytes: it has no Rust golden, so every runtime
 * checks its output against this. When a change to lamina-build or lamina-text changes the
 * output on purpose (the Rust goldens change too: `cargo xtask golden --update`), print the new
 * hash with `pnpm --dir packages/sdk exec vitest run -c vitest.node.config.ts --project measure`
 * (the failure shows it) and update it here.
 */
export const REPORT_SHA256 = "fcf0579526a631fe375d3966c805d5b8e6b262e178f506a0958b8bc00f611601";

/** The 100-page report. Needs `init({ text: true })`. */
function buildReport(inputs: Inputs): Uint8Array {
  return report(input(inputs, REPORT_INPUTS[0]), input(inputs, REPORT_INPUTS[1]));
}

/** An example's golden, by path from the repository root. */
export function goldenPath(example: Example): string {
  return `fixtures/golden/examples/${example.golden}.pdf`;
}

/** Every file the examples and the report read (not the goldens), sorted: what a Worker bundles. */
export const INPUT_PATHS: readonly string[] = [
  ...new Set<string>([...REPORT_INPUTS, ...EXAMPLES.flatMap((e) => e.inputs)]),
].sort();

/** Every fixture the suite reads: the inputs and the goldens, sorted. */
const FIXTURE_PATHS: readonly string[] = [...INPUT_PATHS, ...EXAMPLES.map(goldenPath)].sort();

/** The paths of `paths` that `inputs` lacks (a runner that bundles its inputs checks this is empty). */
export function missingInputs(inputs: Inputs, paths: readonly string[] = INPUT_PATHS): string[] {
  return paths.filter((p) => inputs[p] === undefined);
}

/** How a runner reads a fixture (by path from the repository root). */
export type Read = (path: string) => Uint8Array | Promise<Uint8Array>;

/** How a runner registers a test (Vitest's `it`, Bun's `test`, `Deno.test`). */
export type Register = (name: string, body: () => Promise<void>) => void;

/** What {@link runSuite} registers: everything (default), only the measurement, or only the examples. */
export type SuitePart = "all" | "measure" | "examples";

/** The report's test name, the same in every runtime. */
export const MEASURE_TEST = "builds the 100-page report with the same bytes as every runtime, and measures";

/**
 * Registers the suite with a runner's test function: first the report's hash check with the
 * performance smoke (so it runs cold, in the file's fresh module instance), then one test per
 * example comparing its bytes with the golden. The first test to run calls `init({ text: true })`
 * and reads every fixture once with `read`; failures throw an `Error` saying what differed.
 */
export function runSuite(runtime: string, read: Read, register: Register, part: SuitePart = "all"): void {
  let loading: Promise<Inputs> | undefined;
  const inputs = (): Promise<Inputs> => {
    loading ??= (async () => {
      await init({ text: true });
      return Object.fromEntries(await Promise.all(FIXTURE_PATHS.map(async (p) => [p, await read(p)] as const)));
    })();
    return loading;
  };
  if (part !== "examples") {
    register(MEASURE_TEST, async () => {
      const { measurements, report } = await measure(runtime, await inputs());
      for (const m of measurements) printMeasurement(m);
      const hash = await sha256(report);
      if (hash !== REPORT_SHA256) throw new Error(`the report's sha256 is ${hash}, not ${REPORT_SHA256}`);
    });
  }
  if (part === "measure") return;
  for (const example of EXAMPLES) {
    register(`${example.name} builds the bytes of the Rust ${example.golden} golden`, async () => {
      const loaded = await inputs();
      const problem = await difference(example.build(loaded), input(loaded, goldenPath(example)));
      if (problem !== undefined) throw new Error(problem);
    });
  }
}

/** Lowercase hex SHA-256 with Web Crypto (every runtime has it). */
export async function sha256(bytes: Uint8Array): Promise<string> {
  const digest = new Uint8Array(await crypto.subtle.digest("SHA-256", bytes as Uint8Array<ArrayBuffer>));
  return Array.from(digest, (b) => b.toString(16).padStart(2, "0")).join("");
}

/** One timing or memory figure of the performance smoke (pdf-library §4; recorded, not asserted). */
export interface Measurement {
  readonly runtime: string;
  /**
   * `hello-first` (the first call after `init()`), `hello` (median), `report` (median), or
   * `wasm-memory` (both modules' linear memory after the reports: memory never shrinks, so this
   * is its peak; hello() and report() free their builder and document, so it is one run's peak,
   * not the garbage collector's backlog).
   */
  readonly case: string;
  /** Milliseconds, or bytes for `wasm-memory`. */
  readonly value: number;
}

/** The line prefix `cargo xtask pdf-runtimes` reads measurements from (stdout, any runner). */
const MEASUREMENT_PREFIX = "LAMINA_MEASUREMENT ";

/** Prints `m` on its own line for `cargo xtask pdf-runtimes`. */
export function printMeasurement(m: Measurement): void {
  console.info(`${MEASUREMENT_PREFIX}${JSON.stringify(m)}`);
}

/** The median; for an even count, the mean of the two middle values. */
function median(values: readonly number[]): number {
  const sorted = [...values].sort((a, b) => a - b);
  const mid = Math.floor(sorted.length / 2);
  const upper = sorted[mid] ?? Number.NaN;
  return sorted.length % 2 === 1 ? upper : ((sorted[mid - 1] ?? Number.NaN) + upper) / 2;
}

/**
 * Why `actual` isn't the golden's bytes (sizes and hashes, not a byte dump), or `undefined`
 * when they are equal.
 */
export async function difference(actual: Uint8Array, golden: Uint8Array): Promise<string | undefined> {
  const [a, g] = [await sha256(actual), await sha256(golden)];
  return a === g ? undefined : `${actual.length} bytes (sha256 ${a}) instead of the golden's ${golden.length} (${g})`;
}

/**
 * Times hello (the first call after `init()`, then the median of 20) and the 100-page report
 * (median of 3) with `performance.now()`. Returns the report's bytes, for the hash check.
 * Needs `init({ text: true })`, and must run first in a fresh module instance (before any other
 * example): then `hello-first` is cold and `wasm-memory` is the hello and report runs' alone.
 *
 * @param settle - awaited before reading the clock at the start and the end of each sample: in
 *   workerd the clock only advances across I/O, so the Worker passes a zero-length timer; elsewhere
 *   it does nothing.
 */
export async function measure(
  runtime: string,
  inputs: Inputs,
  settle: () => Promise<void> = async () => {},
): Promise<{ measurements: Measurement[]; report: Uint8Array }> {
  const time = async (f: () => unknown): Promise<number> => {
    await settle();
    const start = performance.now();
    f();
    await settle();
    return performance.now() - start;
  };
  const first = await time(hello);
  const hellos: number[] = [];
  for (let i = 0; i < 20; i++) hellos.push(await time(hello));
  // report() returns a JS copy of the saved bytes and frees its document, so the last run's
  // bytes stay valid for the hash check.
  let bytes: Uint8Array = new Uint8Array();
  const reports: number[] = [];
  for (let i = 0; i < 3; i++) {
    reports.push(
      await time(() => {
        bytes = buildReport(inputs);
      }),
    );
  }
  const memory = wasmMemory();
  return {
    measurements: [
      { runtime, case: "hello-first", value: first },
      { runtime, case: "hello", value: median(hellos) },
      { runtime, case: "report", value: median(reports) },
      { runtime, case: "wasm-memory", value: (memory.base ?? 0) + (memory.text ?? 0) },
    ],
    report: bytes,
  };
}
