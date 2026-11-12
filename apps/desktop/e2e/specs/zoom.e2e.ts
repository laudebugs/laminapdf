// Scenario 4: zooming in and out keeps the page in view and draws it sharp. A zoom redraws a
// sharp page over itself, so the scenario waits for the new zoom, then for the page.
import { $, browser, expect } from "@wdio/globals";
import { fixture, openAndWait, view, waitForSharp } from "../app.ts";

/** Clicks `label` and waits until the view's zoom leaves `from`. */
async function zoom(label: string, from: number): Promise<void> {
  await $(`button[aria-label="${label}"]`).click();
  await browser.waitUntil(async () => (await view()).percent !== from, { timeoutMsg: `${label} didn't zoom` });
}

describe("zoom", () => {
  it("should keep the current page while zooming in and out", async () => {
    await openAndWait(fixture("fixtures", "golden", "examples", "flowing_report.pdf"));
    const before = await view();
    await zoom("Zoom in", before.percent);
    await waitForSharp(before.currentPage);
    const zoomedIn = await view();
    expect(zoomedIn.percent).toBeGreaterThan(before.percent);
    expect(zoomedIn.currentPage).toBe(before.currentPage);
    await zoom("Zoom out", zoomedIn.percent);
    await zoom("Zoom out", (await view()).percent);
    await waitForSharp(before.currentPage);
    const zoomedOut = await view();
    expect(zoomedOut.percent).toBeLessThan(before.percent);
    expect(zoomedOut.currentPage).toBe(before.currentPage);
  });
});
