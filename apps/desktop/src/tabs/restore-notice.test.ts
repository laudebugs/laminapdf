// The restore notice's state: it waits for the viewer to reach the restored tab's page, then the
// first page change takes it away; a missing file found or closed counts down.
import { describe, expect, it } from "vitest";
import { RestoreNoticeState } from "./restore-notice.ts";

/** A viewer's page as a store the test moves. */
function pages(start: number) {
  let currentPage = start;
  const listeners = new Set<() => void>();
  return {
    getState: () => ({ currentPage }),
    subscribe: (listener: () => void) => {
      listeners.add(listener);
      return () => listeners.delete(listener);
    },
    goTo(page: number) {
      currentPage = page;
      for (const listener of listeners) listener();
    },
  };
}

describe("the restore notice", () => {
  // The document the viewer shows.
  let shown: number | undefined = 1;

  it("stays while the viewer moves to the restored page, and goes on the first page after it", () => {
    const viewer = pages(0);
    const notice = new RestoreNoticeState(viewer, () => shown);
    notice.show({ tabs: 3, missing: 0 }, 8);
    viewer.goTo(4);
    viewer.goTo(8);
    expect(notice.getState()).toEqual({ tabs: 3, missing: 0 });
    viewer.goTo(9);
    expect(notice.getState()).toBeUndefined();
  });

  it("goes on the first page change when no document is coming back (its file is missing)", () => {
    const viewer = pages(0);
    const notice = new RestoreNoticeState(viewer, () => shown);
    notice.show({ tabs: 1, missing: 1 }, undefined);
    viewer.goTo(2);
    expect(notice.getState()).toBeUndefined();
  });

  it("counts a missing file down when it is found, and goes on Dismiss", () => {
    const notice = new RestoreNoticeState(pages(0), () => shown);
    notice.show({ tabs: 2, missing: 2 }, 0);
    notice.found();
    expect(notice.getState()).toEqual({ tabs: 2, missing: 1 });
    notice.dismiss();
    expect(notice.getState()).toBeUndefined();
  });

  it("shows nothing for a restore without tabs", () => {
    const notice = new RestoreNoticeState(pages(0), () => shown);
    notice.show({ tabs: 0, missing: 0 }, undefined);
    expect(notice.getState()).toBeUndefined();
  });

  it("settles on another document's first page instead of taking it for a navigation", () => {
    const viewer = pages(0);
    const notice = new RestoreNoticeState(viewer, () => shown);
    shown = 1;
    notice.show({ tabs: 2, missing: 1 }, 0);
    shown = 2;
    viewer.goTo(3);
    expect(notice.getState()).toEqual({ tabs: 2, missing: 1 });
    viewer.goTo(4);
    expect(notice.getState()).toBeUndefined();
  });
});
