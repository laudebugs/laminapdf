// Saving in the built reader (todo E1e, Paper O02), in this run's browser:
// - where the browser can't write files, Save is Download: the file downloads with the edit, the
//   document is clean, and the next save appends to what was downloaded;
// - in Chromium with the File System Access API (a picked file mocked in the page), Save writes in
//   place, and a refused write leaves every edit unsaved and the file as it was;
// - on a certified file the strip names the conflict, and Save asks before removing the signature;
// - on an approval-signed file the strip says the changes go after the signature, and Save as
//   optimised isn't offered.
// Each saved file is read back with Lamina PDF's SDK in Node, and the downloaded one reopened in
// the reader. No CSP violation or other origin throughout.

import { readFileSync } from "node:fs";
import { Document, init } from "@laminapdf/sdk";
import type { Browser, Page } from "playwright";
import { afterAll, beforeAll, describe, expect, inject, it } from "vitest";
import { BROWSER, launch } from "./browser.ts";
import { clean, type InitScript, load, type Watched, WITHOUT_PICKER } from "./reader.ts";

const origin = inject("origin");
const FIXTURES = new URL("../../../fixtures/", import.meta.url);
const FILE = new URL("golden/links-and-outlines.pdf", FIXTURES);
const NAME = "links-and-outlines.pdf";

/** No pickers at all: Save becomes Download, as in Firefox and Safari. */
const NO_PICKERS: InitScript = {
  script: () => {
    Reflect.deleteProperty(window, "showOpenFilePicker");
    Reflect.deleteProperty(window, "showSaveFilePicker");
  },
  arg: [],
};

/**
 * A picked file mocked in the page: the open picker answers a handle over `bytes`, whose writes
 * land in `window.__file` (and `window.__writes` counts them); `refuse` makes every write fail as a
 * permission the browser took back.
 */
function pickedFile(bytes: number[], refuse: boolean): InitScript {
  return {
    // The argument is numbers only: the first says whether to refuse, the rest are the file.
    script: (arg: number[]) => {
      const name = "links-and-outlines.pdf";
      const refused = arg[0] === 1;
      const scope = window as unknown as { __file: Uint8Array; __writes: number };
      scope.__file = new Uint8Array(arg.slice(1));
      scope.__writes = 0;
      const handle = {
        kind: "file",
        name,
        getFile: async () => new File([scope.__file as BlobPart], name, { type: "application/pdf" }),
        createWritable: async () => {
          if (refused) throw new DOMException("The user took the permission back.", "NotAllowedError");
          let next = new Uint8Array();
          return {
            write: async (chunk: Uint8Array) => {
              next = new Uint8Array(chunk);
            },
            close: async () => {
              scope.__file = next;
              scope.__writes += 1;
            },
            abort: async () => undefined,
          };
        },
        queryPermission: async () => "granted",
        requestPermission: async () => "granted",
      };
      Object.assign(window, { showOpenFilePicker: async () => [handle] });
      Reflect.deleteProperty(window, "showSaveFilePicker");
    },
    arg: [refuse ? 1 : 0, ...bytes],
  };
}

/** The top bar's save state ("" before any edit). */
const saveState = (page: Page) => page.locator(".paper-top-bar-save-state").textContent();

/** Chooses the More menu's item `name`. */
async function more(page: Page, name: string): Promise<void> {
  await page.getByRole("button", { name: "More" }).click();
  await page.getByRole("menuitem", { name }).click();
}

/** Page 1's rotation in `bytes`, read with the SDK. */
function firstRotation(bytes: Uint8Array): number {
  using doc = Document.open(bytes);
  return doc.pageInfo(0).rotation;
}

describe(`saving in ${BROWSER}`, () => {
  let browser: Browser;

  beforeAll(async () => {
    await init();
    browser = await launch();
  });

  afterAll(async () => {
    await browser?.close();
  });

  /** The reader with `file` open through Choose a file, set up by `script`. */
  async function opened(script: InitScript, file = FILE): Promise<Watched> {
    const watched = await load(browser, origin, "/", script, {
      ignoreHTTPSErrors: true,
      viewport: { width: 1280, height: 800 },
    });
    const { page } = watched;
    const chooser = script === NO_PICKERS || script === WITHOUT_PICKER ? page.waitForEvent("filechooser") : undefined;
    await page.getByRole("button", { name: "Choose a file" }).click();
    if (chooser !== undefined) await (await chooser).setFiles(file.pathname);
    await page.locator(".web-viewer canvas").first().waitFor();
    // The window names the document: by its title when it has one (the signed fixtures do), else the file.
    await expect.poll(() => page.title()).not.toBe("Lamina PDF");
    return watched;
  }

  /** Turns page 1 and waits for the change to count. */
  async function turn(page: Page, unsaved = "1 unsaved change"): Promise<void> {
    await more(page, "Rotate page right");
    await expect.poll(() => saveState(page)).toBe(unsaved);
  }

  it("downloads where the browser can't write files, and the next save appends to the download", async () => {
    const watched = await opened(NO_PICKERS);
    const { page } = watched;
    await turn(page);
    let download = page.waitForEvent("download");
    await page.getByRole("button", { name: "Download", exact: true }).click();
    const first = new Uint8Array(readFileSync(await (await download).path()));
    await expect.poll(() => saveState(page)).toBe("No unsaved changes");
    await expect.poll(() => page.getByText(`Downloaded ${NAME}`).count()).toBe(1);

    await turn(page);
    download = page.waitForEvent("download");
    await page.getByRole("button", { name: "Download", exact: true }).click();
    const second = new Uint8Array(readFileSync(await (await download).path()));
    expect([
      firstRotation(first),
      firstRotation(second),
      second.subarray(0, first.length).every((b, i) => b === first[i]),
    ]).toEqual([90, 180, true]);
    await clean(watched);
  });

  it("reopens a downloaded save in the reader with the change", async () => {
    const watched = await opened(NO_PICKERS);
    const { page } = watched;
    await turn(page);
    const download = page.waitForEvent("download");
    await page.getByRole("button", { name: "Download", exact: true }).click();
    const path = await (await download).path();
    await expect.poll(() => saveState(page)).toBe("No unsaved changes");
    await more(page, "Close document");
    const chooser = page.waitForEvent("filechooser");
    await page.getByRole("button", { name: "Choose a file" }).click();
    await (await chooser).setFiles({ name: NAME, mimeType: "application/pdf", buffer: readFileSync(path) });
    await page.locator(".web-viewer canvas").first().waitFor();
    const box = await page.locator('.lamina-page[data-page-index="0"]').boundingBox();
    expect((box?.width ?? 0) > (box?.height ?? 0)).toBe(true);
    await clean(watched);
  });

  it.runIf(BROWSER === "chromium")("saves in place through the File System Access API", async () => {
    const watched = await opened(pickedFile([...readFileSync(FILE)], false));
    const { page } = watched;
    await turn(page);
    await page.getByRole("button", { name: "Save", exact: true }).click();
    await expect.poll(() => saveState(page)).toBe("No unsaved changes");
    await expect.poll(() => page.getByText(`Saved ${NAME}`).count()).toBe(1);
    const written = new Uint8Array(
      await page.evaluate(() => [...(window as unknown as { __file: Uint8Array }).__file]),
    );
    const original = readFileSync(FILE);
    expect([firstRotation(written), written.subarray(0, original.length).every((b, i) => b === original[i])]).toEqual([
      90,
      true,
    ]);
    await clean(watched);
  });

  it.runIf(BROWSER === "chromium")(
    "leaves every edit unsaved and the file as it was when the write is refused",
    async () => {
      const watched = await opened(pickedFile([...readFileSync(FILE)], true));
      const { page } = watched;
      await turn(page);
      await page.getByRole("button", { name: "Save", exact: true }).click();
      await page
        .getByText(`Couldn't save ${NAME}: permission to write it was refused. Your changes are still here.`)
        .waitFor();
      const writes = await page.evaluate(() => (window as unknown as { __writes: number }).__writes);
      expect([await saveState(page), writes]).toEqual(["1 unsaved change", 0]);
      await clean(watched);
    },
  );

  it("names the conflict on a certified file, and asks before removing the certification", async () => {
    const watched = await opened(NO_PICKERS, new URL("signed/certified-p1.pdf", FIXTURES));
    const { page } = watched;
    await turn(page);
    await page.getByText("Saving will invalidate the certification.").waitFor();
    await page.getByRole("button", { name: "Download", exact: true }).click();
    const ask = page.getByRole("alertdialog", { name: "Remove the certification and save?" });
    await ask.waitFor();
    await ask.getByRole("button", { name: "Cancel" }).click();
    await ask.waitFor({ state: "detached" });
    expect(await saveState(page)).toBe("1 unsaved change");
    await clean(watched);
  });

  it("says a signed file's changes go after the signature, and won't rewrite it", async () => {
    const watched = await opened(NO_PICKERS, new URL("signed/approval.pdf", FIXTURES));
    const { page } = watched;
    // A metadata edit leaves the signed page as it was; rotating that page would reshow the
    // signature's widget, which the next test covers.
    await more(page, "Document properties…");
    const dialog = page.getByRole("dialog", { name: "Document properties" });
    await dialog.getByLabel("Title").fill("Approved budget");
    await page.getByRole("button", { name: "Apply" }).click();
    await expect.poll(() => saveState(page)).toBe("1 unsaved change");
    await page
      .getByText("Signed file. Saving adds your changes after the signature; the signature stays valid.")
      .waitFor();
    await page.getByRole("button", { name: "Save options" }).click();
    await page.getByRole("menuitem", { name: "Save as optimised" }).click();
    await page.getByText("Not available for this file: a full rewrite would invalidate the signature.").waitFor();
    expect(await saveState(page)).toBe("1 unsaved change");
    await clean(watched);
  });

  it("says rotating the page a signature is shown on would invalidate it", async () => {
    const watched = await opened(NO_PICKERS, new URL("signed/approval.pdf", FIXTURES));
    const { page } = watched;
    await turn(page);
    await page.getByText("Saving will invalidate the signature.").waitFor();
    await clean(watched);
  });
});
