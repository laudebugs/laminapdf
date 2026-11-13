// Recent files (todo C7b; R01), in this run's browser. A local file opened once is listed on the
// home screen with its page count, and pressing it opens it again (from its kept handle where the
// browser keeps one, else from the picker). With the default settings a linked `#url=` document
// adds nothing to the browser's storage. "Clear recent files and cached documents" empties both
// the list (IndexedDB) and the cached copies (OPFS).

import { readFileSync } from "node:fs";
import type { Browser, Page } from "playwright";
import { afterAll, beforeAll, describe, expect, inject, it } from "vitest";
import { BROWSER, launch } from "./browser.ts";
import { load, opened, WITHOUT_PICKER } from "./reader.ts";
import { type RemoteHost, startRemoteHost } from "./remote-host.ts";

const origin = inject("origin");
const CORPUS = new URL("../../../fixtures/corpus/", import.meta.url);

/** Every string the reader's IndexedDB holds, read through the platform's key-value store. */
function storedText(page: Page): Promise<string> {
  return page.evaluate(async () => {
    const names = (await indexedDB.databases()).map((d) => d.name).filter((n): n is string => n !== undefined);
    const out: unknown[] = [];
    for (const name of names) {
      const db = await new Promise<IDBDatabase>((resolve, reject) => {
        const request = indexedDB.open(name);
        request.onsuccess = () => resolve(request.result);
        request.onerror = () => reject(request.error);
      });
      for (const store of db.objectStoreNames) {
        const all = await new Promise<unknown[]>((resolve, reject) => {
          const request = db.transaction(store).objectStore(store).getAll();
          request.onsuccess = () => resolve(request.result);
          request.onerror = () => reject(request.error);
        });
        out.push(...all);
      }
      db.close();
    }
    return JSON.stringify(out, (_, value) => (value instanceof Blob ? "[blob]" : value));
  });
}

/** The names in the OPFS root. */
function opfsNames(page: Page): Promise<string[]> {
  return page.evaluate(async () => {
    const root = await navigator.storage.getDirectory();
    const names: string[] = [];
    for await (const [name] of (root as unknown as { entries(): AsyncIterable<[string, unknown]> }).entries()) {
      names.push(name);
    }
    return names;
  });
}

describe(`recent files in ${BROWSER}`, () => {
  let browser: Browser;
  let host: RemoteHost;

  beforeAll(async () => {
    browser = await launch();
    host = await startRemoteHost();
    host.add("hello.pdf", readFileSync(new URL("hello.pdf", CORPUS)));
  });

  afterAll(async () => {
    await browser?.close();
    await host?.close();
  });

  it("lists a local file with its pages, and opens it again from the list", async () => {
    const watched = await load(browser, origin, "/", WITHOUT_PICKER, { ignoreHTTPSErrors: true });
    const { page } = watched;
    let chooser = page.waitForEvent("filechooser");
    await page.getByRole("button", { name: "Choose a file" }).click();
    await (await chooser).setFiles(new URL("outline.pdf", CORPUS).pathname);
    await opened(watched, "Outline fixture");
    await page.getByRole("button", { name: "More" }).first().click();
    await page.getByRole("menuitem", { name: "Close document" }).click();
    const row = page.getByRole("button", { name: /outline\.pdf/ });
    await expect.poll(() => row.textContent()).toContain("3 pages");
    // Without a kept handle (no File System Access here) the row asks for the file again.
    expect(await row.textContent()).toContain("Choose the file again to reopen");
    chooser = page.waitForEvent("filechooser");
    await row.click();
    await (await chooser).setFiles(new URL("outline.pdf", CORPUS).pathname);
    await opened(watched, "Outline fixture");
    await page.close();
  });

  it("keeps nothing about a linked document: neither its name nor its address (ADR 0011)", async () => {
    const url = `${host.origin}/cors/ranges/hello.pdf`;
    const watched = await load(browser, origin, `/#url=${encodeURIComponent(url)}`, undefined, {
      ignoreHTTPSErrors: true,
      home: false,
    });
    await opened(watched, "hello.pdf");
    const stored = await storedText(watched.page);
    expect(stored).not.toContain("hello.pdf");
    expect(stored).not.toContain(host.origin);
    await watched.page.close();
  });

  it("empties the list and the cached documents from Settings", async () => {
    const watched = await load(browser, origin, "/", WITHOUT_PICKER, { ignoreHTTPSErrors: true });
    const { page } = watched;
    const chooser = page.waitForEvent("filechooser");
    await page.getByRole("button", { name: "Choose a file" }).click();
    await (await chooser).setFiles(new URL("outline.pdf", CORPUS).pathname);
    await opened(watched, "Outline fixture");
    // A cached copy, as the reader keeps one, where the browser has OPFS (Playwright's WebKit
    // doesn't in its throwaway contexts).
    const hasOpfs = await page.evaluate(async () => {
      try {
        const root = await navigator.storage.getDirectory();
        await root.getDirectoryHandle("documents", { create: true });
        return true;
      } catch {
        return false;
      }
    });
    expect(await storedText(page)).toContain("outline.pdf");
    await page.getByRole("button", { name: "More" }).first().click();
    await page.getByRole("menuitem", { name: "Settings" }).click();
    const dialog = page.getByRole("dialog", { name: "Settings" });
    await dialog.getByRole("button", { name: "Storage and privacy" }).click();
    await dialog.getByRole("button", { name: "Clear recent files and cached documents" }).click();
    await page.getByText("Recent files and cached documents cleared").waitFor();
    expect(await storedText(page)).not.toContain("outline.pdf");
    if (hasOpfs) expect(await opfsNames(page)).not.toContain("documents");
    await page.close();
  });
});
