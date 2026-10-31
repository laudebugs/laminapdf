// Open bytes from someone else: a password, tighter limits than the defaults, and the outcome as
// a value rather than an exception (the docs site's encrypted files, limits and errors guides).
// It has no Rust golden: it writes nothing; test/examples.test.ts checks what it reports.
import { Document, isLaminaError } from "@laminapdf/pdf";

/** What opening a file gave: its page count and warning codes, or why it didn't open. */
export type Opened =
  | { readonly ok: true; readonly pages: number; readonly warnings: readonly string[] }
  | { readonly ok: false; readonly code: string; readonly hint: string | undefined };

/**
 * Opens `bytes` with `password` (empty: none) and limits for an upload endpoint: decoded streams
 * of at most 16 MiB each and 64 MiB in all. Needs `init()`.
 */
export function openUntrusted(bytes: Uint8Array, password = ""): Opened {
  try {
    const doc = Document.open(bytes, {
      ...(password === "" ? {} : { password }),
      limits: { maxStreamDecoded: 16 << 20, maxDecodedTotal: 64 << 20 },
    });
    try {
      // Repairs and dropped values are warnings: the file opened, but not exactly as written.
      return { ok: true, pages: doc.pageCount, warnings: doc.warnings.map((w) => w.code) };
    } finally {
      doc.free();
    }
  } catch (e) {
    // `password-required`, `wrong-password`, `not-pdf`, `limit-exceeded`, …: a coded LaminaError
    // whose message and hint never contain text from the file (ADR 0026 §4).
    if (isLaminaError(e)) return { ok: false, code: e.code, hint: e.hint };
    throw e;
  }
}
