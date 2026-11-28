// Signature images (Paper G01), in this run's browser, with no file pickers, so every save is a
// download. A picture of a signature is drawn, typed or uploaded in a dialog, placed on the page
// in view as one undo step, kept in a library on this device, and read back by pdf.js from the
// saved file as a stamp or as ink. Everything the dialog says calls it a picture, never a digital
// signature; on a file certified with no changes allowed, the picture can't become page content.
// The dialog passes axe.

import { readFileSync } from "node:fs";
import { env } from "node:process";
import { getDocument } from "pdfjs-dist/legacy/build/pdf.mjs";
import type { Browser, Download, Page } from "playwright";
import { afterAll, beforeAll, describe, expect, inject, it } from "vitest";
import { launch } from "./browser.ts";
import { clean, load, serious, type Watched } from "./reader.ts";

const origin = inject("origin");
/** Where screenshots go, when asked for (LAMINA_PDF_E2E_SHOTS=<dir>). */
const SHOTS = env.LAMINA_PDF_E2E_SHOTS;
let browser: Browser;
const HELLO = new URL("../../../fixtures/corpus/hello.pdf", import.meta.url);
const CERTIFIED_P1 = new URL("../../../fixtures/signed/certified-p1.pdf", import.meta.url);
const SIGNATURE_PNG = new URL("../../../fixtures/content/add/logo-alpha.png", import.meta.url);

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

/** A screenshot of the page, when screenshots were asked for. */
async function shot(page: Page, name: string): Promise<void> {
  if (SHOTS === undefined) return;
  // Toasts and the redraw after an edit settle first.
  await page.waitForTimeout(800);
  await page.screenshot({ path: `${SHOTS}/signatures-${name}.png` });
}

/** Opens the dialog from the toolbar and returns it. */
async function openDialog(page: Page) {
  await page.getByRole("button", { name: "Add a signature image", exact: true }).click();
  const dialog = page.getByRole("dialog", { name: "Add a signature image" });
  await dialog.waitFor();
  return dialog;
}

/** Draws one stroke across the pad. */
async function draw(page: Page): Promise<void> {
  const pad = page.getByRole("img", { name: /Drawing area/ });
  const box = await pad.boundingBox();
  if (box === null) throw new Error("no pad");
  await page.mouse.move(box.x + 40, box.y + 100);
  await page.mouse.down();
  await page.mouse.move(box.x + 120, box.y + 60, { steps: 8 });
  await page.mouse.move(box.x + 220, box.y + 110, { steps: 8 });
  await page.mouse.up();
}

/** The count of unsaved changes the top bar says, once it says `count`. */
async function changes(page: Page, count: number): Promise<void> {
  const text = count === 1 ? "1 unsaved change" : `${count} unsaved changes`;
  await expect.poll(() => page.locator(".paper-top-bar-save-state").textContent(), { timeout: 20_000 }).toBe(text);
}

/** Saves (a download, with no pickers) and returns the bytes. */
async function save(page: Page): Promise<Uint8Array> {
  const download: Promise<Download> = page.waitForEvent("download");
  await page.getByRole("button", { name: "Download", exact: true }).click();
  return new Uint8Array(readFileSync(await (await download).path()));
}

/** The subtypes of the annotations on page 1 of `bytes`, as pdf.js reads them. */
async function subtypes(bytes: Uint8Array): Promise<string[]> {
  const task = getDocument({ data: bytes.slice(), verbosity: 0 });
  try {
    const doc = await task.promise;
    const annotations = (await (await doc.getPage(1)).getAnnotations()) as { subtype: string }[];
    return annotations.map((a) => a.subtype);
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

describe("signature images", () => {
  it("[cap F.02 web round-trip] draws a signature, places it as a stamp in one undo step, and pdf.js reads it back", async () => {
    const watched = await fill(HELLO);
    const { page } = watched;
    const dialog = await openDialog(page);
    // An unsigned file offers a picture as part of the page.
    expect(await dialog.getByRole("radio", { name: "Part of the page" }).isDisabled()).toBe(false);
    const place = dialog.getByRole("button", { name: "Place signature" });
    expect(await place.isDisabled()).toBe(true);
    await shot(page, "empty");
    await draw(page);
    await shot(page, "drawn");
    expect(await place.isDisabled()).toBe(false);
    await place.click();
    await dialog.waitFor({ state: "hidden" });
    await changes(page, 1);
    await shot(page, "placed");
    expect(await subtypes(await save(page))).toContain("Stamp");
    await clean(watched);
  });

  it("[cap F.01 web round-trip] uploads a picture, places it, and finds it among the saved signatures next time", async () => {
    const watched = await fill(HELLO);
    const { page } = watched;
    let dialog = await openDialog(page);
    await dialog.getByRole("tab", { name: "Upload" }).click();
    await dialog.locator('input[type="file"]').setInputFiles(SIGNATURE_PNG.pathname);
    await dialog.getByRole("button", { name: "Place signature" }).click();
    await dialog.waitFor({ state: "hidden" });
    await changes(page, 1);
    dialog = await openDialog(page);
    await dialog.getByRole("tab", { name: "Saved (1)" }).click();
    expect(await dialog.getByRole("list").locator("li").count()).toBe(1);
    await dialog.getByRole("button", { name: "Cancel" }).click();
    await clean(watched);
  });

  it("[cap F.02 web round-trip] types a name in each style and places it as a stamp", async () => {
    const watched = await fill(HELLO);
    const { page } = watched;
    const dialog = await openDialog(page);
    await dialog.getByRole("tab", { name: "Type" }).click();
    expect(await dialog.getByRole("button", { name: "Place signature" }).isDisabled()).toBe(true);
    await dialog.getByLabel("Your name").fill("Amara Okafor");
    await shot(page, "type");
    await dialog.getByText("AMARA OKAFOR", { exact: true }).click();
    await dialog.getByRole("button", { name: "Place signature" }).click();
    await dialog.waitFor({ state: "hidden" });
    await changes(page, 1);
    expect(await subtypes(await save(page))).toContain("Stamp");
    await clean(watched);
  });

  it("says it is a picture and not a digital signature, and passes axe", async () => {
    const watched = await fill(HELLO);
    const { page } = watched;
    const dialog = await openDialog(page);
    const text = ((await dialog.textContent()) ?? "").replace(/\s+/g, " ");
    expect(text).toContain("This is not a digital signature.");
    expect(text.replace("not a digital signature", "")).not.toMatch(/digital signature/i);
    expect(await serious(page)).toEqual([]);
    await clean(watched);
  });

  it("keeps a picture out of the page's content when the file is certified with no changes allowed", async () => {
    const watched = await fill(CERTIFIED_P1);
    const { page } = watched;
    const dialog = await openDialog(page);
    expect(await dialog.getByRole("radio", { name: "Part of the page" }).isDisabled()).toBe(true);
    await dialog.getByRole("button", { name: "Cancel" }).click();
    await clean(watched);
  });
});
