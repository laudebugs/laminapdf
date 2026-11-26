// The Form data menu (Paper P03), in this run's browser, with no file pickers, so every export is
// a download and every import comes from a file chooser. Answers are exported as JSON, the form
// reset, the file imported back in one undo step, and the form flattened, saved and opened again
// in pdf.js with the answers drawn and the fillable fields gone. On a file a signature locks, reset and
// flatten leave the locked fields and say so; on a certified file, flatten is the conflict
// question and needs Flatten anyway. A file that isn't form data imports nothing. Each dialog
// passes axe.

import { mkdtempSync, readFileSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { getDocument } from "pdfjs-dist/legacy/build/pdf.mjs";
import type { Browser, Download, Page } from "playwright";
import { afterAll, beforeAll, describe, expect, inject, it } from "vitest";
import { BROWSER, launch } from "./browser.ts";
import { clean, load, serious, type Watched } from "./reader.ts";

const origin = inject("origin");
let browser: Browser;
const FIELDS = new URL("../../../fixtures/forms/fields.pdf", import.meta.url);
const LOCKED = new URL("../../../fixtures/signed/form-locked-include.pdf", import.meta.url);
const CERTIFIED = new URL("../../../fixtures/signed/form-certified-p2.pdf", import.meta.url);

/** The reader, wide, with `file` open in Fill & Sign and its fields drawn. */
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
  await page.locator(".lamina-form-field").first().waitFor({ timeout: 20_000 });
  return watched;
}

/** The field control named `name`. */
const field = (page: Page, name: string) => page.getByLabel(name, { exact: true });

/** Opens the Form data menu and chooses `item`. */
async function choose(page: Page, item: string): Promise<void> {
  await page.getByRole("button", { name: "Form data", exact: true }).click();
  await page.getByRole("menuitem", { name: item }).click();
}

/** Saves the document (a download, with no pickers) and returns the bytes. */
async function save(page: Page): Promise<Uint8Array> {
  const download: Promise<Download> = page.waitForEvent("download");
  await page.getByRole("button", { name: "Download", exact: true }).click();
  return new Uint8Array(readFileSync(await (await download).path()));
}

/** The widgets pdf.js finds in `bytes`, and the text its pages draw. */
async function drawn(bytes: Uint8Array): Promise<{ widgets: string[]; text: string }> {
  const task = getDocument({ data: bytes.slice(), verbosity: 0 });
  const doc = await task.promise;
  try {
    const widgets: string[] = [];
    let text = "";
    for (let n = 1; n <= doc.numPages; n++) {
      const page = await doc.getPage(n);
      const annotations = (await page.getAnnotations()) as { subtype: string; fieldName?: string }[];
      for (const a of annotations) if (a.subtype === "Widget") widgets.push(a.fieldName ?? "");
      const content = await page.getTextContent();
      text += content.items.map((item) => ("str" in item ? item.str : "")).join(" ");
    }
    return { widgets, text };
  } finally {
    await task.destroy();
  }
}

/** Exports the answers as JSON through the dialog; returns the file's parsed fields. */
async function exportJson(page: Page, filledOnly: boolean): Promise<{ path: string; fields: Record<string, unknown> }> {
  await choose(page, "Export form data…");
  const dialog = page.getByRole("dialog", { name: "Export form data" });
  await dialog.waitFor();
  await dialog.getByRole("radio", { name: filledOnly ? /^Filled fields only/ : /^All fields/ }).check();
  const download: Promise<Download> = page.waitForEvent("download");
  await dialog.getByRole("button", { name: "Export…" }).click();
  const path = await (await download).path();
  const { fields } = JSON.parse(readFileSync(path, "utf8")) as { fields: Record<string, unknown> };
  return { path, fields };
}

/** Imports the file at `path` through the Form data menu. */
async function importFile(page: Page, path: string): Promise<void> {
  const chooser = page.waitForEvent("filechooser");
  await choose(page, "Import form data…");
  await (await chooser).setFiles(path);
}

beforeAll(async () => {
  browser = await launch();
});

afterAll(async () => {
  await browser?.close();
});

describe(`Form data (${BROWSER})`, () => {
  it("[cap E.11 web round-trip] [cap E.13 web round-trip] [cap E.14 web round-trip] exports, resets, imports back in one step, flattens, and pdf.js reads the answers drawn into the pages with the fields gone", async () => {
    const watched = await fill(FIELDS);
    const { page } = watched;
    await field(page, "Your name").fill("Grace Hopper");
    await field(page, "Your name").press("Tab");
    await expect.poll(() => page.locator(".paper-top-bar-save-state").textContent()).toBe("1 unsaved change");

    const exported = await exportJson(page, true);
    expect(Object.values(exported.fields)).toContain("Grace Hopper");

    // Reset asks, names the count, and puts every answer back to the form's default.
    await choose(page, "Reset form…");
    const reset = page.getByRole("alertdialog", { name: /^Reset all \d+ fields\?/ });
    await reset.waitFor();
    await reset.getByRole("button", { name: "Reset form" }).click();
    await expect.poll(() => field(page, "Your name").inputValue()).toBe("");

    // Importing the file puts the answer back, and Undo takes the whole import out again.
    await importFile(page, exported.path);
    await expect.poll(() => field(page, "Your name").inputValue()).toBe("Grace Hopper");
    await page.getByRole("button", { name: "Undo", exact: true }).first().click();
    await expect.poll(() => field(page, "Your name").inputValue()).toBe("");
    await importFile(page, exported.path);
    await expect.poll(() => field(page, "Your name").inputValue()).toBe("Grace Hopper");

    // Flatten draws the answers into the pages and removes the fields.
    await choose(page, "Flatten form…");
    const flatten = page.getByRole("alertdialog", { name: "Flatten the form?" });
    await flatten.waitFor();
    await flatten.getByRole("button", { name: /^Flatten \d+ fields?$/ }).click();
    await expect.poll(() => page.locator(".lamina-form-field").count()).toBe(0);
    const saved = await drawn(await save(page));
    // What stays is what a flatten never draws: the signature field, a button with no appearance and a field with no type.
    expect(saved.widgets.filter((name) => !["submit", "signature", "untyped"].includes(name))).toEqual([]);
    expect(saved.text).toContain("Grace Hopper");
    await clean(watched);
  });

  it("leaves the fields a signature locks as they are on reset, and says so", async () => {
    const watched = await fill(LOCKED);
    const { page } = watched;
    await field(page, "address").fill("1 New Road");
    await field(page, "address").press("Tab");
    await choose(page, "Reset form…");
    const reset = page.getByRole("alertdialog", { name: /^Reset all \d+ fields\?/ });
    await reset.waitFor();
    expect(await reset.textContent()).toMatch(/locked by a signature/);
    await reset.getByRole("button", { name: "Reset form" }).click();
    await expect.poll(() => field(page, "address").inputValue()).toBe("");
    expect(await field(page, "Your name").inputValue()).toBe("Ada Lovelace");
    // Flatten leaves them too, and the dialog says so.
    await choose(page, "Flatten form…");
    const flatten = page.getByRole("alertdialog", { name: "Flatten the form?" });
    await flatten.waitFor();
    expect(await flatten.textContent()).toMatch(/locked by a signature .* left as/);
    await flatten.getByRole("button", { name: "Cancel" }).click();
    await clean(watched);
  });

  it("asks before flattening a certified file, which needs Flatten anyway", async () => {
    const watched = await fill(CERTIFIED);
    const { page } = watched;
    await choose(page, "Flatten form…");
    const flatten = page.getByRole("alertdialog", { name: "Flatten the form?" });
    await flatten.waitFor();
    expect(await flatten.textContent()).toMatch(/certif/i);
    await flatten.getByRole("button", { name: "Cancel" }).click();
    expect(await page.locator(".lamina-form-field").count()).toBeGreaterThan(0);
    await choose(page, "Flatten form…");
    await flatten.getByRole("button", { name: "Flatten anyway" }).click();
    await expect.poll(() => page.locator(".lamina-form-field").count()).toBe(0);
    await clean(watched);
  });

  it("imports nothing from a file that isn't form data, and says why", async () => {
    const watched = await fill(FIELDS);
    const { page } = watched;
    const path = join(mkdtempSync(join(tmpdir(), "form-data-")), "broken.xfdf");
    writeFileSync(path, '<?xml version="1.0"?><xfdf xmlns="http://ns.adobe.com/xfdf/"><fields><field name="a">');
    await importFile(page, path);
    const dialog = page.getByRole("dialog", { name: "Can’t import broken.xfdf" });
    await dialog.waitFor();
    expect(await serious(page)).toEqual([]);
    await dialog.getByRole("button", { name: "OK" }).click();
    await expect.poll(() => page.locator(".paper-top-bar-save-state").textContent()).not.toMatch(/unsaved/);
    await clean(watched);
  });

  it("passes axe with the menu and each dialog open", async () => {
    const watched = await fill(FIELDS);
    const { page } = watched;
    await page.getByRole("button", { name: "Form data", exact: true }).click();
    await page.getByRole("menu", { name: "Form data" }).waitFor();
    expect(await serious(page)).toEqual([]);
    await page.keyboard.press("Escape");
    for (const [item, dialog] of [
      ["Export form data…", page.getByRole("dialog", { name: "Export form data" })],
      ["Reset form…", page.getByRole("alertdialog", { name: /^Reset all/ })],
      ["Flatten form…", page.getByRole("alertdialog", { name: "Flatten the form?" })],
    ] as const) {
      await choose(page, item);
      await dialog.waitFor();
      expect(await serious(page)).toEqual([]);
      await page.keyboard.press("Escape");
      await dialog.waitFor({ state: "hidden" });
    }
    await clean(watched);
  });
});
