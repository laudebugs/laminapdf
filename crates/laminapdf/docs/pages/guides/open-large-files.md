---
title: "Open large files by ranges"
description: "Open a PDF on a web server without downloading all of it, fetching only the byte ranges each read needs."
version: "0.2.0"
url: "https://docs.laminapdf.com/guides/open-large-files/"
---

# Open large files by ranges

Open a PDF on a web server without downloading all of it, fetching only the byte ranges each read needs.

You'll open a PDF on a web server by fetching only the parts you read. A large report opens after a few small requests, and reading its first page fetches that page and little else. You need this when files are too large to download before the first page shows.

This example opens a remote file, then reads its page count and first page:

`packages/sdk/examples/open-large-file.ts`:

```ts
// Opens a PDF on a web server by range requests: only the parts it reads are downloaded, so a
// large file opens without fetching all of it. The server must answer `Range` requests, as most
// file hosts and CDNs do; across origins it must also allow the `Range` and `If-Range` headers.
// Needs `init()`.
import { Document, httpRangeSource } from "@laminapdf/sdk";

/** What a first look at a remote file needs: its page count and the first page's size. */
export interface FirstLook {
  readonly pages: number;
  /** The first page's displayed size, in points (1/72 inch). */
  readonly width: number;
  readonly height: number;
}

/** Opens the PDF at `url` range by range and reads its page count and first page. */
export async function firstLook(url: string): Promise<FirstLook> {
  // The first request reads the end of the file, where a PDF keeps the index of its objects.
  using doc = await Document.openRange(httpRangeSource(url));
  // A read can need bytes that haven't arrived. `ensure` catches the `data-pending` error, fetches
  // the ranges it names, and runs the read again until it has what it needs.
  const pages = await doc.ensure(() => doc.pageCount);
  const first = await doc.ensure(() => doc.pageInfo(0));
  return { pages, width: first.width, height: first.height };
}
```

## Each read fetches the bytes it needs

`Document.openRange` fetches only what opening needs: the end of the file, where a PDF keeps the index of its objects, then that index and the document's catalog. Calls on the document stay synchronous, so a read that needs bytes not fetched yet can't wait for them. It throws `data-pending` instead, with the byte ranges it needs. `doc.ensure` runs your read, fetches those ranges when it throws, and runs it again until it returns:

*A read that needs missing bytes fetches them and runs again.*

`Document.openRange` opens a remote file from the few byte ranges the open needs. A later read, such as `doc.pageInfo(0)`, may need bytes that haven't arrived: it then throws `data-pending`, naming the ranges it needs. `doc.ensure` catches the error, fetches those ranges through the source (`httpRangeSource`), supplies them to the document and runs the read again. Once the bytes are there, the read returns its result.

Wrap every read after the open in `doc.ensure`, saving included: `await doc.ensure(() => doc.save())` fetches whatever the save writes. A read fails with the last `data-pending` when its fetches bring nothing new, or when it still needs bytes after 1,024 rounds of fetches. Pass `maxRounds` to `Document.openRange` to change that number.

## The server must answer `Range` requests

`httpRangeSource(url)` asks for byte ranges with the `Range` header, and most file hosts and CDNs answer them. Across origins, the server must also allow the `Range` and `If-Range` request headers (`Access-Control-Allow-Headers: Range, If-Range`) and expose `Content-Range`, `Content-Length` and `ETag` (`Access-Control-Expose-Headers`). It must not compress the response, or the ranges it sends wouldn't match the file's bytes. An error status from the server fails with `fetch-failed`, its `data.status` holding the status code.

A server that ignores `Range` and answers `200` sends the whole file. Lamina PDF then opens that one download, as `Document.open` would.

## A file that changes on the server fails the read

Every request after the first sends `If-Range` with the `ETag` the first response gave, when it's a strong one, or else that response's `Last-Modified` date. When the file changes between two reads, the read fails with `source-changed` rather than mixing two versions. Open the file again to read the new one.

## Limit how much a file can make you fetch

A remote file decides how much you fetch, so cap it when the file comes from someone else. Pass `budget` to `Document.openRange`: it's the most bytes the document may hold, and past it a read fails with `source-budget-exceeded`. Pass `signal` with the same options to stop the open's fetches. Later reads take their own: `doc.ensure(read, { signal: AbortSignal.timeout(10_000) })`.

The [limits for files you don't trust](../../pages/guides/untrusted-files.md) apply here too: pass `limits` with the other options.
