---
title: "Errors and warnings"
description: "One coded error shape on every surface, and warnings for what Lamina PDF repaired."
version: "0.2.0-alpha.4"
url: "https://docs.laminapdf.com/concepts/errors-and-warnings/"
---

# Errors and warnings

One coded error shape on every surface, and warnings for what Lamina PDF repaired.

Real PDFs are often damaged, so Lamina PDF separates a call that failed from one that worked around a problem. An error stops the call. A warning lets it finish and tells you what was repaired, dropped or approximated. Both carry a stable code your program can branch on.

## Errors stop the call

In TypeScript an error is a `LaminaPdfError`, and in Rust it's a `laminapdf::Error`. Branch on its `code`, never on its `message`: `isLaminaPdfError(e, "wrong-password")` checks the code and narrows the type in one step.

## Warnings say what was repaired

A warning means the call went ahead, but Lamina PDF had to repair, drop or approximate something in the file. Read them with `doc.warnings` in TypeScript or `doc.warnings()` in Rust. Show them to the user when the difference matters, such as a page that may not look the way its author intended.

## Every surface has the same fields

Errors and warnings have the same fields in TypeScript and Rust: `code`, `kind`, `message`, `hint`, `location`, `data` and `docsUrl`. A usage error can also carry `didYouMean`, the closest names to one Lamina PDF didn't recognise. `toJSON()` in TypeScript gives the same object as `serde` does in Rust, so a server and a browser can pass them to each other unchanged.

*A call stops with an error or completes with warnings, and both carry the same core fields.*

A call either stops with a `LaminaPdfError` or completes, with any warnings on `doc.warnings`. Errors and warnings carry the same fields: `code`, `kind`, `message`, `hint`, `location`, `data` and `docsUrl`; an error can also carry `didYouMean`. Text from the document goes only in `data`, never in `message` or `hint`.

- **`kind` sorts errors by who can fix them**, such as `input`, `limit`, `usage`, `state` or `io`. For example, a server returns a 4xx status for `input` and `limit` and a 5xx for the rest, as the [Cloudflare Workers quick start](../../pages/quick-start/workers.md) does.
- **Text from the document is untrusted**, so it never appears in `message` or `hint`. It goes only in `data`, where you can escape it before you show it.

This example opens someone else's file and returns the error's code and hint as a value, instead of letting the error escape:

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

The [error and warning codes](../../errors/README.md) list every code with its kind, a summary and a hint.

Next: make a PDF with [text and fonts](../../pages/guides/fonts-and-text.md).
