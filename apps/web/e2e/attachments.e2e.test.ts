// The Attachments tab in the built reader (todo E3f, Paper O14), in this run's browser:
// - a file with attachments gets the tab, which lists each file with its size, kind and the badge
//   naming its relationship, shows the selected one's details and checksum, and downloads its bytes
//   exactly as attached;
// - in Organise, Add file… attaches a file as one edit; the downloaded PDF carries it byte for
//   byte, as Lamina PDF's SDK and pdf.js read it; Delete attachment takes it out with a toast;
// - a portfolio's files show its visible columns;
// - a program gets the warning before it is saved;
// - a file without attachments has no such tab in Read.
// No CSP violation or other origin throughout.

import { createHash } from "node:crypto";
import { readFileSync } from "node:fs";
import { Document, init } from "@laminapdf/sdk";
import { getDocument } from "pdfjs-dist/legacy/build/pdf.mjs";
import type { Browser, Page } from "playwright";
import { afterAll, beforeAll, describe, expect, inject, it } from "vitest";
import { BROWSER, launch } from "./browser.ts";
import { chooseFile, clean, load, type Watched } from "./reader.ts";

const origin = inject("origin");

/** No pickers: Save becomes Download and Choose a file the file input, in every engine. */
const NO_PICKERS = {
  script: () => {
    Reflect.deleteProperty(window, "showOpenFilePicker");
    Reflect.deleteProperty(window, "showSaveFilePicker");
  },
  arg: [],
};

/** The attachment fixture: notes.txt (held by a paperclip on page 1 too) and readings.csv, page 2's data. */
const ASSOCIATED = readFileSync(new URL("../../../fixtures/attachments/associated.pdf", import.meta.url));

/** The portfolio fixture: two files with an owner, a due date and a page count each. */
const PORTFOLIO = readFileSync(new URL("../../../fixtures/attachments/portfolio.pdf", import.meta.url));

/** What the Add file… case attaches. */
const INVOICE = Buffer.from('<?xml version="1.0" encoding="UTF-8"?>\n<invoice number="42"/>\n');

const sha256 = (bytes: Uint8Array) => createHash("sha256").update(bytes).digest("hex");

/** Chooses the More menu's item `name`. */
async function more(page: Page, name: string): Promise<void> {
  await page.getByRole("button", { name: "More" }).click();
  await page.getByRole("menuitem", { name }).click();
}

/** Opens `bytes` as `name` with Choose a file. */
async function openBytes(page: Page, name: string, bytes: Buffer): Promise<void> {
  const chooser = page.waitForEvent("filechooser");
  await page.getByRole("button", { name: "Choose a file" }).click();
  await (await chooser).setFiles({ name, mimeType: "application/pdf", buffer: bytes });
  await page.locator(".web-viewer canvas").first().waitFor();
}

/** Opens the rail's Attachments tab and answers its panel. */
async function attachmentsTab(page: Page) {
  if ((await page.getByRole("tab", { name: "Attachments" }).count()) === 0) {
    await page.getByRole("button", { name: "Sidebar" }).click();
  }
  await page.getByRole("tab", { name: "Attachments" }).click();
  return page.getByRole("region", { name: "Attachments" });
}

/** The bytes of the download pressing `button` starts. */
async function downloaded(page: Page, press: () => Promise<void>): Promise<Buffer> {
  const download = page.waitForEvent("download", { timeout: 60_000 });
  await press();
  return readFileSync(await (await download).path());
}

describe(`attachments in ${BROWSER}`, () => {
  let browser: Browser;

  beforeAll(async () => {
    await init();
    browser = await launch();
  });

  afterAll(async () => {
    await browser?.close();
  });

  /** The reader in a wide window. */
  function reader(): Promise<Watched> {
    return load(browser, origin, "/", NO_PICKERS, { ignoreHTTPSErrors: true, viewport: { width: 1280, height: 900 } });
  }

  it("[cap J.05 web read] [cap H.09 web read] lists a file's attachments and downloads one byte for byte", async () => {
    const watched = await reader();
    const { page } = watched;
    await openBytes(page, "associated.pdf", ASSOCIATED);
    const panel = await attachmentsTab(page);
    await expect.poll(() => panel.getByText("2 files", { exact: false }).count()).toBe(1);
    const readings = panel.getByRole("option", { name: /readings\.csv/ });
    await readings.click();
    expect(await readings.getAttribute("aria-label")).toBe("readings.csv, 26 B, data, data for page 2");
    // Its stated checksum is wrong on purpose: the details say so.
    await expect.poll(() => panel.getByText("Doesn’t match").count()).toBe(1);
    const bytes = await downloaded(page, () => panel.getByRole("button", { name: "Download" }).click());
    expect(bytes.toString("latin1")).toBe("day,reading\n1,12.5\n2,13.1\n");
    await clean(watched);
  });

  it("[cap J.05 web round-trip] [cap M.09 web round-trip] attaches a file in Organise, saves it into the PDF and deletes it", async () => {
    const watched = await reader();
    const { page } = watched;
    await openBytes(page, "associated.pdf", ASSOCIATED);
    await page.getByRole("button", { name: "Organise", exact: true }).click();
    const panel = await attachmentsTab(page);
    await expect.poll(() => panel.getByText("2 files", { exact: false }).count()).toBe(1);
    const chooser = page.waitForEvent("filechooser");
    await panel.getByRole("button", { name: "Add file…" }).click();
    await (await chooser).setFiles({ name: "invoice.xml", mimeType: "text/xml", buffer: INVOICE });
    await panel.getByRole("option", { name: /invoice\.xml/ }).click();
    await expect.poll(() => panel.getByText("Matches").count()).toBe(1);
    await panel.getByLabel("Relationship").selectOption("alternative");
    await expect.poll(() => page.locator(".paper-top-bar-save-state").textContent()).toBe("2 unsaved changes");

    // The attached file's own bytes come back out, as attached.
    const out = await downloaded(page, () => panel.getByRole("button", { name: "Download" }).click());
    expect(sha256(out)).toBe(sha256(INVOICE));

    // The PDF saved with it carries it, associated with the document, and everything it had: its
    // two pages, its other files byte for byte and the paperclip annotation on page 1.
    const saved = await downloaded(page, () =>
      page.getByRole("button", { name: "Download", exact: true }).first().click(),
    );
    {
      using before = Document.open(ASSOCIATED);
      using doc = Document.open(saved);
      expect(doc.pageCount).toBe(2);
      const files = doc.attachments();
      expect(files.map((f) => f.name)).toEqual(["invoice.xml", "notes.txt", "readings.csv"]);
      const invoice = files[0];
      expect([invoice?.mime, invoice?.relationship, invoice?.holders.map((h) => h.kind)]).toEqual([
        "text/xml",
        "alternative",
        ["embeddedFiles", "document"],
      ]);
      expect(sha256(doc.attachmentBytes(invoice?.id ?? "").bytes)).toBe(sha256(INVOICE));
      for (const kept of before.attachments()) {
        const after = files.find((f) => f.name === kept.name);
        expect([after?.holders.map((h) => h.kind), sha256(doc.attachmentBytes(after?.id ?? "").bytes)]).toEqual([
          kept.holders.map((h) => h.kind),
          sha256(before.attachmentBytes(kept.id).bytes),
        ]);
      }
    }
    const task = getDocument({ data: new Uint8Array(saved), verbosity: 0 });
    try {
      const read = await task.promise;
      const files = (await read.getAttachments()) as Map<string, { filename: string }> | null;
      expect([...(files?.keys() ?? [])]).toEqual(["invoice.xml", "notes.txt", "readings.csv"]);
      const content = (await read.getAttachmentContent("invoice.xml")) as Uint8Array;
      expect(sha256(content)).toBe(sha256(INVOICE));
      const annotations = await (await read.getPage(1)).getAnnotations();
      expect(annotations.map((a: { subtype: string }) => a.subtype)).toEqual(["FileAttachment"]);
    } finally {
      await task.destroy();
    }

    await panel.getByRole("button", { name: "Delete attachment" }).click();
    await expect.poll(() => page.getByText("Attachment deleted").count()).toBe(1);
    await expect.poll(() => panel.getByRole("option", { name: /invoice\.xml/ }).count()).toBe(0);
    await clean(watched);
  });

  it("attaches a file dropped on the panel, which saves out byte for byte", async () => {
    const watched = await reader();
    const { page } = watched;
    await chooseFile(watched, "hello.pdf");
    await page.locator(".web-viewer canvas").first().waitFor();
    await page.getByRole("button", { name: "Organise", exact: true }).click();
    const panel = await attachmentsTab(page);
    const zone = panel.getByRole("region", { name: "Drop files here to attach" });
    const dataTransfer = await page.evaluateHandle(
      (bytes) => {
        const files = new DataTransfer();
        files.items.add(new File([new Uint8Array(bytes)], "invoice.xml", { type: "text/xml" }));
        return files;
      },
      [...INVOICE],
    );
    await zone.dispatchEvent("dragover", { dataTransfer });
    await zone.dispatchEvent("drop", { dataTransfer });
    await panel.getByRole("option", { name: /invoice\.xml/ }).waitFor();
    const out = await downloaded(page, () => panel.getByRole("button", { name: "Download" }).click());
    expect(sha256(out)).toBe(sha256(INVOICE));
    await clean(watched);
  });

  it("[cap J.06 web read] lists a portfolio's files with its columns", async () => {
    const watched = await reader();
    const { page } = watched;
    await openBytes(page, "portfolio.pdf", PORTFOLIO);
    const panel = await attachmentsTab(page);
    const report = panel.getByRole("option", { name: /report\.pdf/ });
    await report.waitFor();
    // Owner and Due show; Pages is a hidden column.
    expect(await report.textContent()).toContain("Owner: Finance");
    expect(await report.textContent()).toContain("Due: ");
    expect(await report.textContent()).not.toContain("Pages:");
    expect(await panel.getByRole("option", { name: /notes\.txt/ }).textContent()).toContain("Owner: Review");
    await clean(watched);
  });

  it("warns before it saves a program", async () => {
    const watched = await reader();
    const { page } = watched;
    await chooseFile(watched, "hello.pdf");
    await page.locator(".web-viewer canvas").first().waitFor();
    await page.getByRole("button", { name: "Organise", exact: true }).click();
    const panel = await attachmentsTab(page);
    const chooser = page.waitForEvent("filechooser");
    await panel.getByRole("button", { name: "Add file…" }).click();
    await (await chooser).setFiles({
      name: "setup.exe",
      mimeType: "application/x-msdownload",
      buffer: Buffer.from("MZ"),
    });
    await panel.getByRole("option", { name: /setup\.exe/ }).waitFor();
    await panel.getByRole("button", { name: "Download" }).click();
    const warning = page.getByRole("alertdialog", { name: "This file is a program" });
    await warning.waitFor();
    await warning.getByRole("button", { name: "Cancel" }).click();
    await expect.poll(() => warning.count()).toBe(0);
    await clean(watched);
  });

  it("shows no Attachments tab in Read for a file without attachments", async () => {
    const watched = await reader();
    const { page } = watched;
    await chooseFile(watched, "outline.pdf");
    await expect.poll(() => page.getByRole("tab", { name: "Outline" }).count()).toBe(1);
    await expect.poll(() => page.getByRole("tab", { name: "Attachments" }).count()).toBe(0);
    await more(page, "Close document");
    await clean(watched);
  });
});
