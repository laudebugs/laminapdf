import type { HistoryEntry, HistoryState, Viewer } from "@laminapdf/viewer";
import { describe, expect, it } from "vitest";
import type { Engine } from "../session.ts";
import { followHistory, pageAddress } from "./history.ts";

describe("pageAddress", () => {
  it("sets the page and keeps the other keys", () => {
    const at = { pathname: "/", search: "", hash: "#url=https%3A%2F%2Fexample.com%2Fa.pdf&zoom=page-fit" };
    expect(pageAddress(at, 4)).toBe("/#url=https%3A%2F%2Fexample.com%2Fa.pdf&zoom=page-fit&page=5");
  });

  it("replaces a page and drops a named destination", () => {
    expect(pageAddress({ pathname: "/r", search: "?x", hash: "#page=2&nameddest=intro" }, 0)).toBe("/r?x#page=1");
  });
});

/** A browser history: entries of (state, URL), and where it is. */
function fakeWindow() {
  const entries: { state: unknown; url: string }[] = [{ state: null, url: "/#url=https%3A%2F%2Fexample.com%2Fa.pdf" }];
  let index = 0;
  const location = {
    pathname: "/",
    search: "",
    get hash() {
      return (entries[index]?.url ?? "").replace(/^[^#]*/, "");
    },
  };
  const win = {
    location,
    crypto: globalThis.crypto,
    history: {
      get state() {
        return entries[index]?.state ?? null;
      },
      pushState(state: unknown, _title: string, url: string) {
        entries.splice(index + 1);
        entries.push({ state, url });
        index++;
      },
      replaceState(state: unknown, _title: string, url?: string) {
        const entry = entries[index];
        if (entry !== undefined) entries[index] = { state, url: url ?? entry.url };
      },
      back: () => index--,
    },
  };
  return { win: win as unknown as Window, entries, back: () => index-- };
}

/** A viewer's history that records what the app asks of it. */
function fakeViewer() {
  const listeners: ((entry: HistoryEntry, replaced: boolean) => void)[] = [];
  let state: HistoryState = { current: undefined, back: undefined, forward: undefined };
  const went: number[] = [];
  const history = {
    getState: () => state,
    onEntry: (l: (entry: HistoryEntry, replaced: boolean) => void) => {
      listeners.push(l);
      return () => undefined;
    },
    goTo: (id: number) => {
      went.push(id);
      return true;
    },
  };
  const entry = (id: number, page: number): HistoryEntry => ({ id, point: { page, x: 0, y: 0 }, zoom: "fit-width" });
  return {
    viewer: { history } as unknown as Viewer,
    went,
    add(id: number, page: number, replaced: boolean) {
      state = { ...state, current: entry(id, page) };
      for (const l of listeners) l(entry(id, page), replaced);
    },
  };
}

const engineOf = (viewer: Viewer) =>
  ({ getState: () => ({ viewer }), subscribe: () => () => undefined }) as unknown as Engine;

describe("followHistory", () => {
  it("marks the entry the reader is on, then pushes each jump with its page and the document's url", () => {
    const { win, entries } = fakeWindow();
    const view = fakeViewer();
    followHistory(win, engineOf(view.viewer));
    view.add(1, 0, true);
    view.add(2, 6, false);
    expect(entries.map((e) => e.url)).toEqual([
      "/#url=https%3A%2F%2Fexample.com%2Fa.pdf",
      "/#url=https%3A%2F%2Fexample.com%2Fa.pdf&page=7",
    ]);
    expect(entries.every((e) => typeof (e.state as { laminaHistory?: unknown }).laminaHistory === "object")).toBe(true);
  });

  it("restores the place an entry marks after Back", () => {
    const { win, back } = fakeWindow();
    const view = fakeViewer();
    const history = followHistory(win, engineOf(view.viewer));
    view.add(1, 0, true);
    view.add(2, 6, false);
    back();
    expect(history.restore()).toBe(true);
    expect(view.went).toEqual([1]);
  });

  it("leaves an entry another view made to the fragment", () => {
    const { win, back } = fakeWindow();
    const first = fakeViewer();
    followHistory(win, engineOf(first.viewer));
    first.add(1, 0, true);
    first.add(2, 3, false);
    // A reload: another view follows the same browser history.
    const second = fakeViewer();
    const history = followHistory(win, engineOf(second.viewer));
    back();
    expect(history.restore()).toBe(false);
    expect(second.went).toEqual([]);
  });

  it("restores nothing for an entry without a mark", () => {
    const { win } = fakeWindow();
    const view = fakeViewer();
    expect(followHistory(win, engineOf(view.viewer)).restore()).toBe(false);
  });
});
