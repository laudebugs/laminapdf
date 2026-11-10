// Link previews (todo T5a) in the built reader, in this run's browser. A "paper" built here with
// @laminapdf/sdk: pages 1–2 cite [1]–[30], linked to named destinations at `XYZ` points on each
// reference entry, split over both columns of pages 7–8; page 1 also links to page 3 as a `Fit`
// destination and to a figure on page 5, which has /Rotate 90. Hovering a citation shows its entry
// after 300 ms, the preview can be hovered and goes on Escape, a click jumps there and Back returns,
// the keyboard and a touch long-press work too, other link kinds say what they do, and nothing
// asks another origin for anything. C6b's links.pdf gives the page actions and the other-file link.

import { readFileSync } from "node:fs";
import { Builder, Font, init } from "@laminapdf/sdk";
import { type Browser, devices, type Page } from "playwright";
import { afterAll, beforeAll, describe, expect, inject, it } from "vitest";
import { BROWSER, launch } from "./browser.ts";
import { load as loadPage, type Watched } from "./reader.ts";
import { type RemoteHost, startRemoteHost } from "./remote-host.ts";

const origin = inject("origin");
const LINKS = readFileSync(new URL("../../../fixtures/links/links.pdf", import.meta.url));

/** Reference `n`'s entry, as the fixture writes its first line. */
const entry = (n: number) => `[${n}] A. Author${n}. On preview ${n}. Journal of Links, ${2000 + n}.`;

/** Where reference `n` is: page 7 or 8 (0-based 6 or 7), left or right column, and its line's top. */
function placeOfReference(n: number): { page: number; x: number; top: number } {
  const i = n - 1;
  const page = i < 16 ? 6 : 7;
  const k = i % 16;
  return { page, x: k < 8 ? 56 : 316, top: 100 + (k % 8) * 60 };
}

type Surface = ReturnType<ReturnType<Builder["page"]>["surface"]>;
const FONT = () => Font.standard("Helvetica");

/** Pages 1–2: "As shown in [n]", each citation linked to its reference. */
function drawCitations(s: Surface, p: number): void {
  for (let k = 0; k < 15; k++) {
    const n = p * 15 + k + 1;
    const y = 120 + k * 36;
    s.text("As shown in", { font: FONT(), size: 11, at: [56, y] });
    s.text(`[${n}]`, { font: FONT(), size: 11, at: [124, y], link: { target: { type: "named", name: `cite.${n}` } } });
  }
}

/** Pages 7–8: the references in two columns, two lines each. */
function drawReferences(s: Surface, p: number): void {
  for (let n = 1; n <= 30; n++) {
    const at = placeOfReference(n);
    if (at.page !== p) continue;
    s.text(entry(n), { font: FONT(), size: 9, at: [at.x, at.top + 10] });
    s.text("Second line of the entry.", { font: FONT(), size: 9, at: [at.x, at.top + 22] });
  }
}

/** Page 1's links to page 3 (`Fit`) and to the figure on the turned page 5. */
function drawOtherLinks(s: Surface): void {
  const page3 = { type: "page", index: 2, fit: { type: "page" } } as const;
  s.text("see page 3", { font: FONT(), size: 11, at: [300, 120], link: { target: page3 } });
  s.text("see the figure", {
    font: FONT(),
    size: 11,
    at: [300, 156],
    link: { target: { type: "named", name: "figure" } },
  });
}

/** The paper: 8 letter pages (page 5 turned 90°), 30 citations and their references. */
function paper(): Uint8Array {
  const b = new Builder({ id: new TextEncoder().encode("t5a-link-preview"), producer: "lamina tests" });
  for (let n = 1; n <= 30; n++) {
    const { page, x, top } = placeOfReference(n);
    b.destination(`cite.${n}`, page, { type: "xyz", left: x, top });
  }
  b.destination("figure", 4, { type: "xyz", left: 72, top: 200 });
  for (let p = 0; p < 8; p++) {
    const page = b.page([612, 792]);
    if (p === 4) page.rotate(90);
    const s = page.surface();
    s.text(`Page ${p + 1}`, { font: FONT(), size: 10, at: [56, 40] });
    if (p < 2) drawCitations(s, p);
    if (p === 0) drawOtherLinks(s);
    if (p === 4) s.text("Figure 1. A turned figure.", { font: FONT(), size: 11, at: [72, 210] });
    if (p >= 6) drawReferences(s, p);
    page.finish();
  }
  const doc = b.finish();
  const bytes = doc.save();
  doc.free();
  return bytes;
}

const shownPage = (page: Page) => page.getByRole("textbox", { name: "Page" }).inputValue();
/** The page the address names (`#page=`), which the phone layout shows without a page field. */
const addressPage = (page: Page) => new URLSearchParams(new URL(page.url()).hash.slice(1)).get("page");
const tooltip = (page: Page) => page.locator("[role=tooltip].paper-link-preview");

describe(`link previews in ${BROWSER}`, () => {
  let browser: Browser;
  let host: RemoteHost;

  beforeAll(async () => {
    await init({ text: true });
    browser = await launch();
    host = await startRemoteHost();
    host.add("paper.pdf", paper());
    host.add("links.pdf", LINKS);
  });

  afterAll(async () => {
    await browser?.close();
    await host?.close();
  });

  const fileUrl = (name: string) => `${host.origin}/cors/ranges/${name}`;

  async function open(name: string, options: Parameters<typeof loadPage>[4] = {}): Promise<Watched> {
    const watched = await loadPage(browser, origin, `/#url=${encodeURIComponent(fileUrl(name))}`, undefined, {
      ignoreHTTPSErrors: true,
      viewport: { width: 1280, height: 900 },
      home: false,
      ...options,
    });
    await watched.page.locator(".web-viewer a.lamina-link").first().waitFor();
    return watched;
  }

  it("shows a citation's reference 300 ms after the mouse comes onto it", async () => {
    const { page, foreign } = await open("paper.pdf");
    const link = page.getByRole("link", { name: "Link to page 7" }).nth(11);
    await link.scrollIntoViewIfNeeded();
    const box = await link.boundingBox();
    if (box === null) throw new Error("no box");
    await page.evaluate(() => {
      const w = window as unknown as { shownAt?: number };
      new MutationObserver(() => {
        if (w.shownAt === undefined && document.querySelector("[role=tooltip].paper-link-preview") !== null)
          w.shownAt = performance.now();
      }).observe(document.body, { childList: true, subtree: true });
    });
    const hoveredAt = await page.evaluate(() => performance.now());
    await page.mouse.move(box.x + box.width / 2, box.y + box.height / 2);
    await tooltip(page).waitFor();
    const shownAt = await page.evaluate(() => (window as unknown as { shownAt?: number }).shownAt ?? 0);
    expect(shownAt - hoveredAt).toBeGreaterThanOrEqual(250);
    expect(shownAt - hoveredAt).toBeLessThan(600);
    const picture = tooltip(page).getByRole("img");
    await picture.waitFor({ timeout: 2000 });
    expect(await picture.getAttribute("aria-label")).toContain(entry(12));
    // The file itself, read from its host; previews ask nothing of any origin.
    expect(foreign.filter((url) => url !== fileUrl("paper.pdf"))).toEqual([]);
    await page.close();
  });

  it("shows a right-column reference's column", async () => {
    const { page } = await open("paper.pdf");
    // [12] is the 12th citation: page 7, the right column's fourth entry.
    await page.getByRole("link", { name: "Link to page 7" }).nth(11).hover();
    const name = await tooltip(page).getByRole("img").getAttribute("aria-label");
    expect([name?.includes(entry(12)), name?.includes(entry(4))]).toEqual([true, false]);
    await page.close();
  });

  it("stays while hovered, goes 300 ms after the mouse leaves both, and goes on Escape", async () => {
    const { page } = await open("paper.pdf");
    const link = page.getByRole("link", { name: "Link to page 7" }).first();
    await link.hover();
    await tooltip(page).waitFor();
    await tooltip(page).hover();
    await page.waitForTimeout(500);
    expect(await tooltip(page).count()).toBe(1);
    await page.mouse.move(5, 5);
    await expect.poll(() => tooltip(page).count(), { timeout: 2000 }).toBe(0);
    await link.hover();
    await tooltip(page).waitFor();
    await page.keyboard.press("Escape");
    expect(await tooltip(page).count()).toBe(0);
    await page.close();
  });

  it("jumps to the reference on a click, with its entry at the top, and Back returns", async () => {
    const { page } = await open("paper.pdf");
    const link = page.getByRole("link", { name: "Link to page 8" }).first();
    await link.scrollIntoViewIfNeeded();
    await link.click();
    await expect.poll(() => shownPage(page)).toBe("8");
    // [17]'s entry: its text span's top is within 16 CSS px of the viewer's top.
    const span = page.locator(".web-viewer .lamina-layer-text span", { hasText: "[17] A. Author17" });
    await span.waitFor();
    const [spanTop, viewTop] = await Promise.all([
      span.evaluate((el) => el.getBoundingClientRect().top),
      page.locator(".web-viewer .lamina-viewer").evaluate((el) => el.getBoundingClientRect().top),
    ]);
    expect(Math.abs(spanTop - viewTop)).toBeLessThan(16 + 12);
    await page.goBack();
    // Back to the citing line: the link is in view again.
    await expect
      .poll(() =>
        link.evaluate((a) => a.getBoundingClientRect().top > 0 && a.getBoundingClientRect().bottom < innerHeight),
      )
      .toBe(true);
    await page.goForward();
    await expect.poll(() => shownPage(page)).toBe("8");
    await page.close();
  });

  it("shows on keyboard focus, which names the preview, and Enter follows", async () => {
    const { page } = await open("paper.pdf");
    await page.keyboard.press("Tab");
    const link = page.getByRole("link", { name: "Link to page 7" }).first();
    await link.focus();
    await tooltip(page).waitFor();
    const id = await tooltip(page).getAttribute("id");
    expect(await link.getAttribute("aria-describedby")).toContain(id ?? "-");
    await page.keyboard.press("Enter");
    await expect.poll(() => shownPage(page)).toBe("7");
    await page.close();
  });

  it("shows the top of the page for a Fit link", async () => {
    const { page } = await open("paper.pdf");
    await page.getByRole("link", { name: "Link to page 3" }).first().hover();
    await expect.poll(async () => tooltip(page).getByRole("img").getAttribute("aria-label")).toBe("Top of page 3");
    await page.close();
  });

  it("previews the figure on the turned page", async () => {
    const { page } = await open("paper.pdf");
    await page.getByRole("link", { name: "Link to page 5" }).first().hover();
    await expect
      .poll(async () => tooltip(page).getByRole("img").getAttribute("aria-label"), { timeout: 5000 })
      .toContain("Figure 1");
    await page.close();
  });

  it("says what a page action and another file's link do, and never previews a web link", async () => {
    const { page } = await open("links.pdf");
    await page.getByRole("link", { name: "Goes to the next page" }).first().hover();
    await expect.poll(() => tooltip(page).textContent()).toContain("Goes to the next page");
    expect(await tooltip(page).getByRole("img").count()).toBe(0);
    await page.mouse.move(5, 5);
    await expect.poll(() => tooltip(page).count(), { timeout: 2000 }).toBe(0);
    await page.getByRole("link", { name: "The project site" }).hover();
    await page.waitForTimeout(600);
    expect(await tooltip(page).count()).toBe(0);
    await page.close();
  });

  // Firefox has no touch emulation in Playwright.
  describe.skipIf(BROWSER === "firefox")("with touch", () => {
    const device = BROWSER === "webkit" ? devices["iPhone 15"] : devices["Pixel 7"];

    /** A finger held on `link` for `ms`, without moving. */
    async function longPress(page: Page, link: ReturnType<Page["getByRole"]>, ms = 650): Promise<void> {
      const box = await link.boundingBox();
      if (box === null) throw new Error("no box");
      const init = { pointerType: "touch", pointerId: 9, isPrimary: true, clientX: box.x + 2, clientY: box.y + 2 };
      await link.dispatchEvent("pointerdown", init);
      await page.waitForTimeout(ms);
      await link.dispatchEvent("pointerup", init);
      await link.dispatchEvent("click", { button: 0 });
    }

    it("opens a sheet on a long-press, selects no text, and Go follows", async () => {
      const { page } = await open("paper.pdf", {
        ...device,
        viewport: device?.viewport ?? { width: 390, height: 844 },
      });
      const link = page.getByRole("link", { name: "Link to page 7" }).first();
      expect(await link.evaluate((a) => getComputedStyle(a).getPropertyValue("-webkit-touch-callout") || "none")).toBe(
        "none",
      );
      await longPress(page, link);
      const sheet = page.getByRole("dialog");
      await sheet.waitFor();
      expect(await page.evaluate(() => document.getSelection()?.toString() ?? "")).toBe("");
      const before = addressPage(page);
      await sheet.getByRole("button", { name: "Go to page 7" }).click();
      await expect.poll(() => addressPage(page)).toBe("7");
      expect(before).not.toBe("7");
      await page.close();
    });

    it("follows a tap without a sheet", async () => {
      const { page } = await open("paper.pdf", {
        ...device,
        viewport: device?.viewport ?? { width: 390, height: 844 },
      });
      await page.getByRole("link", { name: "Link to page 3" }).first().tap();
      await expect.poll(() => addressPage(page)).toBe("3");
      expect(await page.getByRole("dialog").count()).toBe(0);
      await page.close();
    });
  });
});
