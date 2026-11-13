// Editing bookmarks in the built reader (todo E3b, Paper O11), in this run's browser: the Outline
// tab's editor adds a bookmark for the current view, renames it in place, nests it and moves it
// with the keyboard, and the named destinations dialog renames a name a bookmark uses. Saved
// (Save downloads where the browser can't write files) and read back by Lamina PDF's SDK, the
// outline and the names are as edited; reopened, the reader shows them. No CSP violation or other
// origin throughout.

import { readFileSync } from "node:fs";
import { Document, init, type OutlineNode } from "@laminapdf/sdk";
import type { Browser, Page } from "playwright";
import { afterAll, beforeAll, describe, expect, inject, it } from "vitest";
import { BROWSER, launch } from "./browser.ts";
import { clean, load, type Watched } from "./reader.ts";

const origin = inject("origin");
const FILE = new URL("../../../fixtures/golden/links-and-outlines.pdf", import.meta.url);

/** No pickers: Save becomes Download, in every engine, so the saved bytes are the download's. */
const NO_PICKERS = {
  script: () => {
    Reflect.deleteProperty(window, "showOpenFilePicker");
    Reflect.deleteProperty(window, "showSaveFilePicker");
  },
  arg: [],
};

/** The outline as (depth, title), in document order. */
function flatten(nodes: readonly OutlineNode[], depth = 0): [number, string][] {
  return nodes.flatMap((n) => [[depth, n.title] as [number, string], ...flatten(n.children, depth + 1)]);
}

/** Presses Shift and the reader's command key (⌘ on Apple's systems, Ctrl elsewhere) with `key`. */
async function move(page: Page, key: "ArrowUp" | "ArrowDown" | "ArrowLeft" | "ArrowRight"): Promise<void> {
  const apple = await page.evaluate(() => /Mac|iPhone|iPad/.test(navigator.platform));
  await page.keyboard.press(`Shift+${apple ? "Meta" : "Control"}+${key}`);
}

/** Waits until the top bar counts `count` unsaved changes. */
async function unsaved(page: Page, count: number): Promise<void> {
  const text = count === 1 ? "1 unsaved change" : `${count} unsaved changes`;
  await expect.poll(() => page.locator(".paper-top-bar-save-state").textContent()).toBe(text);
}

describe(`bookmarks in ${BROWSER}`, () => {
  let browser: Browser;

  beforeAll(async () => {
    await init({ extract: true });
    browser = await launch();
  });

  afterAll(async () => {
    await browser?.close();
  });

  /** The reader with `bytes` open and the bookmark editor showing. */
  async function editing(watched: Watched, file: string | { name: string; buffer: Buffer }): Promise<void> {
    const { page } = watched;
    const chooser = page.waitForEvent("filechooser");
    // With a document open, Choose a file is in the More menu, which shows the home screen's button.
    if (await page.getByRole("button", { name: "More" }).isVisible()) {
      await page.getByRole("button", { name: "More" }).click();
      await page.getByRole("menuitem", { name: "Choose a file" }).click();
    }
    await page.getByRole("button", { name: "Choose a file" }).click();
    await (await chooser).setFiles(
      typeof file === "string" ? file : { name: file.name, mimeType: "application/pdf", buffer: file.buffer },
    );
    await page.locator(".web-viewer canvas").first().waitFor();
    if (!(await page.getByRole("tab", { name: "Outline" }).isVisible())) {
      await page.getByRole("button", { name: "Sidebar" }).click();
    }
    await page.getByRole("tab", { name: "Outline" }).click();
  }

  it("[cap I.01 web round-trip] [cap I.04 web round-trip] adds, renames, nests and moves a bookmark and renames a destination", async () => {
    const watched = await load(browser, origin, "/", NO_PICKERS, {
      ignoreHTTPSErrors: true,
      viewport: { width: 1280, height: 900 },
    });
    const { page } = watched;
    await editing(watched, FILE.pathname);
    await page.getByRole("button", { name: "Edit bookmarks" }).click();
    const tree = page.getByRole("treegrid", { name: "Document outline" });

    // Add a bookmark for the current view: it goes last, as "New bookmark".
    await page.getByRole("button", { name: "Add bookmark" }).click();
    await unsaved(page, 1);
    const added = tree.getByRole("row", { name: /New bookmark/ });
    // Rename it in place (F2, the keyboard's way to what a double click does).
    await added.click();
    await page.keyboard.press("F2");
    const field = page.getByLabel("Bookmark title");
    await field.fill("Results");
    await field.press("Enter");
    await unsaved(page, 2);
    // Move it up past Website and back down, then nest it under Website: each move waits for the
    // tree to show the one before.
    const tail = async () =>
      (
        await tree
          .getByRole("row")
          .evaluateAll((rows) =>
            rows.map((r) => `${r.getAttribute("aria-level")} ${r.querySelector(".paper-outline-title")?.textContent}`),
          )
      ).slice(-2);
    await tree.getByRole("row", { name: /Results/ }).click();
    await move(page, "ArrowUp");
    await expect.poll(tail).toEqual(["1 Results", "1 Website"]);
    await move(page, "ArrowDown");
    await expect.poll(tail).toEqual(["1 Website", "1 Results"]);
    await move(page, "ArrowRight");
    await unsaved(page, 5);

    // Rename a named destination a bookmark uses.
    await page.getByRole("button", { name: "Named destinations" }).click();
    const dialog = page.getByRole("dialog", { name: "Named destinations" });
    await dialog.getByRole("button", { name: "Rename chapter-2" }).click();
    const name = dialog.getByRole("textbox", { name: "New name for chapter-2" });
    await name.fill("methods");
    await name.press("Enter");
    await unsaved(page, 6);
    await expect.poll(() => dialog.getByRole("row", { name: /methods/ }).textContent()).toContain("1 bookmark");
    await dialog.getByRole("button", { name: "Done" }).click();

    const download = page.waitForEvent("download");
    await page.getByRole("button", { name: "Download", exact: true }).click();
    const saved = new Uint8Array(readFileSync(await (await download).path()));

    using doc = Document.open(saved);
    const outline = flatten(doc.outline());
    expect(outline.slice(-2)).toEqual([
      [0, "Website"],
      [1, "Results"],
    ]);
    expect(doc.namedDestinations().map((d) => d.name)).toContain("methods");
    expect(doc.namedDestinations().map((d) => d.name)).not.toContain("chapter-2");
    using original = Document.open(new Uint8Array(readFileSync(FILE)));
    expect(doc.namedDestination("methods")?.page).toBe(original.namedDestination("chapter-2")?.page);

    // Reopened, the reader shows the edited outline.
    await editing(watched, { name: "links-and-outlines.pdf", buffer: Buffer.from(saved) });
    await page
      .getByRole("treegrid", { name: "Document outline" })
      .getByRole("row", { name: /Results/ })
      .waitFor();
    await clean(watched);
  });
});
