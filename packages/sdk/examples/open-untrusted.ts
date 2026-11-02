// Open bytes from someone else: an optional password, tighter limits than the defaults, and the
// outcome as a value rather than an exception. It writes nothing; it reports what it found.
import { Document, isLaminaPdfError } from "@laminapdf/sdk";

const MiB = 1024 * 1024;
/** Limits for an upload endpoint: each decoded stream, and all of them together. */
const LIMITS = { maxStreamDecoded: 16 * MiB, maxDecodedTotal: 64 * MiB } as const;

/** What opening a file gave: its page count and warning codes, or why it didn't open. */
export type Opened =
  | { readonly ok: true; readonly pages: number; readonly warnings: readonly string[] }
  | { readonly ok: false; readonly code: string; readonly hint: string | undefined };

/** Opens `bytes`, with `password` if the file needs one, under {@link LIMITS}. Needs `init()`. */
export function openUntrusted(bytes: Uint8Array, password?: string): Opened {
  try {
    using doc = Document.open(bytes, { ...(password === undefined ? {} : { password }), limits: LIMITS });
    // Repairs and dropped values are warnings: the file opened, but not exactly as written.
    return { ok: true, pages: doc.pageCount, warnings: doc.warnings.map((w) => w.code) };
  } catch (e) {
    // `password-required`, `wrong-password`, `not-pdf`, `limit-exceeded`, …: a coded error whose
    // message and hint never contain text from the file. Anything else is a bug: let it throw.
    if (isLaminaPdfError(e)) return { ok: false, code: e.code, hint: e.hint };
    throw e;
  }
}
