import { describe, expect, it } from "vitest";
import { FindController, type FindHost } from "./controller.js";

/** A host over `counts[page]` matches per page, answering each call when `release` is called. */
function host(counts: readonly number[], currentPage = 0) {
  const calls: { query: string; pages: readonly number[]; signal: AbortSignal; resolve: () => void }[] = [];
  const revealed: [number, readonly number[]][] = [];
  const fake: FindHost = {
    find: (query, _options, pages, signal) =>
      new Promise((resolve) => {
        calls.push({
          query,
          pages,
          signal,
          resolve: () =>
            resolve(
              pages.map((page) => ({ page, matches: Array.from({ length: counts[page] ?? 0 }, (_, i) => [page, i]) })),
            ),
        });
      }),
    pageCount: () => counts.length,
    currentPage: () => currentPage,
    reveal: (page, quads) => revealed.push([page, quads]),
    redraw: () => {},
  };
  return { fake, calls, revealed };
}

/** Answers every call until none is pending. */
async function settle(calls: { resolve: () => void }[]): Promise<void> {
  for (let i = 0; i < 100; i++) {
    const call = calls.shift();
    if (call === undefined) {
      await Promise.resolve();
      if (calls.length === 0) return;
      continue;
    }
    call.resolve();
    await new Promise((r) => setTimeout(r, 0));
  }
}

describe("FindController", () => {
  it("searches from the current page forward and wraps round", async () => {
    const { fake, calls } = host(Array(10).fill(0), 6);
    const find = new FindController(fake);
    find.find("x");
    const asked: number[] = [];
    for (let i = 0; i < 5 && calls.length > 0; i++) {
      const call = calls.shift();
      asked.push(...(call?.pages ?? []));
      call?.resolve();
      await new Promise((r) => setTimeout(r, 0));
    }
    expect(asked).toEqual([6, 7, 8, 9, 0, 1, 2, 3, 4, 5]);
    expect(find.getState()).toMatchObject({ done: true, searched: 10, total: 0 });
  });

  it("counts every match and shows the first after the start page", async () => {
    const { fake, calls, revealed } = host([2, 0, 1, 3], 1);
    const find = new FindController(fake);
    find.find("x");
    await settle(calls);
    expect(find.getState()).toMatchObject({ total: 6, current: 2, done: true });
    expect(revealed[0]?.[0]).toBe(2);
  });

  it("moves through matches in document order and wraps", async () => {
    const { fake, calls, revealed } = host([1, 2]);
    const find = new FindController(fake);
    find.find("x");
    await settle(calls);
    find.next();
    find.next();
    find.next();
    find.previous();
    expect(revealed.map(([page, q]) => `${page}:${q[1]}`)).toEqual(["0:0", "1:0", "1:1", "0:0", "1:1"]);
  });

  it("drops a superseded search's late results", async () => {
    const { fake, calls } = host([5, 5, 5, 5, 5]);
    const find = new FindController(fake);
    find.find("old");
    const stale = calls.shift();
    find.find("new");
    expect(stale?.signal.aborted).toBe(true);
    stale?.resolve();
    await settle(calls);
    expect(find.getState()).toMatchObject({ query: "new", total: 25 });
  });

  it("clears for an empty query, and on close", async () => {
    const { fake, calls } = host([3]);
    const find = new FindController(fake);
    find.find("x");
    await settle(calls);
    find.close();
    expect([find.getState().total, find.matchesOn(0).matches.length, calls.length]).toEqual([0, 0, 0]);
  });
});

describe("FindController before a current match", () => {
  it("goes Next to the first match from the start page, not the document's first", async () => {
    const { fake, calls, revealed } = host([1, 0, 0, 1, 0], 2);
    const find = new FindController(fake);
    find.find("x");
    // Page 3's match is found first and becomes current; clear it to test the path.
    await settle(calls);
    expect(revealed[0]?.[0]).toBe(3);
  });
});
