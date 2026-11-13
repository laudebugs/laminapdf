// Links in the built reader (todo E3c, Paper O12), in this run's browser: the top bar's Link tool
// draws a link over the tagged report's chart and points it at page 2; a `javascript:` address is
// refused in place; Make link, on the selected word "Widgets", links it to a web address. Saved
// (Save downloads where the browser can't write files) and read back by Lamina PDF's SDK, the
// page has both links, and the link on text is a Link element of the structure tree. No CSP
// violation or other origin throughout.

import { readFileSync } from "node:fs";
import { Document, init } from "@laminapdf/sdk";
import type { Browser, Page } from "playwright";
import { afterAll, beforeAll, describe, expect, inject, it } from "vitest";
import { BROWSER, launch } from "./browser.ts";
import { clean, load } from "./reader.ts";

const origin = inject("origin");
const FILE = new URL("../../../fixtures/golden/tagged-report.pdf", import.meta.url);

/** No pickers: Save becomes Download, in every engine, so the saved bytes are the download's. */
const NO_PICKERS = {
  script: () => {
    Reflect.deleteProperty(window, "showOpenFilePicker");
    Reflect.deleteProperty(window, "showSaveFilePicker");
  },
  arg: [],
};

/** Waits until the top bar counts `count` unsaved changes. */
async function unsaved(page: Page, count: number): Promise<void> {
  const text = count === 1 ? "1 unsaved change" : `${count} unsaved changes`;
  await expect.poll(() => page.locator(".paper-top-bar-save-state").textContent()).toBe(text);
}

/** The viewport point of page-space point (`x`, `y`) on the first page (595 × 842 points). */
async function onPage(page: Page, x: number, y: number): Promise<{ x: number; y: number }> {
  const box = await page.locator(".web-viewer canvas").first().boundingBox();
  if (box === null) throw new Error("the first page isn't shown");
  return { x: box.x + (x / 595.276) * box.width, y: box.y + (y / 841.89) * box.height };
}

describe(`links in ${BROWSER}`, () => {
  let browser: Browser;

  beforeAll(async () => {
    await init({ extract: true });
    browser = await launch();
  });

  afterAll(async () => {
    await browser?.close();
  });

  it("[cap I.02 web round-trip] [cap I.03 web round-trip] [cap A.09 web round-trip] draws a link to a page, refuses a script address and makes a web link on text", async () => {
    const watched = await load(browser, origin, "/", NO_PICKERS, {
      ignoreHTTPSErrors: true,
      viewport: { width: 1280, height: 900 },
    });
    const { page } = watched;
    const chooser = page.waitForEvent("filechooser");
    await page.getByRole("button", { name: "Choose a file" }).click();
    await (await chooser).setFiles(FILE.pathname);
    await page.locator(".web-viewer canvas").first().waitFor();

    // The Link tool: a drag over the chart draws a link, which goes to page 2 once it has a target.
    const tool = page.getByRole("button", { name: "Link", exact: true });
    await tool.click();
    const from = await onPage(page, 60, 150);
    const to = await onPage(page, 290, 230);
    await page.mouse.move(from.x, from.y);
    await page.mouse.down();
    await page.mouse.move(to.x, to.y, { steps: 5 });
    await page.mouse.up();
    const panel = page.getByRole("dialog", { name: "Link" });
    // As a person does: the label, since the radio's input sits hidden under its circle.
    await panel.getByText("A page in this file").click();
    const number = panel.getByRole("textbox", { name: "Page number" });
    await number.fill("2");
    await number.press("Enter");
    await unsaved(page, 1);
    await panel.getByRole("button", { name: "Done" }).click();
    await tool.click();

    // Make link on the selected word; a script address is refused and writes nothing.
    await page.locator(".web-viewer").getByText("Widgets", { exact: false }).first().dblclick();
    await page.getByRole("button", { name: "Make link" }).click();
    const address = panel.getByRole("textbox", { name: "Web address" });
    await address.fill("javascript:alert(1)");
    await address.press("Enter");
    await expect.poll(() => panel.getByText("This address would run a script").count()).toBe(1);
    await address.fill("https://example.com/widgets");
    await address.press("Enter");
    await unsaved(page, 2);
    await panel.getByRole("button", { name: "Done" }).click();

    const download = page.waitForEvent("download");
    await page.getByRole("button", { name: "Download", exact: true }).click();
    const saved = new Uint8Array(readFileSync(await (await download).path()));

    using doc = Document.open(saved);
    const links = doc.pageLinks(0);
    const goes = links.map((l) => (l.kind === "uri" ? l.uri : l.kind === "dest" ? `page ${l.target?.page}` : l.kind));
    expect(goes).toEqual(expect.arrayContaining(["page 1", "https://example.com/widgets"]));
    const kinds = JSON.stringify(doc.structureTree());
    expect(kinds).toContain('"Link"');
    await clean(watched);
  });
});
