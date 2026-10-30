// The one example list every runtime runs (B6c): Node and Vite (Vitest), Chromium, Firefox and
// WebKit (Vitest browser mode), Deno, Bun and workerd. Runtime-neutral: no Node API, no test
// framework; each runner supplies the fixture bytes (`Inputs`) its own way and compares the
// output with the Rust golden, so every runtime produces the same bytes.
import { wasmMemory } from "@lamina/pdf";
import { generateThenEdit } from "../../examples/generate-then-edit.js";
import { hello } from "../../examples/hello.js";
import { imagesAndVectors } from "../../examples/images-and-vectors.js";
import { invoice } from "../../examples/invoice.js";
import { linksAndOutlines } from "../../examples/links-and-outlines.js";
import { multilingual } from "../../examples/multilingual.js";
import { REPORT_PAGES, report } from "../../examples/report.js";

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
export function input(inputs: Inputs, path: string): Uint8Array {
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
];

/** The fonts of the 100-page report. */
export const REPORT_INPUTS = [NOTO, "fixtures/fonts/NotoSansSC-Regular.otf"] as const;

/**
 * The SHA-256 (hex) of the 100-page report's bytes: it has no Rust golden, so every runtime
 * checks its output against this. When a change to lamina-build or lamina-text changes the
 * output on purpose (the Rust goldens change too: `cargo xtask golden --update`), print the new
 * hash with `pnpm --dir packages/pdf exec vitest run -c vitest.node.config.ts test/examples.test.ts`
 * (the failure shows it) and update it here.
 */
export const REPORT_SHA256 = "fcf0579526a631fe375d3966c805d5b8e6b262e178f506a0958b8bc00f611601";

/** The 100-page report. Needs `init({ text: true })`. */
export function buildReport(inputs: Inputs): Uint8Array {
  return report(input(inputs, REPORT_INPUTS[0]), input(inputs, REPORT_INPUTS[1]));
}

export { REPORT_PAGES };

/** Every fixture the suite reads, golden files included. */
export function fixturePaths(): string[] {
  const paths = new Set<string>(REPORT_INPUTS);
  for (const e of EXAMPLES) {
    paths.add(`fixtures/golden/examples/${e.golden}.pdf`);
    for (const p of e.inputs) paths.add(p);
  }
  return [...paths].sort();
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
   * is its peak).
   */
  readonly case: string;
  /** Milliseconds, or bytes for `wasm-memory`. */
  readonly value: number;
}

/** The line prefix `cargo xtask pdf-runtimes` reads measurements from (stdout, any runner). */
export const MEASUREMENT_PREFIX = "LAMINA_MEASUREMENT ";

/** Prints `m` on its own line for `cargo xtask pdf-runtimes`. */
export function printMeasurement(m: Measurement): void {
  console.info(`${MEASUREMENT_PREFIX}${JSON.stringify(m)}`);
}

function median(values: number[]): number {
  const sorted = [...values].sort((a, b) => a - b);
  return sorted[Math.floor(sorted.length / 2)] ?? Number.NaN;
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
 * Needs `init({ text: true })`.
 */
export function measure(runtime: string, inputs: Inputs): { measurements: Measurement[]; report: Uint8Array } {
  const time = (f: () => unknown): number => {
    const start = performance.now();
    f();
    return performance.now() - start;
  };
  const first = time(hello);
  const hellos = Array.from({ length: 20 }, () => time(hello));
  let bytes: Uint8Array = new Uint8Array();
  const reports = Array.from({ length: 3 }, () =>
    time(() => {
      bytes = buildReport(inputs);
    }),
  );
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
