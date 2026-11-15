// Scenario: printing (D4c). The document goes through the same native print path File ▸ Print uses
// (the engine's bands, the OS's print API), to the OS's PDF printer with no dialog, and the engine
// reads the result back: every page printed, each at its own size and orientation.
import { tmpdir } from "node:os";
import { join } from "node:path";
import { expect } from "@wdio/globals";
import { fixture, openAndWait, type PageSize, pageSizes, printToFile } from "../app.ts";

/** D1b's bar: each printed page's size within 2 pt of the document's. */
const TOLERANCE = 2;

function within(printed: readonly PageSize[], document: readonly PageSize[]): string[] {
  return document.flatMap((page, i) => {
    const out = printed[i];
    const off =
      out === undefined ||
      Math.abs(out.width - page.width) > TOLERANCE ||
      Math.abs(out.height - page.height) > TOLERANCE;
    return off ? [`page ${i + 1}: ${JSON.stringify(page)} printed as ${JSON.stringify(out)}`] : [];
  });
}

describe("printing", () => {
  it("should print pages of four sizes and orientations each on its own paper", async () => {
    await openAndWait(fixture("fixtures", "print", "mixed-sizes.pdf"));
    const document = await pageSizes();
    const printed = await printToFile(join(tmpdir(), `lamina-print-${Date.now()}.pdf`));
    expect(printed).toHaveLength(document.length);
    expect(within(printed, document)).toEqual([]);
  });

  it("should print all 50 pages of a long document", async () => {
    await openAndWait(fixture("fixtures", "print", "mixed-50.pdf"));
    const printed = await printToFile(join(tmpdir(), `lamina-print-50-${Date.now()}.pdf`));
    expect(printed).toHaveLength(50);
  });
});
