// XFA forms (E8 plan §2.4), in this run's browser, with no file pickers, so every save is a
// download. A dynamic form says it needs Adobe's apps and has nothing to fill; a static one fills
// as usual, its first change takes the XFA version out in the same undo step, and pdf.js then
// shows the fields' answers; a signed one keeps its XFA version and says so.

import { readFileSync } from "node:fs";
import { getDocument } from "pdfjs-dist/legacy/build/pdf.mjs";
import type { Browser, Download, Page } from "playwright";
import { afterAll, beforeAll, describe, expect, inject, it } from "vitest";
import { BROWSER, launch } from "./browser.ts";
import { clean, load, serious, type Watched } from "./reader.ts";

const origin = inject("origin");
let browser: Browser;
const DYNAMIC = new URL("../../../fixtures/forms/xfa-dynamic.pdf", import.meta.url);
const STATIC = new URL("../../../fixtures/forms/xfa-static.pdf", import.meta.url);

/** The reader, wide, with `file` open in Fill & Sign. */
async function fill(file: URL): Promise<Watched> {
  const watched = await load(
    browser,
    origin,
    "/",
    {
      script: () => {
        Reflect.deleteProperty(window, "showOpenFilePicker");
        Reflect.deleteProperty(window, "showSaveFilePicker");
      },
      arg: undefined,
    } as never,
    { ignoreHTTPSErrors: true, viewport: { width: 1400, height: 900 }, bypassCSP: true },
  );
  const { page } = watched;
  const chooser = page.waitForEvent("filechooser");
  await page.getByRole("button", { name: "Choose a file" }).click();
  await (await chooser).setFiles(file.pathname);
  await page.locator(".web-viewer canvas").first().waitFor({ timeout: 30_000 });
  await page.getByRole("button", { name: "Fill & Sign", exact: true }).click();
  await page.getByRole("toolbar", { name: "Fill & Sign tools" }).waitFor();
  return watched;
}

/** The XFA notice under the toolbar. */
const notice = (page: Page) => page.getByRole("region", { name: "XFA form" });

/** The top bar's save state, once it says `text`. */
async function saveState(page: Page, text: string): Promise<void> {
  await expect.poll(() => page.locator(".paper-top-bar-save-state").textContent(), { timeout: 20_000 }).toBe(text);
}

/** Saves (a download, with no pickers) and returns the bytes. */
async function save(page: Page): Promise<Uint8Array> {
  const download: Promise<Download> = page.waitForEvent("download");
  await page.getByRole("button", { name: "Download", exact: true }).click();
  return new Uint8Array(readFileSync(await (await download).path()));
}

/** What pdf.js makes of the file: whether it is pure XFA, and the values of the fields it lists. */
async function pdfjs(bytes: Uint8Array): Promise<{ pure: boolean; values: Record<string, unknown> }> {
  const task = getDocument({ data: bytes.slice(), verbosity: 0, enableXfa: true });
  const doc = await task.promise;
  try {
    const objects = ((await doc.getFieldObjects()) ?? {}) as Record<string, { value?: unknown }[]>;
    const values = Object.fromEntries(Object.entries(objects).map(([name, [first]]) => [name, first?.value]));
    return { pure: doc.isPureXfa, values };
  } finally {
    await task.destroy();
  }
}

beforeAll(async () => {
  browser = await launch();
});

afterAll(async () => {
  await browser?.close();
});

describe(`XFA forms (${BROWSER})`, () => {
  it("[cap E.17 web read] says a dynamic form needs Adobe's apps, in the notice, the Fields tab and the status bar", async () => {
    const watched = await fill(DYNAMIC);
    const { page } = watched;
    const strip = notice(page);
    await strip.waitFor({ timeout: 20_000 });
    expect(await strip.textContent()).toContain("This form needs Adobe Acrobat or Adobe Reader.");
    expect(await strip.textContent()).toContain("ask the sender for a standard PDF form");
    if ((await page.getByRole("tab", { name: "Fields" }).count()) === 0) {
      await page.getByRole("button", { name: "Sidebar" }).click();
    }
    await page.getByRole("tab", { name: "Fields" }).click();
    await expect
      .poll(() => page.locator(".paper-form-fields-empty").textContent())
      .toContain("Nothing to fill here");
    await expect
      .poll(() => page.locator(".paper-reader-status").textContent())
      .toContain("XFA form · needs Adobe Acrobat");
    expect(await serious(page)).toEqual([]);
    await clean(watched);
  });

  it("[cap E.17 web read] fills a static form's field, takes the XFA version out in the same undo step, and pdf.js shows the answer", async () => {
    const watched = await fill(STATIC);
    const { page } = watched;
    await page.locator(".lamina-form-field").first().waitFor({ timeout: 20_000 });
    const strip = notice(page);
    await expect.poll(() => strip.textContent()).toContain("This form also has an XFA version");
    const name = page.getByLabel("name", { exact: true });
    await name.fill("Grace Hopper");
    await name.press("Tab");
    await saveState(page, "1 unsaved change");
    await expect.poll(() => strip.textContent()).toContain("The XFA version of this form was removed.");
    // One undo step took both: the answer and the XFA version.
    await page.locator(".paper-top-bar").getByRole("button", { name: /^Undo/ }).click();
    await saveState(page, "No unsaved changes");
    await expect.poll(() => strip.textContent()).toContain("This form also has an XFA version");
    expect(await name.inputValue()).toBe("Ada");
    // The notice's own Undo does the same after a new answer.
    await name.fill("Grace Hopper");
    await name.press("Tab");
    await expect.poll(() => strip.textContent()).toContain("The XFA version of this form was removed.");
    await saveState(page, "1 unsaved change");
    const { pure, values } = await pdfjs(await save(page));
    expect({ pure, name: values.name }).toEqual({ pure: false, name: "Grace Hopper" });
    await strip.getByRole("button", { name: "Undo", exact: true }).click();
    await saveState(page, "No unsaved changes");
    await expect.poll(() => strip.textContent()).toContain("This form also has an XFA version");
    await clean(watched);
  });
});
