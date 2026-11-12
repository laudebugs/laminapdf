// Runs a reader's edit of an example: strips its types with Sucrase (no type check: a mistake
// surfaces as the error the SDK throws), turns its `import`s into a `require` that knows only
// `@laminapdf/sdk` and `node:fs/promises`' `writeFile`, and calls the function it exports with the
// example's inputs. Pure over its arguments, so Node tests run it with a fake SDK; the worker
// (worker.ts) gives it the real one.

import { transform } from "sucrase";

/** What a run gives back: the PDF's bytes, or the error as a reader should see it. */
export type RunOutcome =
  | { readonly ok: true; readonly bytes: Uint8Array }
  | { readonly ok: false; readonly code: string; readonly message: string; readonly hint?: string };

/**
 * The input that stands for a quick start's `save`: runExample passes a function in its place that
 * keeps the bytes it is given as the run's PDF.
 */
export const SAVE: unique symbol = Symbol("save");

/** The body of an async function, so a quick start's top-level `await` runs as it does in a module. */
const AsyncFunction = Object.getPrototypeOf(async () => {}).constructor as new (
  ...args: string[]
) => (...args: unknown[]) => Promise<void>;

/**
 * Runs `source`, an example's TypeScript, against `sdk`: calls its export `name` with `args`.
 *
 * @param source the reader's code
 * @param name the function the example exports
 * @param args the example's inputs, in order; `SAVE` for the function it hands its PDF to
 * @param sdk the `@laminapdf/sdk` module, the only package the code may import
 */
export async function runExample(
  source: string,
  name: string,
  args: readonly unknown[],
  sdk: object,
): Promise<RunOutcome> {
  let code: string;
  try {
    code = transform(source, { transforms: ["typescript", "imports"] }).code;
  } catch (e) {
    return { ok: false, code: "syntax", message: e instanceof Error ? e.message : String(e) };
  }
  // The PDF a quick start hands to `save`, or writes with `writeFile`: there are no files here, so
  // the run shows what it would have written.
  let saved: unknown;
  const keep = (pdf: unknown): void => {
    saved = pdf;
  };
  const writeFile = async (_path: unknown, data: unknown) => keep(data);
  // Any other member of node:fs/promises fails as an import of another module would.
  const fs = new Proxy(
    {},
    {
      get: (_target, member) => {
        if (member === "writeFile") return writeFile;
        if (typeof member !== "string" || member === "__esModule" || member === "then") return undefined;
        throw refused(`${member} from node:fs/promises`);
      },
    },
  );
  const exports: Record<string, unknown> = {};
  const require = (specifier: string): object => {
    if (specifier === "@laminapdf/sdk") return sdk;
    if (specifier === "node:fs/promises") return fs;
    throw refused(specifier);
  };
  try {
    // Sucrase keeps `import.meta`, which only a module may name: here it is an object whose `main`
    // is false, as for a module that was imported, so a quick start only defines its function.
    const body = code.replace(/\bimport\.meta\b/g, "__importMeta");
    // The reader's own code, in the worker the docs' headers allow eval for (and nowhere else).
    await new AsyncFunction("exports", "require", "__importMeta", body)(exports, require, { main: false });
    const run = exports[name];
    if (typeof run !== "function") {
      return { ok: false, code: "missing-export", message: `the example no longer exports ${name}` };
    }
    const returned: unknown = await run(...args.map((arg) => (arg === SAVE ? keep : arg)));
    const bytes = returned instanceof Uint8Array ? returned : saved;
    if (!(bytes instanceof Uint8Array)) {
      return { ok: false, code: "not-a-pdf", message: `${name} made no PDF bytes` };
    }
    return { ok: true, bytes };
  } catch (e) {
    return describe(e);
  }
}

/** The error for an import the runner lacks. */
function refused(what: string): Error {
  return new Error(`the example may import only @laminapdf/sdk and writeFile from node:fs/promises, not ${what}`);
}

/** A thrown value as a reader should see it: a `LaminaPdfError`'s code and hint, or the message. */
function describe(e: unknown): RunOutcome {
  if (typeof e === "object" && e !== null && "code" in e && typeof e.code === "string") {
    const hint = "hint" in e && typeof e.hint === "string" ? e.hint : undefined;
    const message = "message" in e && typeof e.message === "string" ? e.message : e.code;
    return hint === undefined ? { ok: false, code: e.code, message } : { ok: false, code: e.code, message, hint };
  }
  return { ok: false, code: "error", message: e instanceof Error ? e.message : String(e) };
}
