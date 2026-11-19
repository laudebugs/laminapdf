// Fill & Sign (Paper P01, E8e), in this run's browser, with no file pickers, so every save is a
// download. Every field type of `fields.pdf` is filled from the keyboard, Tab crossing from page 1
// to page 2 in the form's order, and the file saved and opened again in pdf.js holds each value.
// A required field says so, a read-only one keeps its value, and each field visit is one undo
// step. On a file whose signature locks two fields, those draw read-only, name the signature and
// ask before a change; on a file certified with no changes allowed, the banner shows on entering
// the mode. The mode passes axe.

import { readFileSync } from "node:fs";
import { env } from "node:process";
import { getDocument } from "pdfjs-dist/legacy/build/pdf.mjs";
import type { Browser, Download, Page } from "playwright";
import { afterAll, beforeAll, describe, expect, inject, it } from "vitest";
import { BROWSER, launch } from "./browser.ts";
import { clean, load, serious, type Watched } from "./reader.ts";

const origin = inject("origin");
let browser: Browser;
const FIELDS = new URL("../../../fixtures/forms/fields.pdf", import.meta.url);
const LOCKED = new URL("../../../fixtures/signed/form-locked-include.pdf", import.meta.url);
const CERTIFIED_P1 = new URL("../../../fixtures/signed/certified-p1.pdf", import.meta.url);
/** Where screenshots of the mode go, when asked for (LAMINA_PDF_E2E_SHOTS=<dir>). */
const SHOTS = env.LAMINA_PDF_E2E_SHOTS;

/** The reader, wide, with `file` open in Fill & Sign and, when it has any, its fields drawn. */
async function fill(file: URL, fields = true): Promise<Watched> {
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
  if (fields) await page.locator(".lamina-form-field").first().waitFor({ timeout: 20_000 });
  return watched;
}

/** The field control named `name`. */
const field = (page: Page, name: string) => page.getByLabel(name, { exact: true });

/** The accessible name of the element that has the focus. */
function focusedName(page: Page): Promise<string | null> {
  return page.evaluate(() => document.activeElement?.getAttribute("aria-label") ?? null);
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

/** Each field's value as pdf.js reads the saved file's widgets, by field name. */
async function pdfjsValues(bytes: Uint8Array): Promise<Record<string, unknown>> {
  const task = getDocument({ data: bytes.slice(), verbosity: 0 });
  const doc = await task.promise;
  try {
    const values: Record<string, unknown> = {};
    for (let n = 1; n <= doc.numPages; n++) {
      const annotations = (await (await doc.getPage(n)).getAnnotations()) as {
        subtype: string;
        fieldName?: string;
        fieldValue?: unknown;
      }[];
      for (const a of annotations) {
        if (a.subtype === "Widget" && a.fieldName !== undefined && !(a.fieldName in values)) {
          values[a.fieldName] = a.fieldValue;
        }
      }
    }
    return values;
  } finally {
    await task.destroy();
  }
}

/** Opens the rail's Fields tab and answers its list. */
async function fieldsTab(page: Page) {
  if ((await page.getByRole("tab", { name: "Fields" }).count()) === 0) {
    await page.getByRole("button", { name: "Sidebar" }).click();
  }
  await page.getByRole("tab", { name: "Fields" }).click();
  return page.getByRole("navigation", { name: "Fields" });
}

/** A screenshot of the page, when screenshots were asked for. */
async function shot(page: Page, name: string): Promise<void> {
  if (SHOTS !== undefined) await page.screenshot({ path: `${SHOTS}/forms-${name}-${BROWSER}.png` });
}

beforeAll(async () => {
  browser = await launch();
});

afterAll(async () => {
  await browser?.close();
});

describe(`Fill & Sign (${BROWSER})`, () => {
  it("[cap E.01 web write] [cap E.02 web write] [cap E.03 web write] [cap E.09 web read] fills every field type from the keyboard across both pages, and pdf.js reads each value", async () => {
    const watched = await fill(FIELDS);
    const { page } = watched;
    await shot(page, "empty");
    // Page 1: a text field, a multi-line one, a check box, two radio groups, an editable combo
    // box and a multi-select list box, each reached with Tab in the form's order.
    await field(page, "Your name").fill("Grace Hopper");
    await page.keyboard.press("Tab");
    expect(await focusedName(page)).toBe("address");
    await changes(page, 1);
    await page.keyboard.press("End");
    await page.keyboard.type(", Arlington");
    await page.keyboard.press("Tab");
    await changes(page, 2);
    expect(await focusedName(page)).toBe("agree");
    await page.keyboard.press("Space");
    await changes(page, 3);
    // Each group's other button: the first `colour` button is already the value.
    // A click, then the step: the redraw after the edit draws the buttons anew.
    await page.getByRole("radio", { name: "colour" }).nth(1).click();
    await changes(page, 4);
    await page.getByRole("radio", { name: "size" }).first().click();
    await changes(page, 5);
    await field(page, "country").fill("UK");
    await field(page, "country").press("Tab");
    await changes(page, 6);
    await field(page, "toppings").selectOption(["Ham", "Olives"]);
    await changes(page, 7);
    await shot(page, "page1");
    // Tab from the last field of page 1 reaches page 2's first: the page scrolls to it.
    await field(page, "person.last").focus();
    await page.keyboard.press("Tab");
    expect(await focusedName(page)).toBe("total");
    await field(page, "notes").fill("Bring the milk");
    await field(page, "notes").press("Tab");
    await changes(page, 8);
    await field(page, "pin").fill("1234");
    await field(page, "pin").press("Tab");
    await changes(page, 9);
    await field(page, "code").fill("AB12");
    await field(page, "code").press("Tab");
    await changes(page, 10);
    const values = await pdfjsValues(await save(page));
    expect({
      name: values.name,
      address: values.address,
      agree: values.agree,
      country: values.country,
      toppings: values.toppings,
      notes: values.notes,
      pin: values.pin,
      code: values.code,
    }).toEqual({
      name: "Grace Hopper",
      address: "12 Main Street, Arlington",
      agree: "Off",
      // pdf.js reads a combo box's value as a list, as it does a list box's.
      country: ["UK"],
      toppings: ["Ham", "Olives"],
      notes: "Bring the milk",
      pin: "1234",
      code: "AB12",
    });
    await clean(watched);
  });

  it("[cap E.06 web round-trip] marks the required field, keeps a read-only one, and makes each field visit one undo step", async () => {
    const watched = await fill(FIELDS);
    const { page } = watched;
    const name = field(page, "Your name");
    expect(await name.getAttribute("aria-required")).toBe("true");
    const required = page.locator(".paper-form-required");
    await expect.poll(() => required.textContent()).toBe("1 of 1 required fields filled");
    // A read-only field takes no typing.
    const first = field(page, "person.first");
    await first.click();
    await page.keyboard.type("Ada");
    expect(await first.inputValue()).toBe("Grace");
    // Four keystrokes in one visit: one step.
    await name.click();
    await name.press("End");
    await page.keyboard.type(" Jr.");
    await name.press("Tab");
    await changes(page, 1);
    // Emptied, the required field counts as unfilled.
    await name.fill("");
    await name.press("Tab");
    await changes(page, 2);
    await expect.poll(() => required.textContent()).toBe("0 of 1 required fields filled");
    const values = await pdfjsValues(await save(page));
    expect([values.name, values["person.first"]]).toEqual(["", "Grace"]);
    await clean(watched);
  });

  it("draws a field a signature locks read-only, names the signature and asks before a change", async () => {
    const watched = await fill(LOCKED);
    const { page } = watched;
    const name = field(page, "Your name");
    await expect.poll(() => name.getAttribute("title")).toBe("Locked by the signature in “Sig1”");
    expect(await name.getAttribute("aria-readonly")).toBe("true");
    // Fields tab: the row says it is locked.
    const row = (await fieldsTab(page)).getByRole("link", { name: /Your name/ });
    await expect.poll(() => row.textContent()).toContain("Locked by the signature in “Sig1”");
    // Typing asks first; Cancel keeps the field as it was.
    await name.click();
    await page.keyboard.type("x");
    const dialog = page.getByRole("alertdialog", { name: "Change a locked field?" });
    await dialog.waitFor();
    await shot(page, "locked");
    await dialog.getByRole("button", { name: "Cancel" }).click();
    expect(await name.inputValue()).toBe("Ada Lovelace");
    // A field the lock doesn't cover changes without asking.
    await field(page, "address").fill("1 New Road");
    await field(page, "address").press("Tab");
    await changes(page, 1);
    await clean(watched);
  });

  it("shows the certified banner on entering the mode on a file that allows no changes", async () => {
    const watched = await fill(CERTIFIED_P1, false);
    const { page } = watched;
    const banner = page.getByRole("region", { name: "Certified file" });
    await banner.waitFor({ timeout: 20_000 });
    expect(await banner.textContent()).toContain("Certified: no changes allowed.");
    await shot(page, "certified");
    await clean(watched);
  });

  it("passes axe in Fill & Sign, with the Fields tab open", async () => {
    const watched = await fill(FIELDS);
    const { page } = watched;
    await (await fieldsTab(page)).waitFor();
    expect(await serious(page)).toEqual([]);
    await clean(watched);
  });
});
