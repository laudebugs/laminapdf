// Reading positions over the browser's own IndexedDB (Chromium, Firefox, WebKit): kept by
// fingerprint, the oldest dropped past the cap in the write's own transaction, so two writers at
// once (as two tabs would) leave no more than the cap, emptied by `clear`, and kept in memory
// where no database opens.
import { beforeEach, describe, expect, it } from "vitest";
import { openDatabase } from "../../src/platform/idb.js";
import { createPositions, POSITIONS_LIMIT } from "../../src/platform/positions.js";

const view = (page: number) => ({ point: { page, x: 0, y: 0.25 }, zoom: 150, rotation: 0 });

/** Positions over their own connection to the page's database, as another tab would hold one. */
const connect = () => createPositions(openDatabase(indexedDB));

beforeEach(async () => {
  await connect().clear();
});

describe("reading positions in IndexedDB", () => {
  it("keeps each document's position by fingerprint, and only what it was given", async () => {
    const positions = connect();
    await positions.set("fp1:a", { view: view(12), pageCount: 40, readAt: 5 });
    await positions.set("fp1:b", { view: view(3), pageCount: 9, readAt: 6 });
    expect(await positions.get("fp1:a")).toEqual({ view: view(12), pageCount: 40, readAt: 5 });
    expect(await connect().get("fp1:b")).toEqual({ view: view(3), pageCount: 9, readAt: 6 });
    expect(await positions.get("fp1:none")).toBeUndefined();
    await positions.remove("fp1:a");
    expect(await positions.get("fp1:a")).toBeUndefined();
  });

  it("drops the position read longest ago when a 501st is written", async () => {
    const positions = connect();
    for (let n = 0; n < POSITIONS_LIMIT; n++) {
      await positions.set(`fp1:${n}`, { view: view(n), pageCount: 1000, readAt: 1000 + n });
    }
    await positions.set("fp1:new", { view: view(0), pageCount: 1000, readAt: 5000 });
    expect(await positions.get("fp1:0")).toBeUndefined();
    expect(await positions.get("fp1:1")).toBeDefined();
    expect(await positions.get("fp1:new")).toBeDefined();
  });

  it("keeps no more than the cap when two connections write at once", async () => {
    const limit = 50;
    const first = createPositions(openDatabase(indexedDB), limit);
    const second = createPositions(openDatabase(indexedDB), limit);
    await Promise.all(
      Array.from({ length: 60 }, (_, n) =>
        Promise.all([
          first.set(`fp1:first-${n}`, { view: view(n), pageCount: 100, readAt: 2 * n }),
          second.set(`fp1:second-${n}`, { view: view(n), pageCount: 100, readAt: 2 * n + 1 }),
        ]),
      ),
    );
    const kept = await Promise.all(
      Array.from({ length: 60 }, async (_, n) => [
        await first.get(`fp1:first-${n}`),
        await first.get(`fp1:second-${n}`),
      ]),
    );
    expect(kept.flat().filter((entry) => entry !== undefined)).toHaveLength(limit);
  });

  it("forgets every position on clear", async () => {
    const positions = connect();
    await positions.set("fp1:a", { view: view(1), pageCount: 2, readAt: 1 });
    await positions.clear();
    expect(await positions.get("fp1:a")).toBeUndefined();
  });

  it("keeps positions in memory, with the same cap, where no database opens", async () => {
    const positions = createPositions(openDatabase(undefined), 2);
    await positions.set("fp1:a", { view: view(1), pageCount: 2, readAt: 1 });
    await positions.set("fp1:b", { view: view(1), pageCount: 2, readAt: 2 });
    await positions.set("fp1:c", { view: view(1), pageCount: 2, readAt: 3 });
    expect([await positions.get("fp1:a"), (await positions.get("fp1:c"))?.readAt]).toEqual([undefined, 3]);
  });
});
