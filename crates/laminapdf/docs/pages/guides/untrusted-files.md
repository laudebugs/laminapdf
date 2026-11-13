---
title: "Open files you don't trust"
description: "Cap how much work an uploaded PDF can make Lamina PDF do, and refuse damaged files with strict mode."
version: "0.2.0"
url: "https://docs.laminapdf.com/guides/untrusted-files/"
---

# Open files you don't trust

Cap how much work an uploaded PDF can make Lamina PDF do, and refuse damaged files with strict mode.

You'll open uploads with caps on how much an untrusted file can make Lamina PDF decode, read or draw. You need this on any server or page that opens PDFs from strangers, because a small file can ask for gigabytes of work.

This example opens uploads with tighter limits than the defaults: at most 16 MiB decoded from one stream and 64 MiB in all:

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

## Set only the limits you need

That work takes many forms: a stream that inflates to gigabytes, a page tree thousands of levels deep, drawings nested inside drawings. Lamina PDF keeps every file within the `limits` you pass when you open it. Every limit has a safe default, so you set only the ones you want to change.

## Hard and soft limits

- **Hard limits** stop the call with `limit-exceeded`. Its `data.limit` names the limit, so you can tell the user which one.
- **Soft limits** drop the excess with a warning and carry on.

| Limit | Kind | Default | What it caps |
| --- | --- | --- | --- |
| `maxNestingDepth` | hard | 256 | arrays and dictionaries inside each other; saved graphics states (`q`) |
| `maxStreamDecoded` | hard | 64 MiB | bytes one stream decodes to, per filter stage |
| `maxDecodedTotal` | hard | 1 GiB | bytes one call (`open`, one page's content, one read) decodes, across all streams |
| `maxFilterChain` | hard | 8 | filters on one stream |
| `maxResolveDepth` | hard | 32 | indirect objects in flight while following a chain of references |
| `maxPageTreeDepth` | hard | 256 | `/Kids` levels below the page-tree root (a deeper subtree is skipped with a warning) |
| `maxTreeDepth` | hard | 64 | name and number tree levels, outline depth |
| `maxStringLength` | hard | 16 MiB | bytes in one string or name |
| `maxXrefSections` | soft | 4,096 | `/Prev` hops in the [cross-reference table](../../pages/reference/glossary.md#cross-reference-table) chain (past it, Lamina PDF rebuilds the table by scanning) |
| `maxObjectNumber` | soft | 8,388,607 | the highest object number kept |
| `maxWarnings` | soft | 1,000 | warnings kept per document (then one `warnings-truncated`) |
| `maxContentOperators` | hard | 10,000,000 | drawing operators run in one content call, counting a form's each time it is drawn |
| `maxXobjectDepth` | hard | 32 | forms drawn inside forms |
| `maxImagePixels` | hard (reserved) | 2²⁸ | pixels of one image |
| `maxReadWindow` | hard | 256 MiB | one read from a range source (`Document.openRange`) |

These are the TypeScript names. In Rust they're builder methods, such as `Limits::new().with_max_stream_decoded(…)`.

## Refuse damaged files

Lamina PDF repairs damaged files as it opens them and reports each repair as a warning. Pass `strict: true` to refuse any file that needs repair instead. It then fails with `strict`.

A file on someone else's server can also decide how much you download. [Open it by ranges](../../pages/guides/open-large-files.md) and cap the bytes it may fetch with `budget`.

When a file you trust goes over a limit, [Troubleshooting opening and saving files](../../pages/troubleshooting/files.md#opening-or-reading-a-file-fails-with-limit-exceeded) says how to raise it.

Next: [keep the page responsive](../../pages/guides/keep-the-page-responsive.md).
