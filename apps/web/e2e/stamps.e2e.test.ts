// Stamps and attached files in Comment mode (Paper C05), in this run's browser, with the real
// mouse and keyboard: a standard, a dynamic and a custom image stamp from the picker, and a file
// attached to a spot on the page, saved as a download and read back by pdf.js with every stamp
// drawn and the file's bytes as they were attached. A sound comment plays its recording. axe
// finds no serious violation in the picker or the attachment inspector.

import { readFileSync } from "node:fs";
import { getDocument } from "pdfjs-dist/legacy/build/pdf.mjs";
import type { Browser, Download, Page } from "playwright";
import { afterAll, beforeAll, describe, expect, inject, it } from "vitest";
import { BROWSER, launch } from "./browser.ts";
import { clean, load, serious, type Watched } from "./reader.ts";

const origin = inject("origin");
let browser: Browser;
const TEXT = new URL("../../../fixtures/reading-order/two-columns-ordered.pdf", import.meta.url);
const SOUND = new URL("../../../fixtures/annotations/sound.pdf", import.meta.url);
const PAGE = '.lamina-page[data-page-index="0"]';
/** A 1 × 1 PNG: a custom stamp's image. */
const PNG = Buffer.from(
  "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mNk+M9QDwADhgGAWjR9awAAAABJRU5ErkJggg==",
  "base64",
);
const CSV = Buffer.from("day,reading\n1,12.5\n2,13.1\n");

/**
 * The reader with `file` open in Comment mode, the comments' name already chosen, no file pickers;
 * each media element's play recorded, so a test can tell a sound played.
 */
async function comment(file: URL): Promise<Watched> {
  const watched = await load(
    browser,
    origin,
    "/",
    {
      script: () => {
        Reflect.deleteProperty(window, "showOpenFilePicker");
        Reflect.deleteProperty(window, "showSaveFilePicker");
        localStorage.setItem("lamina.comment.author", "Ada");
        const played: string[] = [];
        Reflect.set(window, "__played", played);
        const play = HTMLMediaElement.prototype.play;
        HTMLMediaElement.prototype.play = function (this: HTMLMediaElement) {
          const started = play.call(this);
          started.then(
            () => played.push("played"),
            (e: unknown) => played.push(`refused: ${String(e)}`),
          );
          return started;
        };
      },
    } as never,
    { ignoreHTTPSErrors: true, viewport: { width: 1400, height: 1600 }, bypassCSP: true },
  );
  const { page } = watched;
  const chooser = page.waitForEvent("filechooser");
  await page.getByRole("button", { name: "Choose a file" }).click();
  await (await chooser).setFiles(file.pathname);
  await page.locator(".web-viewer canvas").first().waitFor({ timeout: 30_000 });
  await page.getByRole("button", { name: "Comment", exact: true }).click();
  await toolbar(page).waitFor();
  return watched;
}

const toolbar = (page: Page) => page.getByRole("toolbar", { name: "Comment tools" });
const inspectorOf = (page: Page) => page.getByRole("complementary", { name: "Comment" });

/** The viewport position of (`x`, `y`) points from page 1's top-left. */
async function at(page: Page, x: number, y: number): Promise<{ x: number; y: number }> {
  const box = await page.locator(PAGE).boundingBox();
  if (box === null) throw new Error("page 1 isn't shown");
  const scale = box.width / 595.275591;
  return { x: box.x + x * scale, y: box.y + y * scale };
}

/** A click on page 1 at (`x`, `y`) points, after a move there so a stamp's preview follows. */
async function click(page: Page, x: number, y: number): Promise<void> {
  const point = await at(page, x, y);
  await page.mouse.move(point.x, point.y, { steps: 3 });
  await page.mouse.click(point.x, point.y);
}

/** Opens the stamp picker on `tab`. */
async function picker(page: Page, tab: "Standard" | "Dynamic" | "Custom") {
  await toolbar(page).getByRole("button", { name: "Stamp", exact: true }).click();
  const dialog = page.getByRole("dialog", { name: "Stamps" });
  await dialog.getByRole("tab", { name: tab }).click();
  return dialog;
}

/** The count of unsaved changes the top bar says, once it says `count`. */
async function changes(page: Page, count: number): Promise<void> {
  const text = count === 1 ? "1 unsaved change" : `${count} unsaved changes`;
  await expect.poll(() => page.locator(".paper-top-bar-save-state").textContent(), { timeout: 30_000 }).toBe(text);
}

/** Saves (a download) and returns page 1's stamps and attachments as pdf.js reads them, each attached file with its bytes. */
async function saved(page: Page) {
  const download: Promise<Download> = page.waitForEvent("download");
  await page.getByRole("button", { name: "Download", exact: true }).click();
  const bytes = new Uint8Array(readFileSync(await (await download).path()));
  const task = getDocument({ data: bytes, verbosity: 0 });
  const doc = await task.promise;
  try {
    const annotations = (await (await doc.getPage(1)).getAnnotations()) as {
      subtype: string;
      hasAppearance: boolean;
      fileId?: string;
      file?: { filename: string };
    }[];
    const marks = annotations.filter((a) => a.subtype === "Stamp" || a.subtype === "FileAttachment");
    // pdf.js gives an attached file's bytes by its ID, apart from the annotation.
    return Promise.all(
      marks.map(async (a) => ({
        ...a,
        content:
          a.fileId === undefined
            ? undefined
            : ((await (
                doc as unknown as { getAttachmentContent(id: string): Promise<Uint8Array | undefined> }
              ).getAttachmentContent(a.fileId)) ?? undefined),
      })),
    );
  } finally {
    await task.destroy();
  }
}

describe(`Stamps and attached files in ${BROWSER}`, () => {
  beforeAll(async () => {
    browser = await launch();
  });

  afterAll(async () => {
    await browser?.close();
  });

  it("[cap D.07 web round-trip] [cap D.08 web round-trip] places a standard, a dynamic and a custom stamp and attaches a file, and pdf.js reads each back", async () => {
    const watched = await comment(TEXT);
    const { page } = watched;

    let dialog = await picker(page, "Standard");
    expect(await serious(page)).toEqual([]);
    await dialog.getByRole("button", { name: "Approved", exact: true }).click();
    await click(page, 150, 420);
    await changes(page, 1);

    dialog = await picker(page, "Dynamic");
    await dialog.getByRole("button", { name: /^Reviewed/ }).click();
    await click(page, 400, 420);
    await changes(page, 2);

    dialog = await picker(page, "Custom");
    await dialog.getByText("No custom stamps yet").waitFor();
    const chooser = page.waitForEvent("filechooser");
    await dialog.getByRole("button", { name: "Add image…" }).click();
    await (await chooser).setFiles({ name: "Company seal.png", mimeType: "image/png", buffer: PNG });
    await dialog.getByRole("button", { name: "Company seal" }).click();
    await click(page, 150, 520);
    await changes(page, 3);

    await toolbar(page).getByRole("button", { name: "Attach file", exact: true }).click();
    const files = page.waitForEvent("filechooser");
    await click(page, 400, 520);
    await (await files).setFiles({ name: "readings.csv", mimeType: "text/csv", buffer: CSV });
    await changes(page, 4);
    // The attachment is selected: its inspector shows the file.
    const inspector = inspectorOf(page);
    await inspector.locator(".paper-comment-file-name", { hasText: "readings.csv" }).waitFor();
    expect(await serious(page)).toEqual([]);

    const marks = await saved(page);
    expect(marks.map((a) => a.subtype).sort()).toEqual(["FileAttachment", "Stamp", "Stamp", "Stamp"]);
    expect(marks.every((a) => a.hasAppearance)).toBe(true);
    const attached = marks.find((a) => a.subtype === "FileAttachment");
    expect([attached?.file?.filename, new TextDecoder().decode(attached?.content)]).toEqual([
      "readings.csv",
      CSV.toString(),
    ]);
    await clean(watched);
  });

  it("[cap D.09 web read] plays a sound comment's recording", async () => {
    const watched = await comment(SOUND);
    const { page } = watched;
    await page.locator(`${PAGE} .lamina-annotation[aria-label^="Sound comment"]`).first().click();
    await inspectorOf(page)
      .getByRole("button", { name: /^Play sound comment/ })
      .click();
    await expect
      .poll(() => page.evaluate(() => (Reflect.get(window, "__played") as string[]).join()), { timeout: 20_000 })
      .toBe("played");
    expect(await serious(page)).toEqual([]);
    await clean(watched);
  });
});
