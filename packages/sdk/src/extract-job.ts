// doc.extractJob(): an extraction stepped a page at a time, whose layout model or OCR engine the
// host runs (ADR 0030 §6). Each step that needs a model's result stops with a `request` naming
// the page and resolution; the host renders that page, runs its model and hands the result to
// `provide`. `extractWithModels` loops a job for hosts whose models are async.
import type {
  ModelInfoInit,
  ModelRequestJs,
  ModelsInit,
  RegionInit,
  RegionLabelName,
  WasmExtractJob,
  WordInit,
} from "@laminapdf/wasm/extract";
import type { Document } from "./document.js";
import { packageError } from "./errors.js";
import type { ExtractFormat, ExtractOptions, ExtractResult } from "./extract.js";
import { parseExtract } from "./extract.js";
import type { BaseGlue } from "./init.js";
import { Handle } from "./internal/handle.js";
import type { Instance } from "./internal/instance.js";

// The model types come from the extract module's own declarations (tsify, from
// crates/laminapdf-wasm/src/extract.rs), so they can't drift from what the binding accepts.
// Named again here so each carries its own docs and `@since`.

/**
 * A layout region a model found, its box `[x, y, width, height]` in top-left points, as the
 * output's boxes are.
 *
 * @since 0.2.0
 */
export type LayoutRegion = RegionInit;

/**
 * A model the host runs, as the output's `generator.models` names it.
 *
 * @since 0.2.0
 */
export type ModelInfo = ModelInfoInit;

/**
 * A model result the job waits for: which model, which page, at what resolution.
 *
 * @since 0.2.0
 */
export type ModelRequest = ModelRequestJs;

/**
 * The models the host runs for an extraction job.
 *
 * @since 0.2.0
 */
export type Models = ModelsInit;

/**
 * A word an OCR engine read, its box as a region's.
 *
 * @since 0.2.0
 */
export type OcrWord = WordInit;

/**
 * A layout region's kind.
 *
 * @since 0.2.0
 */
export type RegionLabel = RegionLabelName;

/**
 * Options for {@link Document.extractJob}: {@link ExtractOptions} and the models the host runs.
 *
 * @since 0.2.0
 */
export type ExtractJobOptions<F extends ExtractFormat = ExtractFormat> = ExtractOptions<F> & {
  /**
   * The models the host runs, each named as the output's `generator.models` records it: a
   * `layout` model is asked for every page's regions, an `ocr` engine for the pages with no
   * visible text. `dpi` (150 by default) is the resolution pages are asked for at.
   */
  readonly models?: Models;
};

/**
 * What {@link ExtractJob.step} returns.
 *
 * @since 0.2.0
 */
export type ExtractStep<F extends ExtractFormat> =
  | {
      readonly state: "progress";
      /** Pages done. */
      readonly done: number;
      /** Pages asked for. */
      readonly total?: number;
      /** `"extract"`, or `"model"` while {@link request} is set. */
      readonly phase: string;
      /** The model result the job waits for: answer it with {@link ExtractJob.provide}. */
      readonly request?: ModelRequest;
    }
  | { readonly state: "done"; readonly output: ExtractResult[F] };

/**
 * A model's result for {@link ExtractJob.provide}, boxes as `[x, y, width, height]` in top-left points.
 *
 * @since 0.2.0
 */
export type ModelResult =
  | { readonly kind: "layout"; readonly regions: readonly LayoutRegion[] }
  | { readonly kind: "ocr"; readonly words: readonly OcrWord[] };

/**
 * An extraction in progress, from {@link Document.extractJob}. It holds WebAssembly memory and is
 * freed with its document, or earlier with `free()` or `using`.
 *
 * @since 0.2.0
 */
export class ExtractJob<F extends ExtractFormat = ExtractFormat> extends Handle<BaseGlue> {
  readonly #job: WasmExtractJob;
  readonly #format: F;

  /** @internal */
  constructor(job: WasmExtractJob, format: F, doc: Document, instance: Instance<BaseGlue>) {
    super("ExtractJob", instance, doc);
    this.#job = job;
    this.#format = format;
  }

  /**
   * Extracts up to `budget` pages. While a model's result is awaited, the step returns
   * `{ state: "progress", phase: "model", request }` and does nothing more until
   * {@link provide} answers it.
   *
   * @returns progress, or `{ state: "done", output }` with the output in the job's format.
   * @throws {@link LaminaPdfError} `cancelled`, `document-changed`, `job-finished`, the reading errors.
   *
   * @since 0.2.0
   */
  step(budget = 1): ExtractStep<F> {
    const out = this.call(() => this.#job.step(budget)) as
      | ({ state: "progress" } & Omit<Extract<ExtractStep<F>, { state: "progress" }>, "state">)
      | { state: "done"; output: string };
    return out.state === "done" ? { state: "done", output: parseExtract(this.#format, out.output) } : out;
  }

  /**
   * Hands the job the result of its open request.
   *
   * @throws {@link LaminaPdfError} `model-result-unexpected` when nothing was asked for or `result`
   *   is the other model's; `invalid-argument` / `unknown-option` for a malformed result.
   *
   * @since 0.2.0
   */
  provide(result: ModelResult): void {
    this.call(() => this.#job.provide(result as Parameters<WasmExtractJob["provide"]>[0]));
  }

  /**
   * Stops the job: its next step fails with `cancelled`.
   *
   * @since 0.2.0
   */
  cancel(): void {
    this.call(() => this.#job.cancel());
  }

  protected release(): void {
    this.#job.free();
  }
}

/**
 * The host's models for {@link extractWithModels}: each renders the page it is asked for and runs.
 *
 * @since 0.2.0
 */
export interface HostModels {
  /** A layout model's regions on `request.page`, rendered at `request.dpi`. */
  readonly layout?: (request: ModelRequest, signal: AbortSignal | undefined) => Promise<readonly LayoutRegion[]>;
  /** An OCR engine's words on `request.page`, rendered at `request.dpi`, in `request.langs`. */
  readonly ocr?: (request: ModelRequest, signal: AbortSignal | undefined) => Promise<readonly OcrWord[]>;
}

/** The host's answer to one request. */
async function answer(
  request: ModelRequest,
  models: HostModels,
  signal: AbortSignal | undefined,
): Promise<ModelResult> {
  if (request.kind === "layout" && models.layout !== undefined) {
    return { kind: "layout", regions: await models.layout(request, signal) };
  }
  if (request.kind === "ocr" && models.ocr !== undefined) {
    return { kind: "ocr", words: await models.ocr(request, signal) };
  }
  throw packageError("invalid-argument", `no function in models answers a ${request.kind} request`);
}

/**
 * Runs an extraction whose models the host runs asynchronously: steps a job, and for each
 * request awaits the matching function of `models`, then hands its result back.
 *
 * @example
 * ```ts
 * const json = await extractWithModels(
 *   doc,
 *   { format: "json", models: { layout: { name: "my-layout", version: "1", sha256, licence: "Apache-2.0" } } },
 *   { layout: async ({ page, dpi }) => myModel.detect(await render(page, dpi)) },
 * );
 * ```
 * On a document opened with `Document.openRange`, a step that needs bytes not yet fetched ends
 * the job with `data-pending`, as every error ends a job; fetch the file first (`await
 * doc.ensure(() => doc.pages)` reads only the page tree, so read the whole file for a long
 * extraction), or wrap the call in `doc.ensure`, which runs it again, models included.
 *
 * @throws {@link LaminaPdfError} as {@link ExtractJob.step}; `invalid-argument` when
 *   `options.models` names a model `models` has no function for; `cancelled` once `signal`
 *   aborts, also when the model rejects because of it (the model's error is the `cause`).
 *
 * @since 0.2.0
 */
export async function extractWithModels<F extends ExtractFormat>(
  doc: Document,
  options: ExtractJobOptions<F>,
  models: HostModels,
  { signal, budget = 16 }: { readonly signal?: AbortSignal; readonly budget?: number } = {},
): Promise<ExtractResult[F]> {
  for (const kind of ["layout", "ocr"] as const) {
    if (options.models?.[kind] !== undefined && models[kind] === undefined) {
      throw packageError(
        "invalid-argument",
        `options.models names a ${kind} model, but models has no ${kind} function`,
      );
    }
  }
  using job = doc.extractJob(options);
  // The job only moves between awaits, so the signal is read there: no listener that could
  // outlive the job or its document.
  for (;;) {
    if (signal?.aborted) throw cancelled();
    const step = job.step(budget);
    if (step.state === "done") return step.output;
    if (step.request !== undefined) job.provide(await answerUnlessAborted(step.request, models, signal));
  }
}

function cancelled(cause?: unknown): Error {
  return packageError("cancelled", "the signal aborted the extraction", cause === undefined ? {} : { cause });
}

/** The host's answer, or `cancelled` when the signal aborted before or while the model ran. */
async function answerUnlessAborted(
  request: ModelRequest,
  models: HostModels,
  signal: AbortSignal | undefined,
): Promise<ModelResult> {
  let result: ModelResult;
  try {
    result = await answer(request, models, signal);
  } catch (e) {
    // A model that honours the signal rejects with its own AbortError.
    if (signal?.aborted) throw cancelled(e);
    throw e;
  }
  if (signal?.aborted) throw cancelled();
  return result;
}
