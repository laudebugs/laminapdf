// Text boxes in a document the worker opened in the base module, before the extract module
// loaded, in Chromium, Firefox and WebKit. The base module refuses every text box; after
// `editInTextCopy` the worker edits in a copy that lays them out, with the edits made before it,
// so undo and redo still walk the whole history, saves still adopt, and a restarted worker does
// the same. With `text`, the copy is in the text module, for fonts given by their bytes: a box's
// own font and its fallbacks.

import { Builder, init, type Op } from "@laminapdf/sdk";
import type { ViewerClient } from "@laminapdf/viewer";
import { afterEach, beforeAll, describe, expect, it } from "vitest";
import { hook, next, read, rejection, testViewer } from "./helpers.js";

let bytes: Uint8Array;
let rpc: ViewerClient | undefined;

beforeAll(async () => {
  await init();
  const b = new Builder({ id: new TextEncoder().encode("lamina-e4l-copy0"), producer: "lamina tests" });
  b.page([612, 792]).finish();
  const doc = b.finish();
  bytes = doc.save();
  doc.free();
});

afterEach(() => {
  rpc?.terminate();
  rpc = undefined;
});

/** A fresh worker with the page open and nothing drawn yet, so the model is in the base module. */
async function opened(): Promise<{ client: ViewerClient; page: string }> {
  rpc = await testViewer("pdfium");
  await rpc.open(bytes);
  return { client: rpc, page: rpc.document?.pages[0]?.id as string };
}

/**
 * The op that adds a text box reading `text` near the top of `page`, in `font` (Helvetica when
 * left out), with `fallbacks` for the characters it lacks.
 */
function textBox(page: string, text: string, font?: string, fallbacks?: string[]): Op[] {
  const spec = {
    type: "freeText",
    rect: { x0: 72, y0: 700, x1: 272, y1: 740 },
    text: [[{ text }]],
    ...(font === undefined ? {} : { font }),
    ...(fallbacks === undefined ? {} : { fallbacks }),
  };
  return [{ op: "addAnnot", args: { page, spec } } as Op];
}

const turn = (page: string, rotate = 90): Op[] => [{ op: "rotatePage", args: { page, rotate } } as Op];

/** The box's text, or the error code reading it gave (`unknown-annotation` once it is undone). */
async function textOf(client: ViewerClient, id: string): Promise<string> {
  try {
    const box = await client.textBox(id);
    return box?.spec.text.map((p) => p.map((s) => s.text).join("")).join("\n") ?? "no box";
  } catch (e) {
    return (e as { code?: string }).code ?? String(e);
  }
}

describe("editing in a text-capable copy", () => {
  it("refuses a text box in the base module, and takes one after editInTextCopy, keeping the history", async () => {
    const { client, page } = await opened();
    await client.applyEdit(turn(page), { label: "Rotate page 1" });
    // An undo and a redo before the switch, which the copy makes after the model.
    await client.undoEdit();
    await client.redoEdit();
    const refused = await rejection(client.applyEdit(textBox(page, "Refused")));
    expect((refused as { code?: string }).code).toBe("appearance-needs-text");

    await client.editInTextCopy();
    const id = (await client.applyEdit(textBox(page, "Taken"), { label: "Add a text box" }))
      .createdAnnotations?.[0] as string;
    const added = await textOf(client, id);
    await client.undoEdit();
    await client.undoEdit();
    const undone = [await textOf(client, id), client.document?.pages[0]?.rotation];
    await client.redoEdit();
    await client.redoEdit();
    const history = await client.editHistory();

    expect([added, undone, await textOf(client, id), history.steps.map((s) => s.label), history.position]).toEqual([
      "Taken",
      ["unknown-annotation", 0],
      "Taken",
      ["Rotate page 1", "Add a text box"],
      2,
    ]);
  });

  it("moves to the text module after the extract copy opened, for fonts given by their bytes, and saves", async () => {
    const { client, page } = await opened();
    // A page drawn loads the extract module; reading the annotations opens the extract copy.
    (await client.render({ page: 0, scale: 1 })).bitmap.close();
    await client.pageAnnotations(0);
    await client.applyEdit(turn(page), { label: "Rotate page 1" });
    await client.adoptEdits((await client.copyEdits({})).token);
    const font = await client.addBlob(await read("fixtures/fonts/NotoSans-Regular.ttf"));
    const arabic = await client.addBlob(await read("fixtures/fonts/NotoSansArabic-Regular.ttf"));

    await client.editInTextCopy({ text: true });
    const id = (await client.applyEdit(textBox(page, "Noto", font))).createdAnnotations?.[0] as string;
    // Helvetica has no Arabic: the fallback font draws it.
    const fallback = (await client.applyEdit(textBox(page, "Hello مرحبا", undefined, [arabic])))
      .createdAnnotations?.[0] as string;
    const box = await client.textBox(id);
    const backed = await client.textBox(fallback);
    const saved = await client.adoptEdits((await client.copyEdits({})).token);

    expect([box?.spec.font, backed?.spec.fallbacks, saved.unsaved, saved.steps.length]).toEqual([font, [arabic], 0, 3]);
  });

  it("opens the copy from a save that rewrote the file, with the edits since it", async () => {
    const { client, page } = await opened();
    await client.applyEdit(turn(page), { label: "Rotate page 1" });
    await client.adoptEdits((await client.copyEdits({ mode: "fullRewrite" })).token);
    await client.applyEdit(turn(page, 180), { label: "Rotate page 1 again" });

    await client.editInTextCopy();
    const id = (await client.applyEdit(textBox(page, "After the save"))).createdAnnotations?.[0] as string;
    await client.undoEdit();
    await client.undoEdit();
    const history = await client.editHistory();

    expect([await textOf(client, id), history.steps.map((s) => s.label), history.position]).toEqual([
      "unknown-annotation",
      ["Rotate page 1 again", "addAnnot"],
      0,
    ]);
  });

  it("does it again in a restarted worker, which takes back the text boxes", async () => {
    const { client, page } = await opened();
    await client.editInTextCopy();
    const id = (await client.applyEdit(textBox(page, "Kept"))).createdAnnotations?.[0] as string;
    const restarted = next(client, "restarted");

    void hook(client, "__crash").catch(() => undefined);

    expect(await restarted).toMatchObject({ reopened: true, edits: "restored" });
    expect(await textOf(client, id)).toBe("Kept");
  });
});
