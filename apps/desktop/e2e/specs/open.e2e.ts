// Scenario 2: a fixture opens through Choose a file and its first page is drawn: the viewer says
// so, and the page's canvas holds ink, not blank paper.
import { expect } from "@wdio/globals";
import { fixture, inkedPixels, openAndWait, view } from "../app.ts";

describe("opening a document", () => {
  it("should draw the first page of a file chosen through Choose a file", async () => {
    await openAndWait(fixture("fixtures", "golden", "examples", "flowing_report.pdf"));
    expect((await view()).pageCount).toBe(13);
    expect(await inkedPixels(0)).toBeGreaterThan(0);
  });
});
