// The sidebar (D9d): View ▸ Show or hide sidebar is available with a document open and ticks while
// the sidebar shows; its Pages tab lists a thumbnail per page and goes to a clicked one; its
// Outline tab lists the bookmarks of a file that has them and goes to a clicked one, and says so for
// a file that has none.
import { $, $$, browser, expect } from "@wdio/globals";
import {
  closeAll,
  fixture,
  focusWindow,
  goToPage,
  menuClick,
  menuSnapshot,
  openAndWait,
  view,
  waitForMenus,
} from "../app.ts";

const outlined = fixture("fixtures", "corpus", "outline.pdf");
const plain = fixture("fixtures", "golden", "metadata.pdf");
const TOGGLE = "cmd:view.toggle-sidebar";
const UNTICKED = /Show or hide sidebar ☐\s+<cmd:view\.toggle-sidebar>/;
const TICKED = /Show or hide sidebar ✓\s+<cmd:view\.toggle-sidebar>/;

/** The sidebar's tab named `name`. */
const tab = (name: string) => $(`[role="tab"]=${name}`);

/** Opens or closes the sidebar through View ▸ Show or hide sidebar, whichever state the window was in. */
async function showSidebar(open: boolean): Promise<void> {
  const ticked = TICKED.test(await menuSnapshot());
  if (ticked !== open) await menuClick(TOGGLE);
  await waitForMenus(open ? TICKED : UNTICKED, `the sidebar didn't ${open ? "open" : "close"}`);
}

describe("the sidebar", () => {
  // Each test opens its own copy of a file: a file already open answers with its tab, not a new drawing.
  beforeEach(async () => {
    await closeAll();
  });

  it("should offer the sidebar with a document open and tick it while it shows", async () => {
    await openAndWait(outlined);
    await focusWindow();
    await showSidebar(false);
    await showSidebar(true);
    await expect(tab("Pages")).toBeDisplayed();
    await showSidebar(false);
  });

  it("should list a thumbnail per page and go to the page that is clicked", async () => {
    await openAndWait(outlined);
    await goToPage(1);
    await showSidebar(true);
    await tab("Pages").click();
    await browser.waitUntil(async () => (await $$(".lamina-thumbnail").length) === 3, {
      timeoutMsg: "the Pages tab didn't list the 3 pages",
    });
    await $('.lamina-thumbnail[data-page-index="2"]').click();
    await browser.waitUntil(async () => (await view()).currentPage === 2, {
      timeoutMsg: "clicking the third thumbnail didn't go to page 3",
    });
    await showSidebar(false);
  });

  it("[cap I.11 desktop read] should list the outline of a file that has one and go to a clicked bookmark", async () => {
    await openAndWait(outlined);
    await goToPage(1);
    await showSidebar(true);
    await tab("Outline").click();
    const chapter = $(".paper-outline-title=Chapter Ł1 — Café");
    await chapter.waitForDisplayed({ timeoutMsg: "the Outline tab didn't list the second bookmark" });
    await chapter.click();
    await browser.waitUntil(async () => (await view()).currentPage === 1, {
      timeoutMsg: "clicking the bookmark didn't go to its page",
    });
    await showSidebar(false);
  });

  it("should say a file without bookmarks has none", async () => {
    await openAndWait(plain);
    await showSidebar(true);
    await tab("Outline").click();
    await expect($("h3=This file has no outline")).toBeDisplayed();
    await showSidebar(false);
  });
});
