// The reader's text layer in the built app (todo C5a), in this run's browser: copying page 1 of
// the five-script golden gives the lines Lamina PDF extracts (fixtures/text/page-text.json, the
// snapshot the Rust and TypeScript tests share), from the plain file and from its AES-256 copy
// with the password typed once; a tagged file's headings own their spans in the accessibility
// tree's order, and an untagged file says so once. No CSP violation or other origin throughout.

import { readFileSync } from "node:fs";
import type { Browser, Page } from "playwright";
import { afterAll, beforeAll, describe, expect, inject, it } from "vitest";
import { BROWSER, launch } from "./browser.ts";
import { clean, load as loadPage, type Watched, WITHOUT_PICKER } from "./reader.ts";

const origin = inject("origin");
const FIXTURES = new URL("../../../fixtures/", import.meta.url);

/** Page 1's lines of `file` as Lamina PDF extracts them. */
function lines(file: string): string {
  const snapshot = JSON.parse(readFileSync(new URL("text/page-text.json", FIXTURES), "utf8")) as {
    files: Record<string, { lines: string[] }[]>;
  };
  return (snapshot.files[file]?.[0]?.lines ?? []).join("\n");
}

const squash = (s: string) => s.replace(/\s+/g, " ").trim();

describe(`the text layer in ${BROWSER}`, () => {
  let browser: Browser;

  beforeAll(async () => {
    browser = await launch();
  });

  afterAll(async () => {
    await browser?.close();
  });

  const load = (): Promise<Watched> => loadPage(browser, origin, "/", WITHOUT_PICKER, { ignoreHTTPSErrors: true });

  async function choose({ page }: Watched, file: string): Promise<void> {
    const chooser = page.waitForEvent("filechooser");
    await page.getByRole("button", { name: "Choose a file" }).click();
    await (await chooser).setFiles(new URL(file, FIXTURES).pathname);
  }

  /** Waits for page 1's spans, selects them all and copies: what the copy wrote. */
  async function copyPageOne(page: Page): Promise<string> {
    const layer = page.locator('.lamina-page[data-page-index="0"] .lamina-text-layer');
    await layer.locator("span").first().waitFor({ timeout: 30_000 });
    return page.evaluate(() => {
      const box = document.querySelector('.lamina-page[data-page-index="0"] .lamina-text-layer') as HTMLElement;
      document.getSelection()?.selectAllChildren(box);
      const event = new ClipboardEvent("copy", { clipboardData: new DataTransfer(), bubbles: true, cancelable: true });
      document.dispatchEvent(event);
      return event.clipboardData?.getData("text/plain") ?? "";
    });
  }

  it("copies the five scripts as Lamina PDF extracts them", async () => {
    const watched = await load();
    await choose(watched, "golden/text-shaped.pdf");
    expect(squash(await copyPageOne(watched.page))).toBe(squash(lines("golden/text-shaped.pdf")));
    await clean(watched);
  });

  it("copies the same text from the AES-256 copy once its password is typed", async () => {
    const watched = await load();
    await choose(watched, "text/encrypted/text-shaped-aes-256.pdf");
    const dialog = watched.page.getByRole("dialog", { name: "Enter the password" });
    await dialog.getByLabel("Password", { exact: true }).fill("user");
    await dialog.getByRole("button", { name: "Open" }).click();
    expect(squash(await copyPageOne(watched.page))).toBe(squash(lines("golden/text-shaped.pdf")));
    await clean(watched);
  });

  it("gives a tagged file's first heading the spans of its text", async () => {
    const watched = await load();
    await choose(watched, "golden/tagged-report.pdf");
    const heading = watched.page.locator(".lamina-structure [role=heading][aria-level='1']").first();
    await expect.poll(() => heading.getAttribute("aria-owns"), { timeout: 30_000 }).toBeTruthy();
    const owned = await heading.evaluate((el) =>
      (el.getAttribute("aria-owns") ?? "")
        .split(" ")
        .map((id) => document.getElementById(id)?.textContent ?? "")
        .join(""),
    );
    expect(owned.trim().length).toBeGreaterThan(0);
    await clean(watched);
  });

  it("says once that an untagged file is untagged", async () => {
    const watched = await load();
    await choose(watched, "golden/examples/hello.pdf");
    const note = watched.page.locator(".lamina-structure[role=status]");
    await note.waitFor({ state: "attached", timeout: 30_000 });
    // The viewer fills the live region a frame after adding it, so screen readers announce it.
    await expect.poll(() => note.textContent(), { timeout: 5_000 }).toBe("Untagged document");
    expect(await note.count()).toBe(1);
    await clean(watched);
  });
});
