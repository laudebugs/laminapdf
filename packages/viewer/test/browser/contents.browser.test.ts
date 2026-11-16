// A contents page through the document worker: the listing a dialog previews, the page inserted as
// one step in the reader's font, undo, and a restarted worker inserting the same page again.

import type { ViewerClient } from "@laminapdf/viewer";
import { afterEach, describe, expect, it } from "vitest";
// The reader's font for the PDFs it builds (helpers.ts's `read` serves only paths outside packages/).
import fontUrl from "../../../fonts/pdf/lamina-sans-regular.ttf?url";
import { hook, next, read, testViewer } from "./helpers.js";

/** Four pages labelled i, ii, 1, 2, with bookmarks to each (and a web address and a heading with none). */
const BOOKMARKED = "fixtures/golden/links-and-outlines.pdf";

async function font(): Promise<Uint8Array> {
  return new Uint8Array(await (await fetch(fontUrl)).arrayBuffer());
}

let rpc: ViewerClient | undefined;

afterEach(() => {
  rpc?.terminate();
  rpc = undefined;
});

async function opened(): Promise<ViewerClient> {
  rpc = await testViewer("pdfium");
  await rpc.open(await read(BOOKMARKED));
  return rpc;
}

describe("a contents page", () => {
  it("lists the bookmarks that go to a page, each with the label its page will show", async () => {
    const client = await opened();
    const listing = await client.contentsListing({});
    expect(listing.entries.map((e) => [e.level, e.title, e.labels[0]])).toEqual([
      // The contents page joins the front matter as "i", so its pages move up one.
      [1, "Title page", "ii"],
      [1, "Contents", "iii"],
      [2, "Chapter 1", "1"],
      [2, "Chapter 2", "2"],
    ]);
  });

  it("says how many pages the page takes when given its font", async () => {
    const client = await opened();
    const [plain, laid] = [await client.contentsListing({}), await client.contentsListing({}, await font())];
    expect([plain.pages, laid.pages, laid.entries.length]).toEqual([1, 1, 4]);
  });

  it("inserts the page as one step, which undo takes back", async () => {
    const client = await opened();
    const first = client.document?.pages[0];
    const result = await client.createContentsPage({ bookmark: true }, await font());
    expect(result.order?.length).toBe(5);
    expect(result.history.steps.map((s) => s.label)).toEqual(["Create a contents page"]);
    expect(client.document?.pageCount).toBe(5);
    const page = await client.render({ page: 0, scale: 1 });
    // The contents page takes the first page's size.
    expect([page.width, page.height]).toEqual([Math.floor(first?.width ?? 0), Math.floor(first?.height ?? 0)]);
    page.bitmap.close();
    await client.undoEdit();
    expect(client.document?.pageCount).toBe(4);
  });

  it("inserts the same page again after the worker restarts", async () => {
    const client = await opened();
    await client.createContentsPage({}, await font());
    const restarted = next(client, "restarted");
    void hook(client, "__crash").catch(() => undefined);
    expect(await restarted).toMatchObject({ reopened: true, edits: "restored" });
    expect(client.document?.pageCount).toBe(5);
    expect((await client.editHistory()).steps.map((s) => s.label)).toEqual(["Create a contents page"]);
  });
});
