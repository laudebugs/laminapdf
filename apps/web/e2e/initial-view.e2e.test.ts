// How a file opens, and its article threads, in the built reader (todo E3d, Paper O13), in this
// run's browser:
// - More › Initial view… sets the opening page and zoom, what shows beside the page and the
//   document title in the window; Apply is one edit; the downloaded file holds them, as Lamina
//   PDF's SDK and pdf.js read it; reopened, the reader opens at that page with the Outline tab;
// - a file with article threads gets a Threads tab that lists each thread's parts and goes to
//   them, Next part and Previous part included; a file without threads has no such tab.
// No CSP violation or other origin throughout.

import { readFileSync } from "node:fs";
import { Document, init, loadExtract } from "@laminapdf/sdk";
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

/** The page box's value: the label of the page the viewer shows. */
const shownPage = (page: Page) => page.getByRole("textbox", { name: "Page" }).inputValue();

/**
 * A classic PDF of `objects` (numbered from 1; object 1 the catalog) with its xref table, for
 * files the corpus has no example of.
 */
function pdf(objects: readonly string[]): Buffer {
  let body = "%PDF-1.7\n";
  const offsets: number[] = [];
  objects.forEach((object, i) => {
    offsets.push(body.length);
    body += `${i + 1} 0 obj\n${object}\nendobj\n`;
  });
  const xref = body.length;
  body += `xref\n0 ${objects.length + 1}\n0000000000 65535 f \n`;
  body += offsets.map((o) => `${String(o).padStart(10, "0")} 00000 n \n`).join("");
  body += `trailer\n<< /Size ${objects.length + 1} /Root 1 0 R >>\nstartxref\n${xref}\n%%EOF\n`;
  return Buffer.from(body, "latin1");
}

/** Four letter-size pages and one article thread, "Backlog", in three parts on pages 1, 3 and 4. */
const THREADED = pdf([
  "<< /Type /Catalog /Pages 2 0 R /Threads [7 0 R] >>",
  "<< /Type /Pages /Kids [3 0 R 4 0 R 5 0 R 6 0 R] /Count 4 /MediaBox [0 0 612 792] >>",
  "<< /Type /Page /Parent 2 0 R >>",
  "<< /Type /Page /Parent 2 0 R >>",
  "<< /Type /Page /Parent 2 0 R >>",
  "<< /Type /Page /Parent 2 0 R >>",
  "<< /Type /Thread /F 8 0 R /I << /Title (Backlog) >> >>",
  "<< /Type /Bead /T 7 0 R /N 9 0 R /V 10 0 R /P 3 0 R /R [72 400 540 720] >>",
  "<< /Type /Bead /N 10 0 R /V 8 0 R /P 5 0 R /R [72 72 540 720] >>",
  "<< /Type /Bead /N 8 0 R /V 9 0 R /P 6 0 R /R [72 72 540 400] >>",
]);

/** Chooses the More menu's item `name`. */
async function more(page: Page, name: string): Promise<void> {
  await page.getByRole("button", { name: "More" }).click();
  await page.getByRole("menuitem", { name }).click();
}

describe(`how a file opens, and its threads, in ${BROWSER}`, () => {
  let browser: Browser;

  beforeAll(async () => {
    await init();
    await loadExtract();
    browser = await launch();
  });

  afterAll(async () => {
    await browser?.close();
  });

  /** The reader in a wide window. */
  function reader(): Promise<Watched> {
    return load(browser, origin, "/", NO_PICKERS, { ignoreHTTPSErrors: true, viewport: { width: 1280, height: 900 } });
  }

  /** Opens `bytes` as `name` with Choose a file, from the home screen or over an open document. */
  async function openBytes(page: Page, name: string, bytes: Buffer): Promise<void> {
    const chooser = page.waitForEvent("filechooser");
    if ((await page.getByRole("button", { name: "Choose a file" }).count()) === 0) await more(page, "Close document");
    await page.getByRole("button", { name: "Choose a file" }).click();
    await (await chooser).setFiles({ name, mimeType: "application/pdf", buffer: bytes });
    await page.locator(".web-viewer canvas").first().waitFor();
  }

  it("[cap I.06 web round-trip] [cap I.11 web round-trip] sets where the file opens, its panel and window title, and opens there", async () => {
    const watched = await reader();
    const { page } = watched;
    await chooseFile(watched, "outline.pdf");
    await page.locator(".web-viewer canvas").first().waitFor();
    await more(page, "Initial view…");
    const dialog = page.getByRole("dialog", { name: "Initial view" });
    // Nothing set yet: every control at the reader's default, and nothing to apply.
    await expect.poll(() => dialog.getByRole("button", { name: "Apply" }).isDisabled()).toBe(true);
    await dialog.getByLabel("Open at page").fill("3");
    await dialog.getByLabel("Zoom").selectOption("fitPage");
    await dialog.getByLabel("Show when opened").selectOption("outlines");
    // A person clicks the words beside a radio or checkbox.
    await dialog.getByText("Continuous scroll", { exact: true }).click();
    await dialog.getByText("Show the document title", { exact: true }).click();
    expect(await dialog.getByRole("radio", { name: "Continuous scroll" }).isChecked()).toBe(true);
    expect(await dialog.getByRole("checkbox", { name: "Show the document title" }).isChecked()).toBe(true);
    await dialog.getByRole("button", { name: "Apply" }).click();
    await expect.poll(() => page.locator(".paper-top-bar-save-state").textContent()).toBe("1 unsaved change");
    const download = page.waitForEvent("download");
    await page.getByRole("button", { name: "Download", exact: true }).click();
    const saved = readFileSync(await (await download).path());

    {
      using doc = Document.open(saved);
      const view = doc.initialView();
      expect([view.open, view.pageMode, view.pageLayout, view.preferences.displayDocTitle]).toEqual([
        { kind: "page", target: { page: 2, view: { kind: "fit" } } },
        "outlines",
        "oneColumn",
        true,
      ]);
    }
    const task = getDocument({ data: new Uint8Array(saved), verbosity: 0 });
    const read = await task.promise;
    try {
      // pdf.js answers with Maps, keyed by the dictionary's entries.
      const open = (await read.getOpenAction()) as Map<string, unknown> | null;
      const prefs = (await read.getViewerPreferences()) as Map<string, unknown> | null;
      const dest = open?.get("dest") as [unknown, { name: string }] | undefined;
      expect([
        dest?.[0],
        dest?.[1].name,
        await read.getPageMode(),
        await read.getPageLayout(),
        prefs?.get("DisplayDocTitle"),
        read.numPages,
      ]).toEqual([(await read.getPage(3)).ref, "Fit", "UseOutlines", "OneColumn", true, 3]);
    } finally {
      await task.destroy();
    }

    // Reopened, the reader goes to page 3 and shows the Outline tab.
    await openBytes(page, "outline.pdf", saved);
    await expect.poll(() => shownPage(page)).toBe("A-2");
    await expect.poll(() => page.getByRole("tab", { name: "Outline" }).getAttribute("aria-selected")).toBe("true");
    await clean(watched);
  });

  it("[cap I.09 web read] lists a file's article threads and follows them part by part", async () => {
    const watched = await reader();
    const { page } = watched;
    await openBytes(page, "threads.pdf", THREADED);
    await page.getByRole("button", { name: "Sidebar" }).click();
    await page.getByRole("tab", { name: "Threads" }).click();
    const panel = page.getByRole("region", { name: "Article threads" });
    await expect
      .poll(() => panel.getByRole("button", { name: /Backlog/ }).textContent())
      .toContain("3 parts · pages 1, 3, 4");
    await panel.getByRole("button", { name: "Part 2 · page 3" }).click();
    await expect.poll(() => shownPage(page)).toBe("3");
    expect(await panel.getByRole("button", { name: "Part 2 · page 3" }).getAttribute("aria-current")).toBe("location");
    await panel.getByRole("button", { name: "Next part" }).click();
    await expect.poll(() => shownPage(page)).toBe("4");
    expect(await panel.getByRole("button", { name: "Next part" }).isDisabled()).toBe(true);
    await panel.getByRole("button", { name: "Previous part" }).click();
    await expect.poll(() => shownPage(page)).toBe("3");
    // Back returns to where the reader was before following the thread.
    await page.getByRole("button", { name: "Back to page 4" }).click();
    await expect.poll(() => shownPage(page)).toBe("4");

    // A file without threads has no Threads tab.
    await more(page, "Close document");
    await chooseFile(watched, "outline.pdf");
    await expect.poll(() => page.getByRole("tab", { name: "Outline" }).count()).toBe(1);
    expect(await page.getByRole("tab", { name: "Threads" }).count()).toBe(0);
    await clean(watched);
  });
});
