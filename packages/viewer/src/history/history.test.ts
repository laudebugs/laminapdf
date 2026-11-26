import { describe, expect, it } from "vitest";
import type { Zoom } from "../zoom/zoom.js";
import { type HistoryEntry, type HistoryPlace, NavigationHistory } from "./history.js";

/** A view that is wherever it was last put: `scroll` moves it without a jump. */
function fakeView(start: HistoryPlace = { point: { page: 0, x: 0, y: 0 }, zoom: "fit-width" }) {
  let place: HistoryPlace | undefined = start;
  return {
    host: { place: () => place, show: (p: HistoryPlace) => (place = p) },
    scroll: (page: number, y = 0, zoom: Zoom = "fit-width") => (place = { point: { page, x: 0, y }, zoom }),
    close: () => (place = undefined),
    get page() {
      return place?.point.page;
    },
  };
}

const pageOf = (e: HistoryEntry | undefined) => e?.point.page;

describe("NavigationHistory", () => {
  it("goes back to where a jump left from, and forward again", () => {
    const view = fakeView();
    const history = new NavigationHistory(view.host);
    history.jump(() => view.scroll(5));
    expect(view.page).toBe(5);
    expect(history.back()).toBe(true);
    expect(view.page).toBe(0);
    expect(history.forward()).toBe(true);
    expect(view.page).toBe(5);
    expect(history.forward()).toBe(false);
  });

  it("records no entry for scrolling alone", () => {
    const view = fakeView();
    const history = new NavigationHistory(view.host);
    view.scroll(3);
    view.scroll(4);
    expect(history.getState()).toEqual({ current: undefined, back: undefined, forward: undefined });
    expect(history.back()).toBe(false);
  });

  it("returns to where the reader had scrolled before the jump", () => {
    const view = fakeView();
    const history = new NavigationHistory(view.host);
    history.jump(() => view.scroll(2));
    view.scroll(3, 0.5);
    history.jump(() => view.scroll(9));
    history.back();
    expect(view.host.place()?.point).toEqual({ page: 3, x: 0, y: 0.5 });
  });

  it("keeps where Back left from for Forward", () => {
    const view = fakeView();
    const history = new NavigationHistory(view.host);
    history.jump(() => view.scroll(7));
    view.scroll(8, 0.25);
    history.back();
    history.forward();
    expect(view.host.place()?.point).toEqual({ page: 8, x: 0, y: 0.25 });
  });

  it("drops Forward after a new jump", () => {
    const view = fakeView();
    const history = new NavigationHistory(view.host);
    history.jump(() => view.scroll(1));
    history.jump(() => view.scroll(2));
    history.back();
    history.jump(() => view.scroll(6));
    expect(pageOf(history.getState().forward)).toBeUndefined();
    expect(pageOf(history.getState().back)).toBe(1);
  });

  it("adds nothing for a jump that lands where the view is", () => {
    const view = fakeView();
    const history = new NavigationHistory(view.host);
    history.jump(() => view.scroll(4));
    const before = history.getState();
    history.jump(() => undefined);
    expect(history.getState()).toBe(before);
  });

  it("keeps the zoom with each entry", () => {
    const view = fakeView();
    const history = new NavigationHistory(view.host);
    history.jump(() => view.scroll(4, 0, 200));
    history.back();
    expect(view.host.place()?.zoom).toBe("fit-width");
    history.forward();
    expect(view.host.place()?.zoom).toBe(200);
  });

  it("says where Back and Forward go", () => {
    const view = fakeView();
    const history = new NavigationHistory(view.host);
    history.jump(() => view.scroll(1));
    history.jump(() => view.scroll(2));
    history.back();
    const state = history.getState();
    expect([pageOf(state.back), pageOf(state.current), pageOf(state.forward)]).toEqual([0, 1, 2]);
  });

  it("keeps at most max entries, oldest out first", () => {
    const view = fakeView();
    const history = new NavigationHistory(view.host, { max: 3 });
    for (const page of [1, 2, 3, 4]) history.jump(() => view.scroll(page));
    history.back();
    history.back();
    expect(history.back()).toBe(false);
    expect(view.page).toBe(2);
  });

  it("tells the app about each entry it adds, the first as a replacement", () => {
    const view = fakeView();
    const history = new NavigationHistory(view.host);
    const added: [number | undefined, boolean][] = [];
    history.onEntry((entry, replaced) => added.push([pageOf(entry), replaced]));
    history.jump(() => view.scroll(5));
    history.back();
    history.jump(() => view.scroll(6));
    expect(added).toEqual([
      [0, true],
      [5, false],
      [6, false],
    ]);
  });

  it("goes to an entry by id, and refuses one it no longer has", () => {
    const view = fakeView();
    const history = new NavigationHistory(view.host);
    const ids: number[] = [];
    history.onEntry((entry) => ids.push(entry.id));
    history.jump(() => view.scroll(5));
    history.jump(() => view.scroll(8));
    expect(history.goTo(ids[0] as number)).toBe(true);
    expect(view.page).toBe(0);
    history.reset();
    expect(history.goTo(ids[1] as number)).toBe(false);
  });

  it("records nothing without a document", () => {
    const view = fakeView();
    view.close();
    const history = new NavigationHistory(view.host);
    history.jump(() => undefined);
    expect(history.getState().current).toBeUndefined();
  });

  it("moves entries with their pages, and merges two that a deleted page made one", () => {
    const view = fakeView();
    const history = new NavigationHistory(view.host);
    history.jump(() => view.scroll(3, 0.2));
    history.jump(() => view.scroll(3, 0.7));
    history.jump(() => view.scroll(6));
    // Page 3 is deleted (its places go to the top of the next page, now 3); page 6 is now 5.
    history.remap((old) => (old === 3 ? { page: 3, kept: false } : { page: old > 3 ? old - 1 : old, kept: true }));
    expect(pageOf(history.getState().current)).toBe(5);
    expect(history.back()).toBe(true);
    expect([view.page, history.getState().back?.point.page]).toEqual([3, 0]);
    expect(history.back()).toBe(true);
    expect(view.page).toBe(0);
  });

  it("snapshots and restores a tab's history without firing onEntry or reusing entry ids", () => {
    const view = fakeView();
    const history = new NavigationHistory(view.host);
    history.jump(() => view.scroll(4));
    history.jump(() => view.scroll(9));
    expect(history.back()).toBe(true);
    const saved = history.snapshot();

    // Another tab resets and builds its own history.
    history.reset();
    history.jump(() => view.scroll(2));
    expect(pageOf(history.getState().back)).toBe(4);

    // Restoring the first tab puts back its back and forward entries without emitting onEntry.
    const pushed: number[] = [];
    history.onEntry((entry) => pushed.push(entry.id));
    history.restore(saved);
    expect(pushed).toEqual([]);
    expect([
      pageOf(history.getState().back),
      pageOf(history.getState().current),
      pageOf(history.getState().forward),
    ]).toEqual([0, 4, 9]);

    // Stepping forward and jumping next allocates a fresh id above restored ones.
    expect(history.forward()).toBe(true);
    expect(view.page).toBe(9);
    const maxRestoredId = Math.max(...saved.entries.map((entry) => entry.id));
    history.jump(() => view.scroll(12));
    expect(pushed).toHaveLength(1);
    expect(pushed[0]).toBeGreaterThan(maxRestoredId);
  });

  it("restores an empty snapshot and trims snapshots larger than max", () => {
    const view = fakeView();
    const full = new NavigationHistory(view.host);
    full.jump(() => view.scroll(2));
    full.jump(() => view.scroll(5));
    full.jump(() => view.scroll(8));
    const saved = full.snapshot();

    const small = new NavigationHistory(view.host, { max: 2 });
    small.restore(saved);
    expect([pageOf(small.getState().back), pageOf(small.getState().current)]).toEqual([5, 8]);
    small.restore({ entries: [], index: -1 });
    expect(small.getState()).toEqual({ current: undefined, back: undefined, forward: undefined });
  });

  it("clamps NaN or out-of-range snapshot indices, ignores non-finite ids, and isolates snapshots from mutation", () => {
    const view = fakeView();
    const history = new NavigationHistory(view.host);
    history.jump(() => view.scroll(3));
    const saved = history.snapshot();
    // Mutating the returned snapshot does not alter the live history.
    (saved.entries[0] as { point: { page: number } }).point.page = 99;
    expect(pageOf(history.getState().back)).toBe(0);

    history.restore({
      entries: [
        { id: 2.7, point: { page: 1, x: 0, y: 0 }, zoom: "fit-width" },
        { id: Number.POSITIVE_INFINITY, point: { page: 4, x: 0, y: 0 }, zoom: "fit-width" },
      ],
      index: Number.NaN,
    });
    expect(pageOf(history.getState().current)).toBe(1);
    expect(pageOf(history.getState().forward)).toBe(4);

    // Jumping next allocates a finite integer id above any finite restored id.
    const pushed: number[] = [];
    history.onEntry((entry) => pushed.push(entry.id));
    history.jump(() => view.scroll(6));
    expect(pushed).toHaveLength(1);
    expect(Number.isInteger(pushed[0])).toBe(true);
    expect(pushed[0]).toBeGreaterThanOrEqual(3);

    history.restore({
      entries: [{ id: 1, point: { page: 2, x: 0, y: 0 }, zoom: "fit-width" }],
      index: Number.NEGATIVE_INFINITY,
    });
    expect(pageOf(history.getState().current)).toBe(2);
    history.restore({
      entries: [{ id: 1, point: { page: 2, x: 0, y: 0 }, zoom: "fit-width" }],
      index: Number.POSITIVE_INFINITY,
    });
    expect(pageOf(history.getState().current)).toBe(2);
  });
});
