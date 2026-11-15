---
title: "Open someone else's file"
description: "Open an uploaded PDF with an optional password and tighter limits, and get its page count and warnings, or the error, back as a value."
version: "0.4.0"
url: "https://docs.laminapdf.com/recipes/open-untrusted/"
---

# Open someone else's file

Open an uploaded PDF with an optional password and tighter limits, and get its page count and warnings, or the error, back as a value.

You'll open a PDF from someone else, with a password if it needs one and tighter limits than the defaults, and get the outcome as a value. You need this wherever you accept uploads: a stranger's file can be damaged, locked or built to use up memory.

The function opens the bytes under its limits and returns the page count and warning codes, or the error's code and hint:

`packages/sdk/examples/open-untrusted.ts`:

```ts
// Open bytes from someone else: an optional password, tighter limits than the defaults, and the
// outcome as a value rather than an exception. The example writes nothing: it returns what it found.
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
```

It writes no PDF. The value it returns is the result: `{ ok: true, pages, warnings }` for a file that opened, or `{ ok: false, code, hint }`, such as `password-required` for a locked file opened without its password.

## How it works

- It needs only `init()`: opening a file uses the base module, without fonts or text.
- `Document.open` gets the password only when you pass one, and tries it against both of the file's passwords. See [Open password-protected files](../../pages/guides/password-protected-files.md).
- `LIMITS` allows 16 MiB decoded from one stream and 64 MiB in all, against defaults of 64 MiB and 1 GiB. A file that needs more fails with `limit-exceeded`, and every other limit keeps its default.
- A file can open and still carry warnings: each one names a repair or a dropped value, and the function returns their codes.
- `isLaminaPdfError(e)` picks out Lamina PDF's coded errors, whose message and hint never contain text from the file. Any other error is a bug, so the function throws it again.
- `using` frees the document as soon as the function returns, so the WebAssembly side keeps no copy of the file.

## What to change

- **Ask for the password again:** on `password-required` or `wrong-password`, ask your user and call `openUntrusted` again with their answer.
- **Name the limit:** on `limit-exceeded`, return `e.data?.limit` too. It names the limit the file went over as Rust spells it, such as `max_stream_decoded` for `maxStreamDecoded`.
- **Other limits:** add any limit from the table in [Open files you don't trust](../../pages/guides/untrusted-files.md#hard-and-soft-limits), such as a lower `maxNestingDepth`.
- **Refuse damaged files:** add `strict: true` to the options, and a file that needs repair fails with `strict` instead of opening with warnings.

Next: [save in a Web Worker](../../pages/recipes/web-worker-client.md), so a large file never freezes the page.
