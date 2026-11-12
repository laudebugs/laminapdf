// Scenario 3: the 1,000-page fixture scrolled to its end draws its last page, and the page
// indicator follows; the page field goes back to the start.
import { $, browser, expect } from "@wdio/globals";
import { fixture, goToPage, openAndWait, scrollToEnd, view, waitForSharp } from "../app.ts";

describe("a 1,000-page document", () => {
  it("should reach its last page", async () => {
    await openAndWait(fixture("target", "fixtures", "1000-pages.pdf"));
    expect((await view()).pageCount).toBe(1000);
    await scrollToEnd();
    await waitForSharp(999);
    await browser.waitUntil(async () => (await view()).currentPage === 999, {
      timeoutMsg: "the view isn't on the last page",
    });
    await expect($('input[aria-label="Page"]')).toHaveValue("1000");
    await goToPage(1);
    await waitForSharp(0);
    expect((await view()).currentPage).toBe(0);
  });
});
