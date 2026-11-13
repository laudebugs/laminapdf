// The one example list every runtime runs (B6c): Node and Vite (Vitest), Chromium, Firefox and
// WebKit (Vitest browser mode), Deno, Bun and workerd. Runtime-neutral: no Node API, no test
// framework; each runner supplies the fixture bytes its own way (`runSuite`'s `read`, or the
// Worker's bundled `INPUT_PATHS`) and the output is compared with the Rust golden, so every
// runtime produces the same bytes.
import { type BuilderOptions, type FlowTemplate, init, wasmMemory } from "@laminapdf/sdk";
import { duplicatePages } from "../../examples/duplicate-pages.js";
import { editBookmarks } from "../../examples/edit-bookmarks.js";
import { editMetadata } from "../../examples/edit-metadata.js";
import { editXmp } from "../../examples/edit-xmp.js";
import { elementFlow, type Report } from "../../examples/element-flow.js";
import { extractPages } from "../../examples/extract-pages.js";
import { generateThenEdit } from "../../examples/generate-then-edit.js";
import { hello } from "../../examples/hello.js";
import { imagesAndVectors } from "../../examples/images-and-vectors.js";
import { insertPages } from "../../examples/insert-pages.js";
import { invoice } from "../../examples/invoice.js";
import { invoiceFromJson } from "../../examples/invoice-from-json.js";
import { linksAndOutlines } from "../../examples/links-and-outlines.js";
import { markdownToPdf } from "../../examples/markdown-to-pdf.js";
import { multilingual } from "../../examples/multilingual.js";
import { organisePages } from "../../examples/organise-pages.js";
import { pageLabels } from "../../examples/page-labels.js";
import { pdfAArchive } from "../../examples/pdf-a-archive.js";
import { pdfA3InvoiceWithXml } from "../../examples/pdf-a3-invoice-with-xml.js";
import { pdfUaReport } from "../../examples/pdf-ua-report.js";
import { report } from "../../examples/report.js";
import { openOn } from "../../examples/set-initial-view.js";
import { exampleOptions, HELLO_OPTIONS } from "./hello-options.js";

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
/** The report the element-flow example lays out; its body runs onto a second page. */
const FLOW_REPORT = "fixtures/text/element-flow-report.json";
/** The Markdown report and the fonts it is set in (the bold face is the collection's second). */
const MARKDOWN = "fixtures/markdown/report.md";
const MARKDOWN_FONTS = [
  NOTO,
  "fixtures/fonts/NotoSans-RegularBold.ttc",
  "fixtures/fonts/NotoSansArabic-Regular.ttf",
  "fixtures/fonts/NotoSansDevanagari-Regular.ttf",
] as const;
/** The invoice template, its data, and its bold face (the collection's second). */
const JSON_INVOICE = ["fixtures/documents/invoice.json", "fixtures/documents/invoice-data.json"] as const;
const BOLD = "fixtures/fonts/NotoSans-RegularBold.ttc";
/** The modification date generate-then-edit and edit-metadata record, fixed like the options. */
const EDITED_AT = "D:20261030130000Z";
/** The document edit-metadata edits. */
const METADATA_PDF = "fixtures/golden/metadata.pdf";
/** The document set-initial-view edits: three pages with an outline. */
const OUTLINE_PDF = "fixtures/corpus/outline.pdf";
/** The document edit-bookmarks edits. */
const BOOKMARKS_PDF = "fixtures/golden/links-and-outlines.pdf";
/** The PDF/A-2b file edit-xmp records a review in. */
const PDFA_2B_PDF = "fixtures/golden/examples/pdf_a_archive-2b.pdf";
/** The document organise-pages organises: five pages, with bookmarks, links and named destinations that point at the third. */
const REFERENCES_PDF = "fixtures/pages/references.pdf";
/** The input of duplicate-pages: two form pages, a text field, a checkbox field and a signature field. */
const FORMS_PDF = "fixtures/pages/forms.pdf";
/** The input of page-labels: an example golden of four pages. */
const LABELS_INPUT_PDF = "fixtures/golden/examples/links_and_outlines.pdf";
/** The input of extract-pages: ten pages labelled i–iii, 1–4, A-1, A-2 and Back cover. */
const LABELS_PDF = "fixtures/pages/labels.pdf";

/** An input the runner was asked for (`input` says which), else an error naming it. */
function input(inputs: Inputs, path: string): Uint8Array {
  const bytes = inputs[path];
  if (bytes === undefined) throw new Error(`the runner did not supply ${path}`);
  return bytes;
}

/** A JSON input, parsed. */
function json(inputs: Inputs, path: string): unknown {
  return JSON.parse(new TextDecoder().decode(input(inputs, path)));
}

/** The two-page invoice, filled from its data. Needs `init({ text: true })`. */
function buildInvoiceFromJson(inputs: Inputs): Uint8Array {
  const [template, data] = JSON_INVOICE.map((p) => json(inputs, p));
  return invoiceFromJson(
    template as FlowTemplate,
    data,
    input(inputs, NOTO),
    input(inputs, BOLD),
    exampleOptions("lamina-ex-jsoniv"),
  );
}

/** The report of `FLOW_REPORT`, as the runner supplied it. */
function flowReport(inputs: Inputs): Report {
  return JSON.parse(new TextDecoder().decode(input(inputs, FLOW_REPORT))) as Report;
}

/** The TS examples with Rust goldens (`fixtures/golden/examples/`, ADR 0018 §5). */
export const EXAMPLES: readonly Example[] = [
  { name: "hello", golden: "hello", inputs: [], build: () => hello(HELLO_OPTIONS) },
  {
    name: "invoice",
    golden: "invoice",
    inputs: [NOTO],
    build: (i) => invoice(input(i, NOTO), exampleOptions("lamina-ex-invoic")),
  },
  {
    name: "multilingual",
    golden: "multilingual",
    inputs: SCRIPT_FONTS,
    build: (i) =>
      multilingual(
        SCRIPT_FONTS.map((f) => input(i, f)),
        exampleOptions("lamina-ex-multil"),
      ),
  },
  {
    name: "markdown-to-pdf",
    golden: "markdown_to_pdf",
    inputs: [MARKDOWN, ...MARKDOWN_FONTS, PHOTO],
    build: (i) => {
      const [regular, bold, arabic, devanagari] = MARKDOWN_FONTS.map((f) => input(i, f));
      if (regular === undefined || bold === undefined || arabic === undefined || devanagari === undefined) {
        throw new Error("four fonts");
      }
      return markdownToPdf(
        new TextDecoder().decode(input(i, MARKDOWN)),
        { regular, bold, arabic, devanagari },
        input(i, PHOTO),
        exampleOptions("lamina-ex-mrkdwn"),
      );
    },
  },
  {
    name: "images-and-vectors",
    golden: "images_and_vectors",
    inputs: [PHOTO],
    build: (i) => imagesAndVectors(input(i, PHOTO), exampleOptions("lamina-ex-vector")),
  },
  {
    name: "links-and-outlines",
    golden: "links_and_outlines",
    inputs: [],
    build: () => linksAndOutlines(exampleOptions("lamina-ex-links!")),
  },
  {
    name: "generate-then-edit",
    golden: "generate_then_edit",
    inputs: [],
    build: () => generateThenEdit(exampleOptions("lamina-ex-edit!!"), EDITED_AT),
  },
  {
    name: "edit-metadata",
    golden: "edit_metadata",
    inputs: [METADATA_PDF],
    build: (i) => editMetadata(input(i, METADATA_PDF), EDITED_AT),
  },
  {
    name: "set-initial-view",
    golden: "set_initial_view",
    inputs: [OUTLINE_PDF],
    build: (i) => openOn(input(i, OUTLINE_PDF), 1),
  },
  {
    name: "edit-bookmarks",
    golden: "edit_bookmarks",
    inputs: [BOOKMARKS_PDF],
    build: (i) => editBookmarks(input(i, BOOKMARKS_PDF)),
  },
  {
    name: "duplicate-pages",
    golden: "duplicate_pages",
    inputs: [FORMS_PDF],
    build: (i) => duplicatePages(input(i, FORMS_PDF)),
  },
  {
    name: "insert-pages",
    golden: "insert_pages",
    inputs: [FORMS_PDF, REFERENCES_PDF],
    build: (i) => insertPages(input(i, FORMS_PDF), input(i, REFERENCES_PDF)),
  },
  {
    name: "extract-pages",
    golden: "extract_pages",
    inputs: [LABELS_PDF],
    build: (i) => extractPages(input(i, LABELS_PDF)),
  },
  {
    name: "edit-xmp",
    golden: "edit_xmp",
    inputs: [PDFA_2B_PDF],
    build: (i) => editXmp(input(i, PDFA_2B_PDF), EDITED_AT),
  },
  {
    name: "organise-pages",
    golden: "organise_pages",
    inputs: [REFERENCES_PDF],
    build: (i) => organisePages(input(i, REFERENCES_PDF)),
  },
  {
    name: "page-labels",
    golden: "page_labels",
    inputs: [LABELS_INPUT_PDF],
    build: (i) => pageLabels(input(i, LABELS_INPUT_PDF)),
  },
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
  {
    name: "element-flow",
    golden: "element_flow",
    inputs: [PHOTO, FLOW_REPORT],
    build: (i) => elementFlow(input(i, PHOTO), flowReport(i), exampleOptions("lamina-ex-flow!!")),
  },
  {
    name: "invoice-from-json",
    golden: "invoice_from_json",
    inputs: [...JSON_INVOICE, NOTO, BOLD],
    build: buildInvoiceFromJson,
  },
];

/** The fonts of the 100-page report. */
export const REPORT_INPUTS = [NOTO, "fixtures/fonts/NotoSansSC-Regular.otf"] as const;

/**
 * The SHA-256 (hex) of the 100-page report's bytes: it has no Rust golden, so every runtime
 * checks its output against this. When a change to laminapdf-build or laminapdf-text changes the
 * output on purpose (the Rust goldens change too: `cargo xtask golden --update`), print the new
 * hash with `pnpm --dir packages/sdk exec vitest run -c vitest.node.config.ts --project measure`
 * (the failure shows it) and update it here.
 */
export const REPORT_SHA256 = "09325fa371a5223ee82fe868bf66ecd6e3beb3d3e71ffddb4bd4406909b06516";

/** The 100-page report. Needs `init({ text: true })`. */
function buildReport(inputs: Inputs): Uint8Array {
  return report(input(inputs, REPORT_INPUTS[0]), input(inputs, REPORT_INPUTS[1]), exampleOptions("lamina-ex-report"));
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

/** A quick-start app (`examples/quick-start/<runtime>.ts`): builds hello and hands the bytes to `save`. */
export type QuickStartApp = (
  save: (pdf: Uint8Array) => void | Promise<void>,
  options?: BuilderOptions,
) => Promise<void>;

/** Registers `runtime`'s quick-start app, which must hand `save` the hello golden's bytes. */
export function registerQuickStart(runtime: string, read: Read, register: Register, app: QuickStartApp): void {
  register(`the ${runtime} quick start builds the bytes of the Rust hello golden`, async () => {
    let saved: Uint8Array | undefined;
    await app((pdf) => {
      saved = pdf;
    }, HELLO_OPTIONS);
    if (saved === undefined) throw new Error(`the ${runtime} quick start saved nothing`);
    const problem = await difference(saved, await read("fixtures/golden/examples/hello.pdf"));
    if (problem !== undefined) throw new Error(problem);
  });
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
   * `hello-first` (the first call after `init()`), `hello` (median), `invoice` (median),
   * `invoice-from-json` (best of 30, ADR 0032 §9's figure), `report` (median), or
   * `wasm-memory` (both modules' linear memory after the reports: memory never shrinks, so this
   * is its peak; hello() and report() free their builder and document, so it is one run's peak,
   * not the garbage collector's backlog).
   */
  readonly case: string;
  /** Milliseconds, or bytes for `wasm-memory`. */
  readonly value: number;
}

/** The line prefix `cargo xtask pdf-runtimes` reads measurements from (stdout, any runner). */
const MEASUREMENT_PREFIX = "LAMINA_PDF_MEASUREMENT ";

/** Prints `m` on its own line for `cargo xtask pdf-runtimes`. */
export function printMeasurement(m: Measurement): void {
  console.info(`${MEASUREMENT_PREFIX}${JSON.stringify(m)}`);
}

/** The example named `name`, else an error. */
function example(name: string): Example {
  const found = EXAMPLES.find((e) => e.name === name);
  if (found === undefined) throw new Error(`the suite has no ${name} example`);
  return found;
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
 * Times hello (the first call after `init()`, then the median of 20), the invoice example (the
 * median of 20; its bytes are checked against the Rust golden by the suite's own test), the
 * two-page invoice filled from JSON (the best of 30) and the
 * 100-page report (median of 3) with `performance.now()`. Returns the report's bytes, for the hash check.
 * Needs `init({ text: true })`, and must run first in a fresh module instance (before any other
 * example): then `hello-first` is cold and `wasm-memory` is the timed runs' alone.
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
  const first = await time(() => hello(HELLO_OPTIONS));
  const hellos: number[] = [];
  for (let i = 0; i < 20; i++) hellos.push(await time(() => hello(HELLO_OPTIONS)));
  const invoiceExample = example("invoice");
  const invoices: number[] = [];
  for (let i = 0; i < 20; i++) invoices.push(await time(() => invoiceExample.build(inputs)));
  const fromJson = example("invoice-from-json");
  const filled: number[] = [];
  for (let i = 0; i < 30; i++) filled.push(await time(() => fromJson.build(inputs)));
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
      { runtime, case: "invoice", value: median(invoices) },
      { runtime, case: "invoice-from-json", value: Math.min(...filled) },
      { runtime, case: "report", value: median(reports) },
      { runtime, case: "wasm-memory", value: (memory.base ?? 0) + (memory.text ?? 0) },
    ],
    report: bytes,
  };
}
