// Editing a document's metadata in the desktop app (E1j): File ▸ Document properties… opens the
// dialog over what the engine reads; each field changed on the Description tab, applied as one
// edit, saved in place, and read back by pdf.js, apart in the Info dictionary and the XMP metadata,
// then by the app itself once the document is reopened; the creation date set in a file that had
// no XMP, and the save setting the modification date; values removed with Remove metadata… gone
// from both. Each test edits a copy of a golden file, which the test process reads as it is on disk.
import { copyFileSync, mkdirSync, readFileSync, rmSync } from "node:fs";
import { join } from "node:path";
import { $, browser, expect } from "@wdio/globals";
import { getDocument } from "pdfjs-dist/legacy/build/pdf.mjs";
import { Key } from "webdriverio";
import { fixture, focusWindow, menuClick, openAndWait } from "../app.ts";
import { workingCopies } from "../paths.ts";

const dir = join(workingCopies, "properties");
const golden = (name: string) => fixture("fixtures", "golden", name);

/** A fresh copy of golden file `name`, to edit and save. */
function copyOf(name: string): string {
  const path = join(dir, name);
  copyFileSync(golden(name), path);
  return path;
}

/** The file's Info entries and XMP properties as pdf.js reads them, apart. */
async function readByPdfJs(path: string): Promise<{ info: Record<string, unknown>; xmp: Record<string, unknown> }> {
  const task = getDocument({ data: new Uint8Array(readFileSync(path)), verbosity: 0 });
  const doc = await task.promise;
  try {
    const { info, metadata } = await doc.getMetadata();
    return {
      info: info as Record<string, unknown>,
      xmp: metadata === null ? {} : Object.fromEntries(metadata as unknown as Iterable<[string, unknown]>),
    };
  } finally {
    await task.destroy();
  }
}

/** A PDF date (`D:20261014102000+05'30'`) as milliseconds since the epoch; `NaN` for anything else. */
function pdfDate(date: string): number {
  const m = /^D:(\d{4})(\d{2})(\d{2})(\d{2})(\d{2})(\d{2})(?:(Z)|([+-])(\d{2})'(\d{2})'?)?$/.exec(date);
  if (m === null) return Number.NaN;
  const [, y, mo, d, h, mi, s, , sign, oh, om] = m;
  const offset = sign === undefined ? 0 : (sign === "-" ? -1 : 1) * (Number(oh) * 60 + Number(om));
  return Date.UTC(Number(y), Number(mo) - 1, Number(d), Number(h), Number(mi), Number(s)) - offset * 60_000;
}

/** The dialog's field labelled `label`: the control its `<label>` names. */
async function field(label: string) {
  const labelled = $(`//*[@role="dialog"]//label[normalize-space(.)=${JSON.stringify(label)}]`);
  await labelled.waitForExist({ timeoutMsg: `no field labelled ${label}` });
  return $(`[id="${await labelled.getAttribute("for")}"]`);
}

/** Replaces the text in field `label` with `value`, by the keyboard. */
async function type(label: string, value: string): Promise<void> {
  const input = await field(label);
  await input.clearValue();
  await input.setValue(value);
}

/**
 * Ticks the removal list's box for `value`. Clicked in the page, as `toggleSetting` does: WebKit's
 * driver can't click the hidden input, nor its label.
 */
async function tick(value: string): Promise<void> {
  await browser.execute((text) => {
    const label = [...globalThis.document.querySelectorAll("label")].find((l) => l.textContent?.includes(text));
    label?.querySelector<HTMLInputElement>('input[type="checkbox"]')?.click();
  }, value);
}

/** Opens Document properties from the File menu and waits for its form. */
async function openProperties(): Promise<void> {
  await menuClick("cmd:file.properties");
  await (await field("Title")).waitForDisplayed({ timeoutMsg: "the properties form didn't show" });
}

/** Applies the form, waits for the edit to count, saves in place, and waits for the save. */
async function applyAndSave(file: string): Promise<void> {
  await $("button=Apply").click();
  await browser.waitUntil(async () => (await browser.getTitle()) === `• ${file} — Lamina PDF`, {
    timeoutMsg: `the edit didn't mark ${file} unsaved`,
  });
  await menuClick("cmd:file.save");
  await browser.waitUntil(async () => (await browser.getTitle()) === `${file} — Lamina PDF`, {
    timeout: 15_000,
    timeoutMsg: `${file} wasn't saved`,
  });
}

/** Closes the shown document's tab and waits for the start screen. */
async function closeTab(): Promise<void> {
  await menuClick("cmd:tab.close");
  await $("button=Choose a file").waitForExist();
}

/** The Description tab's values the first scenario types, as Info writes them. */
const DESCRIPTION = [
  "Quarterly figures",
  "Ada Lovelace; Charles Babbage",
  "Repairs and rent, July to September",
  "pdf, xmp, info, board",
  "Report template 2.3",
  "Lamina PDF",
];

/** The same values as the XMP metadata holds them: the authors as a list. */
const DESCRIPTION_XMP = DESCRIPTION.map((value, i) => (i === 1 ? ["Ada Lovelace", "Charles Babbage"] : value));

/** Types the Description tab's values: every field, a keyword removed and one added. */
async function fillDescription(): Promise<void> {
  await type("Title", "Quarterly figures");
  await type("Author", "Ada Lovelace; Charles Babbage");
  await type("Subject", "Repairs and rent, July to September");
  await $('button[aria-label="Remove keyword lamina"]').click();
  await (await field("Keywords")).setValue("board");
  await browser.keys(Key.Enter);
  await type("Creator", "Report template 2.3");
  await type("Producer", "Lamina PDF");
}

/** The saved file's title, author, subject, keywords, creator and producer, in Info and in XMP, by pdf.js. */
async function savedDescription(path: string): Promise<{ info: unknown[]; xmp: unknown[] }> {
  const { info, xmp } = await readByPdfJs(path);
  return {
    info: [info.Title, info.Author, info.Subject, info.Keywords, info.Creator, info.Producer],
    xmp: ["dc:title", "dc:creator", "dc:description", "pdf:keywords", "xmp:creatortool", "pdf:producer"].map(
      (key) => xmp[key],
    ),
  };
}

before(() => {
  rmSync(dir, { recursive: true, force: true });
  mkdirSync(dir, { recursive: true });
});

describe("editing a document's metadata", () => {
  it("[cap J.01 desktop round-trip] [cap J.03 desktop round-trip] sets title, authors, subject, keywords, creator and producer in Info and XMP", async () => {
    const path = copyOf("metadata.pdf");
    await openAndWait(path);
    await focusWindow();
    await openProperties();
    await fillDescription();
    await applyAndSave("metadata.pdf");
    expect(await savedDescription(path)).toEqual({ info: DESCRIPTION, xmp: DESCRIPTION_XMP });
    // Reopened, the dialog shows the saved values.
    await closeTab();
    await openAndWait(path);
    await openProperties();
    expect([await (await field("Title")).getValue(), await (await field("Author")).getValue()]).toEqual([
      "Quarterly figures",
      "Ada Lovelace; Charles Babbage",
    ]);
    await browser.keys(Key.Escape);
    await closeTab();
  });

  it("[cap J.02 desktop round-trip] sets the creation date, and the save sets the modification date, in a file that had no XMP", async () => {
    const path = copyOf("links-and-outlines.pdf");
    const began = Date.now();
    await openAndWait(path);
    await openProperties();
    // A date field takes typed digits differently in each engine: its value is set as React reads it.
    const created = await field("Created");
    await browser.execute(
      (input, value) => {
        const set = Object.getOwnPropertyDescriptor(HTMLInputElement.prototype, "value")?.set;
        set?.call(input, value);
        input.dispatchEvent(new Event("input", { bubbles: true }));
      },
      created,
      "2026-10-14T10:20",
    );
    await applyAndSave("links-and-outlines.pdf");

    const { info, xmp } = await readByPdfJs(path);
    // In the computer's time zone, whatever it is.
    expect([String(info.CreationDate).slice(0, 16), String(xmp["xmp:createdate"]).slice(0, 19)]).toEqual([
      "D:20261014102000",
      "2026-10-14T10:20:00",
    ]);
    // The save set the modification date, the same instant in Info and XMP, after the test began.
    const modified = pdfDate(String(info.ModDate));
    expect([modified >= Math.floor(began / 1000) * 1000, modified]).toEqual([
      true,
      Date.parse(String(xmp["xmp:modifydate"])),
    ]);
    await closeTab();
  });
});

describe("removing a document's metadata", () => {
  it("[cap J.10 desktop round-trip] removes the values chosen in Remove metadata… from Info and XMP", async () => {
    const path = copyOf("metadata.pdf");
    await openAndWait(path);
    await openProperties();
    await $("button=Remove metadata…").click();
    await $('//*[normalize-space(text())="lamina-build tests"]').waitForDisplayed({
      timeoutMsg: "the removal list didn't show",
    });
    await tick("lamina-build tests");
    await tick("Info and XMP carry the same values");
    await $("button=Mark for removal").click();
    await applyAndSave("metadata.pdf");

    const { info, xmp } = await readByPdfJs(path);
    expect([info.Title, info.Creator, info.Subject, xmp["xmp:creatortool"], xmp["dc:description"]]).toEqual([
      "Metadata golden: Info & XMP <in sync>",
      undefined,
      undefined,
      undefined,
      undefined,
    ]);
    await closeTab();
  });
});
