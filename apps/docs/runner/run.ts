// Runs a reader's edit of an example: strips its types with Sucrase (no type check: a mistake
// surfaces as the error the SDK throws), turns its `import`s into a `require` that knows only
// `@laminapdf/sdk`, and calls the function it exports with the example's inputs. Pure over its
// arguments, so Node tests run it with a fake SDK; the worker (worker.ts) gives it the real one.

import { transform } from "sucrase";

/** What a run gives back: the PDF's bytes, or the error as a reader should see it. */
export type RunOutcome =
  | { readonly ok: true; readonly bytes: Uint8Array }
  | { readonly ok: false; readonly code: string; readonly message: string; readonly hint?: string };

/**
 * Runs `source`, an example's TypeScript, against `sdk`: calls its export `name` with `args`.
 *
 * @param source the reader's code
 * @param name the function the example exports
 * @param args the example's inputs, in order
 * @param sdk the `@laminapdf/sdk` module, the only import the code may make
 */
export function runExample(source: string, name: string, args: readonly unknown[], sdk: object): RunOutcome {
  let code: string;
  try {
    code = transform(source, { transforms: ["typescript", "imports"] }).code;
  } catch (e) {
    return { ok: false, code: "syntax", message: e instanceof Error ? e.message : String(e) };
  }
  const exports: Record<string, unknown> = {};
  const require = (specifier: string): object => {
    if (specifier === "@laminapdf/sdk") return sdk;
    throw new Error(`the example may import only @laminapdf/sdk, not ${specifier}`);
  };
  try {
    // The reader's own code, in the worker the docs' headers allow eval for (and nowhere else).
    new Function("exports", "require", code)(exports, require);
    const run = exports[name];
    if (typeof run !== "function") {
      return { ok: false, code: "missing-export", message: `the example no longer exports ${name}` };
    }
    const bytes: unknown = run(...args);
    if (!(bytes instanceof Uint8Array)) {
      return { ok: false, code: "not-a-pdf", message: `${name} returned no PDF bytes` };
    }
    return { ok: true, bytes };
  } catch (e) {
    return describe(e);
  }
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
