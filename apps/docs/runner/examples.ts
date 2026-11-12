// The examples a reader can edit and run on their pages (todo B7e; the docs examples plan, §1;
// "Your first PDF"'s Node app, todo Q4d):
// which function each file exports, and the inputs it takes, in the order it takes them. The
// inputs are the files the SDK's test suite passes (packages/sdk/test/suite/examples.ts); the
// worker (worker.ts) serves them as its own assets.

/** One input: a file's bytes, or its JSON parsed. */
interface FileInput {
  /** The path from the repository root, as the test suite names it. */
  readonly file: string;
  readonly as: "bytes" | "json";
}

/**
 * The fixed `BuilderOptions` the suite passes (its `exampleOptions(id)` in
 * packages/sdk/test/suite/hello-options.ts), so the bytes are the golden's. The e2e suite checks
 * that each example run unchanged gives its golden, so a change there shows here.
 */
interface OptionsInput {
  /** The file ID, 16 ASCII characters. */
  readonly options: string;
}

/**
 * The function a quick start hands its PDF to (its `save`), which the runner gives it in place of
 * writing a file: what it is handed is the run's PDF.
 */
interface SaveInput {
  readonly save: true;
}

export type RunInput = FileInput | OptionsInput | SaveInput;

/** The producer and creation date the suite's `exampleOptions` fix. */
export const FIXED = { producer: "lamina examples", creationDate: "D:20261030120000Z" } as const;

/** A runnable example. */
export interface Runnable {
  /** The function the file exports, called with the inputs. */
  readonly export: string;
  readonly inputs: readonly RunInput[];
}

const NOTO: FileInput = { file: "fixtures/fonts/NotoSans-Regular.ttf", as: "bytes" };
const PHOTO: FileInput = { file: "fixtures/images/photo-rgb.jpg", as: "bytes" };
const FLOW_REPORT: FileInput = { file: "fixtures/text/element-flow-report.json", as: "json" };

/** The runnable examples, by their TypeScript file from the repository root. */
export const RUNNABLE: Readonly<Record<string, Runnable>> = {
  "packages/sdk/examples/hello.ts": { export: "hello", inputs: [{ options: "lamina-ex-hello!" }] },
  "packages/sdk/examples/invoice.ts": { export: "invoice", inputs: [NOTO, { options: "lamina-ex-invoic" }] },
  "packages/sdk/examples/images-and-vectors.ts": {
    export: "imagesAndVectors",
    inputs: [PHOTO, { options: "lamina-ex-vector" }],
  },
  "packages/sdk/examples/links-and-outlines.ts": {
    export: "linksAndOutlines",
    inputs: [{ options: "lamina-ex-links!" }],
  },
  "packages/sdk/examples/element-flow.ts": {
    export: "elementFlow",
    inputs: [PHOTO, FLOW_REPORT, { options: "lamina-ex-flow!!" }],
  },
  "packages/sdk/examples/pdf-ua-report.ts": { export: "pdfUaReport", inputs: [NOTO] },
  // The quick start the first tutorial shows: run as it is, its `save` given the PDF.
  "packages/sdk/examples/quick-start/node.ts": {
    export: "hello",
    inputs: [{ save: true }, { options: "lamina-ex-hello!" }],
  },
};
