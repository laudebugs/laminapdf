// Turn an error's code into what your app does next: ask for a password, ask again, or tell the
// person the file isn't a PDF. Any other code throws, so a code your app doesn't know yet isn't
// swallowed.
import { Document, isLaminaPdfError } from "@laminapdf/sdk";

/** What the app does after trying to open a file. */
export type NextStep =
  | { readonly step: "show"; readonly pages: number; readonly warnings: readonly string[] }
  | { readonly step: "ask-for-password" }
  | { readonly step: "ask-again" }
  | { readonly step: "not-a-pdf" };

/** Opens `bytes`, with `password` if one was given, and says what to do next. Needs `init()`. */
export function openOrAsk(bytes: Uint8Array, password?: string): NextStep {
  try {
    using doc = Document.open(bytes, password === undefined ? {} : { password });
    // Warnings mean the file opened but needed repairs; their codes say which.
    return { step: "show", pages: doc.pageCount, warnings: doc.warnings.map((w) => w.code) };
  } catch (e) {
    if (isLaminaPdfError(e, "password-required")) return { step: "ask-for-password" };
    if (isLaminaPdfError(e, "wrong-password")) return { step: "ask-again" };
    if (isLaminaPdfError(e, "not-pdf")) return { step: "not-a-pdf" };
    throw e;
  }
}
