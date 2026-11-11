// Scenario 2: a fixture opens through Choose a file and its first page is drawn.
import { expect } from "@wdio/globals";
import { fixture, open, view, waitForSharp } from "../app.ts";

describe("opening a document", () => {
  it("should draw the first page of a file chosen through Choose a file", async () => {
    await open(fixture("fixtures", "golden", "examples", "flowing_report.pdf"));
    await waitForSharp(0);
    expect((await view()).pageCount).toBe(13);
  });
});
