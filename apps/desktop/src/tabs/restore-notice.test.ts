// The restore notice's state: for a moment after the restore the view settles, and page changes
// only say where; after that, leaving the settled page takes the notice away. Another document
// coming on screen settles again.
import { describe, expect, it } from "vitest";
import { RestoreNoticeState, SETTLE_MS } from "./restore-notice.ts";

/** A viewer's page as a store the test moves, a document id, and a clock the test sets. */
function setup() {
  let currentPage = 0;
  let shown: number | undefined = 1;
  let time = 0;
  const listeners = new Set<() => void>();
  const pages = {
    getState: () => ({ currentPage }),
    subscribe: (listener: () => void) => {
      listeners.add(listener);
      return () => listeners.delete(listener);
    },
  };
  const notice = new RestoreNoticeState(
    pages,
    () => shown,
    () => time,
  );
  return {
    notice,
    goTo(page: number) {
      currentPage = page;
      for (const listener of listeners) listener();
    },
    show(document: number) {
      shown = document;
    },
    wait(ms: number) {
      time += ms;
    },
  };
}

describe("the restore notice", () => {
  it("stays while the restored view settles, and goes on the first page turned after", () => {
    const { notice, goTo, wait } = setup();
    notice.show({ tabs: 3, missing: 0 });
    goTo(4);
    goTo(5);
    wait(SETTLE_MS);
    expect(notice.getState()).toEqual({ tabs: 3, missing: 0 });
    goTo(6);
    expect(notice.getState()).toBeUndefined();
  });

  it("settles again when another document comes on screen, such as a located file", () => {
    const { notice, goTo, show, wait } = setup();
    notice.show({ tabs: 2, missing: 1 });
    wait(SETTLE_MS);
    show(2);
    goTo(0);
    goTo(1);
    expect(notice.getState()).toEqual({ tabs: 2, missing: 1 });
    wait(SETTLE_MS);
    goTo(2);
    expect(notice.getState()).toBeUndefined();
  });

  it("goes on Dismiss, and shows nothing for a restore without tabs", () => {
    const { notice } = setup();
    notice.show({ tabs: 0, missing: 0 });
    expect(notice.getState()).toBeUndefined();
    notice.show({ tabs: 2, missing: 2 });
    notice.dismiss();
    expect(notice.getState()).toBeUndefined();
  });
});
