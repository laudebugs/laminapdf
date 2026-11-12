// The docs' example runner (todo B7e): a module worker, served at /run/worker.js with its own
// headers (`'unsafe-eval'` for this file alone; every page keeps ADR 0011's). It loads
// @laminapdf/sdk (text module) and @laminapdf/render-hayro on first use, runs a reader's edit of an
// example (run.ts), and draws each page of the result to an ImageBitmap for the page to show:
// hayro, as for the Result tab's pictures (cargo xtask docs-previews), so both look the same.

import { bytesSource } from "@laminapdf/render";
import { initHayro } from "@laminapdf/render-hayro";
import * as sdk from "@laminapdf/sdk";
import notoSans from "../../../fixtures/fonts/NotoSans-Regular.ttf?url";
import photo from "../../../fixtures/images/photo-rgb.jpg?url";
import flowReport from "../../../fixtures/text/element-flow-report.json?url";
import { FIXED, RUNNABLE, type RunInput } from "./examples.js";
import type { RunReply, RunRequest } from "./protocol.js";
import { runExample, SAVE } from "./run.js";

/** The most pages a run draws: each is about 4 MB, and an edit can make thousands. */
const MAX_PAGES = 20;

type Engines = [unknown, Awaited<ReturnType<typeof initHayro>>];
let ready: Promise<Engines> | undefined;

/** The SDK and hayro, loaded on first use; a failed load is tried again on the next run. */
function engines(): Promise<Engines> {
  ready ??= Promise.all([sdk.init({ text: true }), initHayro()]).catch((e: unknown) => {
    ready = undefined;
    throw e;
  });
  return ready;
}
/** Where each input file is served: built into /run/assets/ beside the worker. */
const INPUT_URLS: Readonly<Record<string, string>> = {
  "fixtures/fonts/NotoSans-Regular.ttf": notoSans,
  "fixtures/images/photo-rgb.jpg": photo,
  "fixtures/text/element-flow-report.json": flowReport,
};
const inputs = new Map<string, Promise<unknown>>();

/** Input `file`, fetched once, as bytes or parsed JSON. */
function input(file: string, as: "bytes" | "json"): Promise<unknown> {
  let got = inputs.get(file);
  if (got === undefined) {
    const url = INPUT_URLS[file];
    if (url === undefined) return Promise.reject(new Error(`${file}: not an input the runner serves`));
    got = fetch(url)
      .then(async (r) => {
        if (!r.ok) throw new Error(`${file}: ${r.status}`);
        return as === "json" ? r.json() : new Uint8Array(await r.arrayBuffer());
      })
      .catch((e: unknown) => {
        // Not cached: the next run fetches it again.
        inputs.delete(file);
        throw e;
      });
    inputs.set(file, got);
  }
  return got;
}

/** An input as the example takes it. */
function argument(i: RunInput): unknown {
  if ("save" in i) return SAVE;
  return "options" in i ? { id: new TextEncoder().encode(i.options), ...FIXED } : input(i.file, i.as);
}

async function handle(request: RunRequest): Promise<RunReply> {
  const runnable = RUNNABLE[request.file];
  if (runnable === undefined)
    return { id: request.id, ok: false, code: "error", message: `${request.file} can't run here` };
  const [, hayro] = await engines();
  const args = await Promise.all(runnable.inputs.map(argument));
  const outcome = await runExample(request.source, runnable.export, args, sdk);
  if (!outcome.ok) return { id: request.id, ...outcome };
  using doc = hayro.open(bytesSource(outcome.bytes));
  const pages: ImageBitmap[] = [];
  for (let index = 0; index < Math.min(doc.pageCount, MAX_PAGES); index++) {
    const page = doc.page(index);
    const { width, height, pixels } = await doc.render({ page: index, scale: request.width / page.width });
    pages.push(await createImageBitmap(new ImageData(pixels, width, height)));
  }
  return { id: request.id, ok: true, bytes: outcome.bytes, pages, pageCount: doc.pageCount };
}

/** The worker's own scope: the docs' TypeScript knows only the DOM's `self`. */
const scope = self as unknown as {
  addEventListener(type: "message", listener: (event: MessageEvent<RunRequest>) => void): void;
  postMessage(message: RunReply, transfer: Transferable[]): void;
};

/** Sends `reply`, or its failure to send (an untransferable buffer, say) as an error. */
function send(reply: RunReply): void {
  try {
    scope.postMessage(reply, reply.ok ? [...reply.pages, reply.bytes.buffer as ArrayBuffer] : []);
  } catch (e) {
    for (const page of reply.ok ? reply.pages : []) page.close();
    scope.postMessage(failure(reply.id, e), []);
  }
}

/** A thrown value as an error reply. */
function failure(id: number, e: unknown): RunReply {
  return { id, ok: false, code: "error", message: e instanceof Error ? e.message : String(e) };
}

scope.addEventListener("message", (event) => {
  handle(event.data).then(send, (e: unknown) => send(failure(event.data.id, e)));
});
