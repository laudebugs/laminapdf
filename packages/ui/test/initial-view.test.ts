// The initial view dialog's form logic (O13) and stepping through article threads: what Apply
// sends for each change, a kept open action, and parts on missing pages skipped.
import { describe, expect, it } from "vitest";
import { changeOf, formFrom, type InitialViewData } from "../src/reader/initial-view-form.js";
import { stepThreadPart, type ThreadData } from "../src/reader/threads.js";
import { threadTitle } from "../src/reader/threads-panel.js";

const NOTHING: InitialViewData = {
  open: { kind: "default" },
  preferences: {
    hideToolbar: false,
    hideMenubar: false,
    hideWindowUi: false,
    fitWindow: false,
    displayDocTitle: false,
  },
};

describe("the initial view form", () => {
  it("reads nothing set as the reader's defaults, and makes no change from it", () => {
    const form = formFrom(NOTHING);
    expect(form).toEqual({
      page: undefined,
      zoom: "default",
      layout: "unset",
      mode: "unset",
      window: { displayDocTitle: false, fitWindow: false, hideControls: false },
    });
    expect(changeOf(form, form, false)).toBeUndefined();
  });

  it("sends only what changed", () => {
    const before = formFrom({ ...NOTHING, pageMode: "outlines", pageLayout: "oneColumn" });
    const after = { ...before, page: 2, zoom: "fitWidth" as const, layout: "unset" as const };
    expect(changeOf(before, after, false)).toEqual({ open: { page: 2, zoom: "fitWidth" }, pageLayout: null });
  });

  it("removes the open action when the page and zoom go back to the reader's", () => {
    const before = formFrom({ ...NOTHING, open: { kind: "page", page: 3, zoom: 125 } });
    expect(changeOf(before, { ...before, page: undefined, zoom: "default" }, false)).toEqual({ open: null });
  });

  it("opens at the first page when only the zoom is chosen", () => {
    const before = formFrom(NOTHING);
    expect(changeOf(before, { ...before, zoom: "fitPage" }, false)).toEqual({ open: { page: 0, zoom: "fitPage" } });
  });

  it("never names the page of a file whose open action runs an action", () => {
    const before = formFrom({ ...NOTHING, open: { kind: "action", action: "JavaScript" } });
    const after = { ...before, page: 1, mode: "thumbnails" as const };
    expect(changeOf(before, after, true)).toEqual({ pageMode: "thumbnails" });
  });

  it("hides or shows the reader's toolbar, menus and window controls together", () => {
    const before = formFrom(NOTHING);
    const after = { ...before, window: { ...before.window, hideControls: true, displayDocTitle: true } };
    expect(changeOf(before, after, false)).toEqual({
      preferences: { displayDocTitle: true, hideToolbar: true, hideMenubar: true, hideWindowUi: true },
    });
  });
});

describe("article threads", () => {
  const threads: ThreadData[] = [
    { title: "  ", parts: [{}, { page: 4 }] },
    { title: "Backlog", parts: [{ page: 2 }, {}, { page: 5 }] },
  ];

  it("starts at the first part on a page the file has", () => {
    expect(stepThreadPart(threads, undefined, 1)).toEqual({ thread: 0, part: 1 });
    expect(stepThreadPart(threads, undefined, -1)).toBeUndefined();
  });

  it("skips a part whose page is missing, and stops at the thread's ends", () => {
    expect(stepThreadPart(threads, { thread: 1, part: 0 }, 1)).toEqual({ thread: 1, part: 2 });
    expect(stepThreadPart(threads, { thread: 1, part: 2 }, -1)).toEqual({ thread: 1, part: 0 });
    expect(stepThreadPart(threads, { thread: 1, part: 2 }, 1)).toBeUndefined();
    expect(stepThreadPart(threads, { thread: 0, part: 1 }, -1)).toBeUndefined();
  });

  it("names a thread without a title Untitled thread", () => {
    expect(threads.map(threadTitle)).toEqual(["Untitled thread", "Backlog"]);
  });
});
