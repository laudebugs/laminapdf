// A large local file read from its OPFS copy (C7b), with the size threshold lowered so a small
// fixture counts as large: the first open copies it (reporting progress) and renders from the
// copy; opening the same file again reads the copy already there; the copy is never left behind
// half made. Where the engine has no OPFS in a test context (Playwright's WebKit) the file opens
// from memory, as before.

import { copyName, type ViewerClient } from "@laminapdf/viewer";
import { afterAll, beforeAll, describe, expect, it } from "vitest";
import { read, viewer } from "./helpers.js";

/** Whether this engine gives OPFS to a test page. */
async function hasOpfs(): Promise<boolean> {
  try {
    await navigator.storage.getDirectory();
    return true;
  } catch {
    return false;
  }
}

/** The copies in the reader's OPFS folder, by name and size. */
async function copies(): Promise<Record<string, number>> {
  const root = await navigator.storage.getDirectory();
  const folder = await root.getDirectoryHandle("documents", { create: true });
  const out: Record<string, number> = {};
  for await (const [name, entry] of (
    folder as unknown as { entries(): AsyncIterable<[string, FileSystemHandle]> }
  ).entries()) {
    out[name] = (await (entry as FileSystemFileHandle).getFile()).size;
  }
  return out;
}

let rpc: ViewerClient;
let file: File;

beforeAll(async () => {
  rpc = await viewer();
  file = new File([(await read("fixtures/corpus/outline.pdf")) as BlobPart], "outline.pdf", {
    lastModified: 1_700_000_000_000,
  });
  if (await hasOpfs())
    await (await navigator.storage.getDirectory()).removeEntry("documents", { recursive: true }).catch(() => undefined);
});
afterAll(() => rpc.terminate());

describe("a large local file", () => {
  it("opens from its OPFS copy, made once with progress, and renders from it", async () => {
    const progress: number[] = [];
    const off = rpc.on("progress", ({ loaded }) => progress.push(loaded));
    const opened = await rpc.open(file, { copyAbove: 0 });
    off();
    expect(opened.pageCount).toBe(3);
    const tile = await rpc.render({ page: 0, scale: 0.5 }, { priority: "visible" });
    expect(tile.bitmap.width).toBeGreaterThan(0);
    tile.bitmap.close();
    if (!(await hasOpfs())) return;
    expect(await copies()).toEqual({ [copyName(file)]: file.size });
    expect(progress.at(-1)).toBe(file.size);

    // The same file again: the copy is read as it is, with nothing copied.
    const again: number[] = [];
    const off2 = rpc.on("progress", ({ loaded }) => again.push(loaded));
    await rpc.close();
    expect((await rpc.open(file, { copyAbove: 0 })).pageCount).toBe(3);
    off2();
    expect(again).toEqual([]);
  });

  it("opens from memory when it is under the threshold", async () => {
    await rpc.close();
    const other = new File([(await read("fixtures/corpus/hello.pdf")) as BlobPart], "hello.pdf");
    expect((await rpc.open(other)).pageCount).toBe(1);
    if (await hasOpfs()) expect(Object.keys(await copies())).not.toContain(copyName(other));
  });
});
