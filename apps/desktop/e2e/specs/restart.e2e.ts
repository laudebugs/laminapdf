// Scenario 7: killing the engine mid-scroll shows the toast and the page draws again.
import { $, expect } from "@wdio/globals";
import { fixture, goToPage, killEngine, now, openAndWait, view, waitForSharp, waitForSharpSince } from "../app.ts";

describe("the engine crashing", () => {
  it("should restart it and draw the current page again", async () => {
    await openAndWait(fixture("fixtures", "golden", "examples", "flowing_report.pdf"));
    await goToPage(5);
    await waitForSharp(4);
    const since = await now();
    await killEngine();
    await expect($("*=Document process restarted")).toBeDisplayed();
    await waitForSharpSince(4, since);
    expect((await view()).currentPage).toBe(4);
  });
});
