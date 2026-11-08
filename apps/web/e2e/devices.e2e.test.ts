// The reader on phones and tablets (UI/UX plan §5.4), in Playwright's device profiles with touch:
// iPhone 15 and iPad Pro 11 in WebKit, Pixel 8 in Chromium (Firefox has no mobile emulation). Each
// opens a file, pans and zooms it with synthetic touches (the viewer owns the gestures: pinch keeps
// the point under the fingers within 2 px), never scrolls sideways, and passes axe; the phone shows
// the bottom bar and the tablet the desktop shell.

import { createRequire } from "node:module";
import { env } from "node:process";
import { type Browser, type BrowserType, chromium, devices, type Page, webkit } from "playwright";
import { afterAll, beforeAll, describe, expect, inject, it } from "vitest";
import { chooseFile, clean, load, opened, WITHOUT_PICKER } from "./reader.ts";

const BROWSER = env.LAMINA_PDF_E2E_BROWSER ?? "chromium";
const origin = inject("origin");
const AXE = createRequire(import.meta.url).resolve("axe-core");

/** The profiles each engine runs, and whether each is phone-sized. */
const PROFILES: Record<string, readonly { name: string; phone: boolean }[]> = {
  chromium: [{ name: "Pixel 8", phone: true }],
  webkit: [
    { name: "iPhone 15", phone: true },
    { name: "iPad Pro 11", phone: false },
  ],
};
const ENGINES: Record<string, BrowserType> = { chromium, webkit };

type Finger = { readonly id: number; readonly x: number; readonly y: number };
type Fired = Finger & { readonly type: "pointerdown" | "pointermove" | "pointerup" };

/** The pointer events of `steps`, one frame each: a finger new in a step goes down, one missing from it goes up. */
function eventsOf(steps: readonly (readonly Finger[])[]): Fired[][] {
  let down = new Map<number, Finger>();
  const frames = [...steps, []].map((step) => {
    const now = new Map(step.map((f) => [f.id, f]));
    const moved = step.map((f): Fired => ({ ...f, type: down.has(f.id) ? "pointermove" : "pointerdown" }));
    const lifted = [...down.values()].filter((f) => !now.has(f.id)).map((f): Fired => ({ ...f, type: "pointerup" }));
    down = now;
    return [...moved, ...lifted];
  });
  return frames;
}

/** A touch sequence on the viewer: each step lists the fingers' positions in viewport CSS px. */
async function touches(page: Page, steps: readonly (readonly Finger[])[]): Promise<void> {
  await page.evaluate(async (frames) => {
    const root = document.querySelector(".lamina-viewer") as HTMLElement;
    for (const frame of frames) {
      for (const f of frame) {
        const init = { pointerId: f.id, pointerType: "touch", clientX: f.x, clientY: f.y, bubbles: true };
        root.dispatchEvent(new PointerEvent(f.type, init));
      }
      await new Promise((resolve) => requestAnimationFrame(resolve));
    }
  }, eventsOf(steps));
}

/** The first page's box, in viewport CSS px. */
const firstPage = (page: Page) =>
  page.evaluate(() => {
    const r = (document.querySelector(".web-viewer canvas") as HTMLElement).getBoundingClientRect();
    return { left: r.left, top: r.top, width: r.width, height: r.height };
  });

describe.skipIf(PROFILES[BROWSER] === undefined)(`phones and tablets in ${BROWSER}`, () => {
  let browser: Browser;

  beforeAll(async () => {
    browser = await (ENGINES[BROWSER] as BrowserType).launch();
  });

  afterAll(async () => {
    await browser?.close();
  });

  for (const profile of PROFILES[BROWSER] ?? []) {
    const open = async (bypassCSP = false) => {
      const watched = await load(browser, origin, "/", WITHOUT_PICKER, {
        ...devices[profile.name],
        ignoreHTTPSErrors: true,
        bypassCSP,
      });
      await chooseFile(watched, "hello.pdf");
      await opened(watched, "hello.pdf");
      return watched;
    };

    it(`${profile.name}: opens a file in the ${profile.phone ? "phone" : "desktop"} shell without scrolling sideways`, async () => {
      const watched = await open();
      const { page } = watched;
      const bar = await page.getByRole("toolbar", { name: "Document" }).count();
      const commands = await page.getByRole("button", { name: /Commands/ }).count();
      const wide = await page.evaluate(() => document.documentElement.scrollWidth - window.innerWidth);
      expect([bar, commands > 0, wide]).toEqual(profile.phone ? [1, false, 0] : [0, true, 0]);
      await clean(watched);
    });

    it(`${profile.name}: zooms on a double tap and keeps the point under a pinch within 2 px`, async () => {
      const watched = await open();
      const { page } = watched;
      const before = await firstPage(page);
      const at = { id: 1, x: before.left + before.width / 2, y: before.top + before.height / 3 };
      for (const _ of [0, 1]) await touches(page, [[at], []]);
      await expect.poll(async () => (await firstPage(page)).width).toBeGreaterThan(before.width * 1.5);
      // A pinch outwards from two fingers that also move together, by 30 px down and to the side.
      const zoomed = await firstPage(page);
      const mid = { x: zoomed.left + zoomed.width / 4, y: zoomed.top + zoomed.height / 4 };
      const share = { x: (mid.x - zoomed.left) / zoomed.width, y: (mid.y - zoomed.top) / zoomed.height };
      const steps = [0, 0.25, 0.5, 0.75, 1].map((t) => [
        { id: 1, x: mid.x - 40 - 30 * t + 30 * t, y: mid.y + 30 * t },
        { id: 2, x: mid.x + 40 + 30 * t + 30 * t, y: mid.y + 30 * t },
      ]);
      await touches(page, steps);
      const after = await firstPage(page);
      const end = { x: mid.x + 30, y: mid.y + 30 };
      const now = { x: after.left + share.x * after.width, y: after.top + share.y * after.height };
      expect([after.width > zoomed.width, Math.abs(now.x - end.x) <= 2, Math.abs(now.y - end.y) <= 2]).toEqual([
        true,
        true,
        true,
      ]);
      await clean(watched);
    });

    it(`${profile.name}: pans the document under one finger`, async () => {
      const watched = await open();
      const { page } = watched;
      const box = await firstPage(page);
      const centre = { id: 1, x: box.left + box.width / 2, y: box.top + box.height / 2 };
      for (const _ of [0, 1]) await touches(page, [[centre], []]);
      await expect.poll(async () => (await firstPage(page)).width).toBeGreaterThan(box.width * 1.5);
      const top = await page.evaluate(() => (document.querySelector(".lamina-viewer") as HTMLElement).scrollTop);
      const drag = [0, 1, 2, 3, 4].map((i) => [{ id: 3, x: 150, y: 500 - i * 60 }]);
      await touches(page, drag);
      const moved = await page.evaluate(() => (document.querySelector(".lamina-viewer") as HTMLElement).scrollTop);
      expect(moved - top).toBeGreaterThanOrEqual(200);
      await clean(watched);
    });

    it(`${profile.name}: passes axe`, async () => {
      const { page } = await open(true);
      await page.addScriptTag({ path: AXE });
      const serious = await page.evaluate(async () => {
        const run = (
          window as unknown as { axe: { run: (n: Node) => Promise<{ violations: { id: string; impact: string }[] }> } }
        ).axe;
        const { violations } = await run.run(document.body);
        return violations.filter((v) => v.impact === "serious" || v.impact === "critical").map((v) => v.id);
      });
      expect(serious).toEqual([]);
      await page.close();
    });
  }
});
