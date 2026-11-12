// Scenario 4: zooming in and out mid-document keeps the page under the viewport's centre, and
// draws it sharp. The scroll offset scales with the zoom (pixels grow with the page), so the
// anchor kept is the page, with the offset moving the way the zoom does, never back to the top.
// A zoom redraws a sharp page over itself, so each step waits for the new zoom, then the page.
import { expect } from "@wdio/globals";
import { fixture, goToPage, openAndWait, view, waitForSharp, zoomOnce } from "../app.ts";

describe("zoom", () => {
  it("should keep the current page while zooming in and out", async () => {
    await openAndWait(fixture("fixtures", "golden", "examples", "flowing_report.pdf"));
    await goToPage(5);
    await waitForSharp(4);
    const before = await view();
    await zoomOnce("Zoom in");
    await waitForSharp(before.currentPage);
    const zoomedIn = await view();
    expect(zoomedIn.percent).toBeGreaterThan(before.percent);
    expect(zoomedIn.currentPage).toBe(before.currentPage);
    expect(zoomedIn.scrollTop).toBeGreaterThan(before.scrollTop);
    await zoomOnce("Zoom out");
    await zoomOnce("Zoom out");
    await waitForSharp(before.currentPage);
    const zoomedOut = await view();
    expect(zoomedOut.percent).toBeLessThan(before.percent);
    expect(zoomedOut.currentPage).toBe(before.currentPage);
    expect(zoomedOut.scrollTop).toBeLessThan(before.scrollTop);
  });
});
