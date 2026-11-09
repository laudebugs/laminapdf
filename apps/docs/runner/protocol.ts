// The messages between the page (editor.ts) and the runner (worker.ts). Their own module, with no
// package imports, so the site's typecheck reads them without the built SDK and hayro the worker
// needs; scripts/build-runner.mjs typechecks the worker itself (runner/tsconfig.json).

/** A request: run `source`, the reader's edit of `file`, and draw its pages `width` px wide. */
export interface RunRequest {
  readonly id: number;
  readonly file: string;
  readonly source: string;
  readonly width: number;
}

/** The answer: the PDF and its pages, or the error. */
export type RunReply =
  | {
      readonly id: number;
      readonly ok: true;
      readonly bytes: Uint8Array;
      /** The first pages' pictures (the worker draws at most 20). */
      readonly pages: ImageBitmap[];
      /** How many pages the PDF has. */
      readonly pageCount: number;
    }
  | {
      readonly id: number;
      readonly ok: false;
      readonly code: string;
      readonly message: string;
      readonly hint?: string;
    };
