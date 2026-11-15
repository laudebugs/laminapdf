// Pages across files in the document worker (E2m): a file previewed before its pages come in
// (pages, an encrypted file's password, its thumbnails), made a source of the open document and
// inserted from; parts written as files or as one zip; files merged into a new document; and a
// restarted worker that adds the source again before it replays the insert. A step from an
// encrypted source can't come back: the page keeps no password.

import { Builder, Document, init, isLaminaPdfError, type Op } from "@laminapdf/sdk";
import { closeSource, mergeFiles, previewSource, renderSource, type ViewerClient, writeParts } from "@laminapdf/viewer";
import { afterEach, beforeAll, describe, expect, it } from "vitest";
import { ENCRYPTED, hook, next, PASSWORD, read, rejection, testViewer } from "./helpers.js";

/** `count` letter pages, the first `width` points wide; each file a new one (its own ID). */
function pages(count: number, width = 612): Uint8Array {
  using b = new Builder({ producer: "lamina tests" });
  for (let i = 0; i < count; i++) b.page(i === 0 ? [width, 792] : [612, 792]).finish();
  using doc = b.finish();
  return doc.save();
}

let rpc: ViewerClient | undefined;

beforeAll(async () => {
  await init({ text: true });
});

afterEach(() => {
  rpc?.terminate();
  rpc = undefined;
});

async function opened(bytes = pages(3)): Promise<ViewerClient> {
  rpc = await testViewer("pdfium");
  await rpc.open(bytes);
  return rpc;
}

/** The ZIP's entries' names, from its central directory. */
function zipNames(zip: Uint8Array): string[] {
  const view = new DataView(zip.buffer, zip.byteOffset, zip.byteLength);
  const end = zip.length - 22;
  let at = view.getUint32(end + 16, true);
  const names: string[] = [];
  for (let i = 0; i < view.getUint16(end + 10, true); i++) {
    const length = view.getUint16(at + 28, true);
    names.push(new TextDecoder().decode(zip.subarray(at + 46, at + 46 + length)));
    at += 46 + length;
  }
  return names;
}

describe("pages across files", () => {
  it("previews a file, draws its pages, and asks for an encrypted one's password", async () => {
    const client = await opened();
    const preview = await previewSource(client, pages(2, 400));
    expect(preview.pages.map((p) => [p.width, p.height])).toEqual([
      [400, 792],
      [612, 792],
    ]);
    const thumb = await renderSource(client, preview.key, 0, 0.25);
    expect([thumb.width, thumb.height]).toEqual([100, 198]);
    thumb.bitmap.close();
    await closeSource(client, preview.key);

    const locked = await read(ENCRYPTED);
    const refused = await rejection(previewSource(client, locked));
    expect(isLaminaPdfError(refused, "password-required")).toBe(true);
    expect(
      isLaminaPdfError(await rejection(previewSource(client, locked, { password: "nope" })), "wrong-password"),
    ).toBe(true);
    const unlocked = await previewSource(client, locked, { password: PASSWORD });
    expect([unlocked.encrypted, unlocked.pages.length > 0]).toEqual([true, true]);
  });

  it("inserts pages from a source, and adds the source again in a restarted worker before replaying", async () => {
    const client = await opened();
    const added = await client.addSource(pages(2, 400));
    const op = { op: "insertPagesFrom", args: { source: added.id, pages: added.pages, at: "end" } } as Op;
    const result = await client.applyEdit([op], { label: "Insert 2 pages from a file" });
    expect(result.order?.map((p) => p.width)).toEqual([612, 612, 612, 400, 612]);

    const restarted = next(client, "restarted");
    void hook(client, "__crash").catch(() => undefined);
    expect(await restarted).toMatchObject({ reopened: true, edits: "restored" });
    expect(client.document?.pages.map((p) => p.width)).toEqual([612, 612, 612, 400, 612]);
    expect((await client.editHistory()).steps.map((s) => s.label)).toEqual(["Insert 2 pages from a file"]);
  });

  it("loses an insert from an encrypted source when the worker restarts", async () => {
    const client = await opened();
    const added = await client.addSource(await read(ENCRYPTED), { password: PASSWORD });
    const op = { op: "insertPagesFrom", args: { source: added.id, pages: added.pages.slice(0, 1), at: "start" } } as Op;
    await client.applyEdit([op], { label: "Insert 1 page from a file" });
    const restarted = next(client, "restarted");
    void hook(client, "__crash").catch(() => undefined);
    expect(await restarted).toMatchObject({ reopened: true, edits: "lost" });
    expect(client.document?.pages.length).toBe(3);
  });

  it("writes parts as files or as one zip, with progress, leaving the document as it is", async () => {
    const client = await opened(pages(4));
    const ids = client.document?.pages.map((p) => p.id) ?? [];
    const heard: number[] = [];
    const files = await writeParts(
      client,
      [ids.slice(0, 1), ids.slice(1)],
      {},
      { onProgress: (p) => heard.push(p.done) },
    );
    expect("files" in files && files.files.map((bytes) => Document.open(bytes).pageCount)).toEqual([1, 3]);
    expect(heard.at(-1)).toBe(2);
    const zipped = await writeParts(client, [ids.slice(0, 2), ids.slice(2)], {
      zip: { names: ["a.pdf", "b.pdf"], modified: Date.UTC(2026, 10, 15) },
    });
    expect("zip" in zipped && zipNames(zipped.zip)).toEqual(["a.pdf", "b.pdf"]);
    expect(client.document?.pages.length).toBe(4);
    expect((await client.editHistory()).steps).toEqual([]);
  });

  it("merges files into a new document, with the open one first when asked", async () => {
    const client = await opened(pages(3));
    const merged = await mergeFiles(client, [{ bytes: pages(2, 400), title: "Other" }], { open: "Report" });
    using doc = Document.open(merged);
    expect(doc.pages.map((p) => p.width)).toEqual([612, 612, 612, 400, 612]);
    const alone = await mergeFiles(client, [{ bytes: pages(1, 300) }, { bytes: pages(1, 500) }]);
    expect(Document.open(alone).pages.map((p) => p.width)).toEqual([300, 500]);
  });
});
