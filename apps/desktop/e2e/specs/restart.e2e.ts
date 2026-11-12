// Scenario 7: killing the engine mid-scroll shows the toast, keeps the page, and the restarted
// engine draws what the view needs next.
import { expect } from "@wdio/globals";
import { drawsAfterZoomOut, fixture, goToPage, killEngine, openAndWait, view, waitForSharp, withText } from "../app.ts";

describe("the engine crashing", () => {
  it("should restart it and keep drawing the current page", async () => {
    await openAndWait(fixture("fixtures", "golden", "examples", "flowing_report.pdf"));
    await goToPage(5);
    await waitForSharp(4);
    await killEngine();
    await expect(withText("Document process restarted")).toBeDisplayed();
    expect((await view()).currentPage).toBe(4);
    await drawsAfterZoomOut();
  });
});
