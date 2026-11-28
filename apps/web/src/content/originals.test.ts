import type { ObjectRef, Op, PageObject } from "@laminapdf/sdk";
import { describe, expect, it } from "vitest";
import { Originals } from "./originals.ts";

/** One page of two images, each at (100 × 100) on a 612 × 792 page, whose names change with every edit that touches the page. */
function world(pages = 1) {
  const versions = new Map<number, number>();
  const boxes = new Map<number, [number, number, number, number][]>();
  for (let page = 0; page < pages; page++) {
    versions.set(page, 0);
    boxes.set(page, [
      [100, 100, 200, 200],
      [300, 100, 400, 200],
    ]);
  }
  const ops: { page: number; edits: Record<string, unknown>[] }[] = [];
  let position = 0;
  let chain: Promise<unknown> = Promise.resolve();
  const pageOf = (id: string) => Number(id.slice(1));
  const objects = (page: number): PageObject[] =>
    (boxes.get(page) ?? []).map(
      (bounds, ordinal) =>
        ({
          object: { ordinal, hash: `v${versions.get(page)}` } as unknown as ObjectRef,
          kind: "image",
          bounds,
        }) as unknown as PageObject,
    );
  let originals: Originals;
  /** Moves the box `edit` names, refusing a stale name as the engine does. */
  const move = (page: number, edit: Record<string, unknown>) => {
    if (edit.transform === undefined) return;
    const { object, matrix } = edit.transform as { object: ObjectRef; matrix: number[] };
    if (object.hash !== objects(page)[0]?.object.hash) throw new Error("content-object-stale");
    const box = boxes.get(page)?.[object.ordinal];
    if (box === undefined) return;
    box[0] += matrix[4] ?? 0;
    box[2] += matrix[4] ?? 0;
    box[1] += matrix[5] ?? 0;
    box[3] += matrix[5] ?? 0;
  };
  const apply = async (list: readonly Op[], _label: string, pages: readonly number[]) => {
    const args = (list[0] as unknown as { args: { page: string; edits: Record<string, unknown>[] } }).args;
    const page = pageOf(args.page);
    for (const edit of args.edits) move(page, edit);
    ops.push({ page, edits: args.edits });
    versions.set(page, (versions.get(page) ?? 0) + 1);
    position++;
    await originals.read(pages);
    return true;
  };
  originals = new Originals({
    contentObjects: async (page) => objects(page),
    itemsOn: () => [],
    pageId: (page) => `p${page}`,
    pageSize: () => ({ width: 612, height: 792 }),
    apply,
    applyNow: apply,
    enqueue: <T>(work: () => Promise<T>) => {
      const run = chain.then(work);
      chain = run.catch(() => undefined);
      return run;
    },
    position: () => position,
    changed: () => undefined,
    clearAdded: () => undefined,
  });
  return {
    originals,
    ops,
    boxes,
    setPosition: (to: number) => {
      position = to;
    },
  };
}

describe("changes asked for in quick succession", () => {
  it("names each object by what the change before it left", async () => {
    const { originals, ops, boxes } = world();
    await originals.read([0]);
    originals.choose(0, [originals.on(0)[0] as never]);
    // X then Y, with no wait between: the second is built when its turn comes.
    const both = await Promise.all([originals.setBox({ x: 150 }), originals.setBox({ y: 140 })]);
    expect(both).toEqual([true, true]);
    expect(ops).toHaveLength(2);
    expect(boxes.get(0)?.[0]).toEqual([150, 140, 250, 240]);
  });

  it("moves by the nudges' total once, however long the step takes", async () => {
    const { originals, ops, boxes } = world();
    await originals.read([0]);
    originals.choose(0, [originals.on(0)[0] as never]);
    originals.nudge(5, 0);
    const first = originals.flush();
    originals.nudge(5, 0);
    await first;
    await originals.flush();
    expect(ops).toHaveLength(2);
    expect(boxes.get(0)?.[0]).toEqual([110, 100, 210, 200]);
  });
});

describe("what the page can't say", () => {
  const first = (originals: Originals, page = 0) => originals.on(page)[0] as never;

  it("is kept for the page it is about, and comes back with the history", async () => {
    const { originals, setPosition } = world(2);
    await originals.read([0, 1]);
    originals.choose(0, [first(originals)]);
    await originals.crop([10, 0, 0, 0]);
    expect(originals.cropOf(first(originals))).toEqual([10, 0, 0, 0]);
    // The same ordinal on another page was not cropped.
    expect(originals.cropOf(first(originals, 1))).toBeUndefined();
    // Undo to the start forgets it; Redo brings it back; a step of another class after keeps it.
    setPosition(0);
    await originals.read([0]);
    expect(originals.cropOf(first(originals))).toBeUndefined();
    setPosition(1);
    await originals.read([0]);
    expect(originals.cropOf(first(originals))).toEqual([10, 0, 0, 0]);
    setPosition(2);
    await originals.read([0]);
    expect(originals.cropOf(first(originals))).toEqual([10, 0, 0, 0]);
  });
});
