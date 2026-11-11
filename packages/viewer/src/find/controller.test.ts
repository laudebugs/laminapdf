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
              pages.map((page) => ({
                page,
                matches: Array.from({ length: counts[page] ?? 0 }, (_, i) => [page, i]),
                snippets: Array.from({ length: counts[page] ?? 0 }, (_, i) => ({
                  before: "",
                  match: `p${page}m${i}`,
                  after: "",
                })),
              })),
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

  it("lists every match in document order, whatever order the pages were searched in", async () => {
    const { fake, calls } = host([2, 0, 1, 3], 2);
    const find = new FindController(fake);
    find.find("x");
    await settle(calls);
    const list = find.results();
    expect(list.map((r) => [r.ordinal, r.page, r.snippet.match])).toEqual([
      [0, 0, "p0m0"],
      [1, 0, "p0m1"],
      [2, 2, "p2m0"],
      [3, 3, "p3m0"],
      [4, 3, "p3m1"],
      [5, 3, "p3m2"],
    ]);
    // The same list until the state changes.
    expect(find.results()).toBe(list);
    find.next();
    expect(find.results()).not.toBe(list);
  });

  it("goes to a result by its ordinal and keeps Next going from there", async () => {
    const { fake, calls, revealed } = host([2, 0, 1, 3]);
    const find = new FindController(fake);
    find.find("x");
    await settle(calls);
    find.goTo(4);
    expect(find.getState().current).toBe(4);
    expect(revealed.at(-1)).toEqual([3, [3, 1]]);
    find.next();
    expect(find.getState().current).toBe(5);
    // Out of range or not a whole number: nothing changes.
    for (const bad of [-1, 6, 1.5]) find.goTo(bad);
    expect(find.getState().current).toBe(5);
  });

  it("empties the list for a new query and on close", async () => {
    const { fake, calls } = host([1, 1]);
    const find = new FindController(fake);
    find.find("x");
    await settle(calls);
    expect(find.results()).toHaveLength(2);
    find.find("y");
    expect(find.results()).toEqual([]);
    await settle(calls);
    find.close();
    expect(find.results()).toEqual([]);
  });

  it("searches edited pages again without moving the view, keeping the current match where it can", async () => {
    const counts = [2, 3, 1];
    const { fake, calls, revealed } = host(counts);
    const find = new FindController(fake);
    find.find("x");
    await settle(calls);
    find.goTo(4); // page 1's last match
    const shown = revealed.length;
    // An edit leaves page 1 with one match and page 2 with none.
    counts[1] = 1;
    counts[2] = 0;
    find.refresh([1, 2]);
    expect(calls.map((c) => c.pages)).toEqual([[1, 2]]);
    await settle(calls);
    expect(find.getState()).toMatchObject({ total: 3, current: 2, done: true });
    expect(find.matchesOn(2).matches).toEqual([]);
    expect(revealed.length).toBe(shown);
  });

  it("leaves an idle find alone, and searches pages edited during a search again once it ends", async () => {
    const counts = [1, 1, 1, 1, 1, 1, 1, 1];
    const { fake, calls } = host(counts);
    const find = new FindController(fake);
    find.refresh([0]);
    expect(calls).toEqual([]);
    find.find("x");
    // The first call has read pages 0–3; then an edit takes page 0's match away.
    calls.shift()?.resolve();
    await new Promise((r) => setTimeout(r, 0));
    counts[0] = 0;
    find.refresh([0]);
    expect(calls.map((c) => c.pages)).toEqual([[4, 5, 6, 7]]);
    await settle(calls);
    expect(find.getState()).toMatchObject({ total: 7, done: true });
    expect(find.matchesOn(0).matches).toEqual([]);
  });
});
