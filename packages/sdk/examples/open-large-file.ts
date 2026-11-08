// Opens a PDF on a web server by range requests: only the parts it reads are downloaded, so a
// large file opens without fetching all of it. The server must answer `Range` requests, as most
// file hosts and CDNs do; across origins it must also allow the `Range` header. Needs `init()`.
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
