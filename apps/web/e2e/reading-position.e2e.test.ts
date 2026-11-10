// Reopening each document where the reader left it (todo T5b), in this run's browser. A 40-page
// file built here with @laminapdf/sdk reopens at its page and zoom, with Back going to page 1. The
// position follows the file's content, not its name: a renamed copy and the file after an
// incremental save come back there, a different file with the same name and size doesn't, and a
// link naming a page wins. Only the position is stored, under a fingerprint; the setting and
// Forget reading positions empty the store.

import { mkdtempSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { Builder, Document, init } from "@laminapdf/sdk";
import type { Browser, Page } from "playwright";
import { afterAll, beforeAll, describe, expect, inject, it } from "vitest";
import { BROWSER, launch } from "./browser.ts";
import { clean, load, opened, type Watched, WITHOUT_PICKER } from "./reader.ts";
import { type RemoteHost, startRemoteHost } from "./remote-host.ts";

const origin = inject("origin");

/** A 40-page document of letter pages, each with one box; `id` (16 bytes) keeps two of them apart. */
function longDocument(id: string): Uint8Array {
  const b = new Builder({ id: new TextEncoder().encode(id), producer: "lamina tests" });
  for (let i = 0; i < 40; i++) {
    const page = b.page([612, 792]);
    page
      .surface()
      .fillColor([0.2, 0.4, 0.6])
      .path((p) => p.rect([40, 40 + (i % 10) * 60, 200, 40]))
      .fill();
    page.finish();
  }
  const doc = b.finish();
  const bytes = doc.save();
  doc.free();
  return bytes;
}

/** `bytes` with its title set by an incremental update, as an editor's save adds one. */
function editedIncrementally(bytes: Uint8Array): Uint8Array {
  const doc = Document.open(bytes);
  doc.apply([{ op: "setInfo", args: { patch: { title: "Edited" }, at: "D:20261110120000Z" } }]);
  const saved = doc.save();
  doc.free();
  return saved;
}

const shownPage = (page: Page) => page.getByRole("textbox", { name: "Page" }).inputValue();

/** Opens the file at `path` with Choose a file, from the home screen (More takes the page there). */
async function choose(page: Page, path: string): Promise<void> {
  const home = page.getByRole("button", { name: "Choose a file" }).first();
  if (!(await home.isVisible())) {
    await page.getByRole("button", { name: "More" }).first().click();
    await page.getByRole("menuitem", { name: "Choose a file" }).click();
  }
  const chooser = page.waitForEvent("filechooser");
  await home.click();
  await (await chooser).setFiles(path);
}

/** Goes to page `n` and zoom `percent`, then waits for the position to be saved. */
async function readTo(page: Page, n: number, percent: number): Promise<void> {
  const input = page.getByRole("textbox", { name: "Page" });
  await input.fill(String(n));
  await input.press("Enter");
  await expect.poll(() => shownPage(page)).toBe(String(n));
  await page.getByRole("button", { name: /^Zoom, \d+%$/ }).click();
  await page.getByRole("menuitem", { name: `${percent}%` }).click();
  await page.getByRole("button", { name: `Zoom, ${percent}%` }).waitFor();
  // Saved a second after the view stops changing.
  await page.waitForTimeout(1500);
}

/** Every value of the reader's `positions` store, with its key. */
function storedPositions(page: Page): Promise<{ key: string; value: unknown }[]> {
  return page.evaluate(
    () =>
      new Promise((resolve, reject) => {
        const request = indexedDB.open("lamina-pdf");
        request.onerror = () => reject(request.error);
        request.onsuccess = () => {
          const db = request.result;
          const out: { key: string; value: unknown }[] = [];
          const cursor = db.transaction("positions").objectStore("positions").openCursor();
          cursor.onsuccess = () => {
            const at = cursor.result;
            if (at === null) {
              db.close();
              resolve(out);
              return;
            }
            out.push({ key: String(at.key), value: at.value });
            at.continue();
          };
          cursor.onerror = () => reject(cursor.error);
        };
      }),
  );
}

/** Opens Settings › Storage and privacy. */
async function storageSettings(page: Page) {
  await page.getByRole("button", { name: "More" }).first().click();
  await page.getByRole("menuitem", { name: "Settings" }).click();
  const dialog = page.getByRole("dialog", { name: "Settings" });
  await dialog.getByRole("button", { name: "Storage and privacy" }).click();
  return dialog;
}

describe(`reading positions in ${BROWSER}`, () => {
  let browser: Browser;
  let host: RemoteHost;
  let folder: string;
  let original: Uint8Array;

  /** Writes `bytes` as `name` in a folder of its own (`dir`), and returns its path. */
  const file = (dir: string, name: string, bytes: Uint8Array) => {
    const at = join(mkdtempSync(join(folder, `${dir}-`)), name);
    writeFileSync(at, bytes);
    return at;
  };

  /** A fresh page on the home screen, and `long-read.pdf` opened and read to page 12 at 150 %. */
  async function readLongFile(): Promise<{ watched: Watched; path: string }> {
    const watched = await load(browser, origin, "/", WITHOUT_PICKER, { ignoreHTTPSErrors: true });
    const path = file("first", "long-read.pdf", original);
    await choose(watched.page, path);
    await opened(watched, "long-read.pdf");
    await readTo(watched.page, 12, 150);
    return { watched, path };
  }

  beforeAll(async () => {
    await init();
    browser = await launch();
    host = await startRemoteHost();
    folder = mkdtempSync(join(tmpdir(), "lamina-positions-"));
    original = longDocument("lamina-t5b-first");
  });

  afterAll(async () => {
    await browser?.close();
    await host?.close();
  });

  it("reopens a file at its page and zoom, says so, and Back goes to page 1", async () => {
    const { watched, path } = await readLongFile();
    const { page } = watched;
    // Away to another page, then the same file again.
    await choose(page, file("other", "other.pdf", longDocument("lamina-t5b-third")));
    await opened(watched, "other.pdf");
    await choose(page, path);
    await opened(watched, "long-read.pdf");
    await expect.poll(() => shownPage(page)).toBe("12");
    await page.getByRole("button", { name: "Zoom, 150%" }).waitFor();
    await page.getByText("Opened at page 12, where you stopped reading").waitFor({ state: "attached" });
    await page.getByRole("button", { name: "Back to page 1" }).click();
    await expect.poll(() => shownPage(page)).toBe("1");
    await clean(watched);
  });

  it("follows the content: a renamed copy and an incremental save restore, a lookalike doesn't", async () => {
    const { watched } = await readLongFile();
    const { page } = watched;
    await choose(page, file("renamed", "renamed-copy.pdf", original));
    await opened(watched, "renamed-copy.pdf");
    await expect.poll(() => shownPage(page)).toBe("12");
    const edited = editedIncrementally(original);
    expect(edited.subarray(0, original.length)).toEqual(original);
    await choose(page, file("edited", "long-read.pdf", edited));
    await opened(watched, "Edited");
    await expect.poll(() => shownPage(page)).toBe("12");
    const lookalike = longDocument("lamina-t5b-other");
    expect(lookalike.length).toBe(original.length);
    await choose(page, file("lookalike", "long-read.pdf", lookalike));
    await opened(watched, "long-read.pdf");
    await page.waitForTimeout(500);
    expect(await shownPage(page)).toBe("1");
    await clean(watched);
  });

  it("lets a link that names a page win over the kept position", async () => {
    host.add("long-read.pdf", original);
    const url = encodeURIComponent(`${host.origin}/cors/ranges/long-read.pdf`);
    const watched = await load(browser, origin, `/#url=${url}`, undefined, { ignoreHTTPSErrors: true, home: false });
    const { page } = watched;
    await opened(watched, "long-read.pdf");
    await readTo(page, 12, 150);
    // The same link again, in the same browser: back at page 12; only its fingerprint was kept.
    const visit = async (path: string) => {
      await page.goto("about:blank");
      await page.goto(new URL(path, origin).href);
      await opened(watched, "long-read.pdf");
    };
    await visit(`/#url=${url}`);
    await expect.poll(() => shownPage(page)).toBe("12");
    expect(JSON.stringify(await storedPositions(page))).not.toContain(host.origin);
    await visit(`/#url=${url}&page=3`);
    await expect.poll(() => shownPage(page)).toBe("3");
    await page.close();
  });

  it("stores only the view, page count and time under a fingerprint, and forgets on request", async () => {
    const { watched } = await readLongFile();
    const { page } = watched;
    const stored = await storedPositions(page);
    expect(stored.length).toBeGreaterThan(0);
    for (const { key, value } of stored) {
      expect(key).toMatch(/^fp1:[0-9a-f]{64}$/);
      expect(Object.keys(value as object).sort()).toEqual(["pageCount", "readAt", "view"]);
    }
    expect(JSON.stringify(stored)).not.toContain("long-read");
    // The command, from the palette.
    await page.getByRole("button", { name: /^Commands/ }).click();
    await page.getByRole("dialog").getByRole("combobox").fill("Forget reading positions");
    await page.keyboard.press("Enter");
    await page.getByText("Reading positions forgotten").waitFor();
    expect(await storedPositions(page)).toEqual([]);
    // The setting: off writes nothing.
    const dialog = await storageSettings(page);
    const remember = dialog.getByRole("switch", { name: "Remember where I stopped reading" });
    // The switch's label covers its input, as a person sees it.
    await dialog.getByText("Remember where I stopped reading").click();
    await expect.poll(() => remember.isChecked()).toBe(false);
    await page.keyboard.press("Escape");
    await readTo(page, 20, 150);
    expect(await storedPositions(page)).toEqual([]);
    await clean(watched);
  });
});
