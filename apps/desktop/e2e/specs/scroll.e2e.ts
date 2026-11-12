// Scenario 3: the 1,000-page fixture reaches its last page and the indicator follows.
import { $, expect } from "@wdio/globals";
import { fixture, goToPage, openAndWait, view, waitForSharp } from "../app.ts";

describe("a 1,000-page document", () => {
  it("should reach its last page", async () => {
    await openAndWait(fixture("target", "fixtures", "1000-pages.pdf"));
    expect((await view()).pageCount).toBe(1000);
    await goToPage(1000);
    await waitForSharp(999);
    expect((await view()).currentPage).toBe(999);
    await expect($('input[aria-label="Page"]')).toHaveValue("1000");
  });
});
