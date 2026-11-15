// Scenario: printing (D4c). The document goes through the same native print path File ▸ Print uses
// (the engine's bands, the OS's print API), to the OS's PDF printer with no dialog, and the engine
// reads the result back: every page printed, each at its own size and orientation (macOS: #626).
import { tmpdir } from "node:os";
import { join } from "node:path";
import { platform } from "node:process";
import { expect } from "@wdio/globals";
import { closeAll, fixture, focusWindow, openAndWait, type PageSize, pageSizes, printToFile } from "../app.ts";

/** D1b's bar: each printed page's size within 2 pt of the document's. */
const TOLERANCE = 2;

/**
 * macOS puts every page on the first page's paper size, turned to the page's orientation (#626),
 * so there only the orientation is checked, and the size of pages the first page's size.
 */
const MAC = platform === "darwin";

const near = (a: number, b: number): boolean => Math.abs(a - b) <= TOLERANCE;

function sameSize(a: PageSize, b: PageSize): boolean {
  return near(a.width, b.width) && near(a.height, b.height);
}

function sameSheet(a: PageSize, b: PageSize): boolean {
  const short = (p: PageSize) => Math.min(p.width, p.height);
  const long = (p: PageSize) => Math.max(p.width, p.height);
  return near(short(a), short(b)) && near(long(a), long(b));
}

function onItsPaper(page: PageSize, out: PageSize, first: PageSize): boolean {
  if (!MAC) return sameSize(page, out);
  const turned = page.width > page.height === out.width > out.height;
  return turned && (sameSize(page, out) || !sameSheet(page, first));
}

function within(printed: readonly PageSize[], document: readonly PageSize[]): string[] {
  const first = document[0];
  return document.flatMap((page, i) => {
    const out = printed[i];
    const off = out === undefined || first === undefined || !onItsPaper(page, out, first);
    return off ? [`page ${i + 1}: ${JSON.stringify(page)} printed as ${JSON.stringify(out)}`] : [];
  });
}

describe("printing", () => {
  // Each case opens its fixture afresh: a retry would otherwise only show the tab still open.
  beforeEach(async () => {
    await focusWindow();
    await closeAll();
  });

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
