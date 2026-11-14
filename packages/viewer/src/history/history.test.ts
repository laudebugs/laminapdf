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
});
