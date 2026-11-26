// Several PDFs opened together, in their own run (wdio.open-several.conf.ts): the app started with
// three paths asks whether to merge them into one document, and Merge into one document writes the
// new file where the person chooses and opens it, with every page and a bookmark for each file.
import { existsSync, mkdirSync, readFileSync } from "node:fs";
import { join } from "node:path";
import { $, browser, expect } from "@wdio/globals";
import { getDocument } from "pdfjs-dist/legacy/build/pdf.mjs";
import { invoke, waitForDocument } from "../app.ts";
import { workingCopies } from "../paths.ts";

const dir = join(workingCopies, "open-several");

describe("several files opened together", () => {
  it("[cap C.06 desktop round-trip] asks whether to merge them, and opens the merged file with every page and a bookmark for each", async () => {
    mkdirSync(dir, { recursive: true });
    const merged = join(dir, "hello and others.pdf");
    const ask = $('//*[@role="dialog"][.//*[normalize-space(.)="You opened 3 PDFs"]]');
    await ask.waitForDisplayed({ timeoutMsg: "no question about the three files" });
    expect(await ask.getText()).toContain("insert_pages.pdf");
    await invoke("e2e_queue_save_as", { path: merged });
    await ask.$("button=Merge into one document").click();
    await browser.waitUntil(async () => existsSync(merged), { timeoutMsg: "the merged file wasn't written" });
    // One page of hello, three each of the others, and it opens as a tab.
    await waitForDocument(7);
    const task = getDocument({ data: new Uint8Array(readFileSync(merged)), verbosity: 0 });
    const doc = await task.promise;
    try {
      const outline = (await doc.getOutline()) ?? [];
      expect([doc.numPages, outline.map((item) => item.title)]).toEqual([
        7,
        ["hello", "insert_pages", "extract_pages"],
      ]);
    } finally {
      await task.destroy();
    }
  });
});
