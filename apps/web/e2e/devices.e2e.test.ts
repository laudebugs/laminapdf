// The reader on phones and tablets (UI/UX plan §5.4), in Playwright's device profiles with touch:
// iPhone 15 and iPad Pro 11 in WebKit (the nightly's WebKit leg), Pixel 8 in Chromium, also narrowed
// to 320 px (every PR); Firefox has no mobile emulation. Each
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

/** One device profile to run: Playwright's, optionally narrowed, and whether it is phone-sized. */
interface Profile {
  readonly label: string;
  readonly device: string;
  readonly phone: boolean;
  readonly width?: number;
}

/** The profiles each engine runs: the narrowest phone the layout supports, 320 wide, in Chromium too. */
const PROFILES: Record<string, readonly Profile[]> = {
  chromium: [
    { label: "Pixel 8", device: "Pixel 8", phone: true },
    { label: "Pixel 8 at 320 px", device: "Pixel 8", phone: true, width: 320 },
  ],
  webkit: [
    { label: "iPhone 15", device: "iPhone 15", phone: true },
    { label: "iPad Pro 11", device: "iPad Pro 11", phone: false },
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

/**
 * Scrolls the viewer to `at` of its range on each axis (0 the start, 0.5 the middle), so a gesture
 * has room to move the page both ways; returns how far it can still scroll down.
 */
const scrollTo = (page: Page, at: number) =>
  page.evaluate(async (share) => {
    const root = document.querySelector(".lamina-viewer") as HTMLElement;
    root.scrollTo({
      left: (root.scrollWidth - root.clientWidth) * share,
      top: (root.scrollHeight - root.clientHeight) * share,
      behavior: "instant",
    });
    await new Promise((resolve) => requestAnimationFrame(resolve));
    return root.scrollHeight - root.clientHeight - root.scrollTop;
  }, at);

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
      const device = devices[profile.device];
      const viewport = profile.width === undefined ? device?.viewport : { width: profile.width, height: 640 };
      const watched = await load(browser, origin, "/", WITHOUT_PICKER, {
        ...device,
        ...(viewport === undefined ? {} : { viewport }),
        ignoreHTTPSErrors: true,
        bypassCSP,
      });
      await chooseFile(watched, "hello.pdf");
      await opened(watched, "hello.pdf");
      return watched;
    };

    it(`${profile.label}: opens a file in the ${profile.phone ? "phone" : "desktop"} shell without scrolling sideways`, async () => {
      const watched = await open();
      const { page } = watched;
      const bar = await page.getByRole("toolbar", { name: "Document" }).count();
      const commands = await page.getByRole("button", { name: /Commands/ }).count();
      // Nothing reaches past the window, and every control in the bars is a 44 px target.
      const { wide, past, small } = await page.evaluate(() => {
        const bars = [...document.querySelectorAll("header *, footer *")];
        const targets = bars.filter((el) => el.matches("button, a[href], [role=button], [role=tab], input"));
        // Touch sizes follow a coarse pointer (design language §8.7); a profile without one is a desktop's.
        const coarse = matchMedia("(pointer: coarse)").matches;
        return {
          wide: document.documentElement.scrollWidth - window.innerWidth,
          past: bars.filter((el) => el.getBoundingClientRect().right > window.innerWidth + 0.5).length,
          small: targets
            .filter(() => coarse)
            .filter((el) => {
              const r = el.getBoundingClientRect();
              return r.width > 0 && (r.width + 0.5 < 44 || r.height + 0.5 < 44);
            })
            .map((el) => el.getAttribute("aria-label") ?? el.textContent),
        };
      });
      expect([bar, commands > 0, wide, past]).toEqual(profile.phone ? [1, false, 0, 0] : [0, true, 0, 0]);
      expect(small).toEqual([]);
      await clean(watched);
    });

    it(`${profile.label}: zooms on a double tap and keeps the point under a pinch within 2 px`, async () => {
      const watched = await open();
      const { page } = watched;
      const before = await firstPage(page);
      const at = { id: 1, x: before.left + before.width / 2, y: before.top + before.height / 3 };
      for (const _ of [0, 1]) await touches(page, [[at], []]);
      await expect.poll(async () => (await firstPage(page)).width).toBeGreaterThan(before.width * 1.5);
      // A pinch around the middle of the window: the fingers spread from 80 to 140 px apart while
      // both also move 30 px right and down.
      await scrollTo(page, 0.5);
      const zoomed = await firstPage(page);
      const mid = await page.evaluate(() => ({ x: window.innerWidth / 2, y: window.innerHeight / 2 }));
      const share = { x: (mid.x - zoomed.left) / zoomed.width, y: (mid.y - zoomed.top) / zoomed.height };
      const steps = [0, 0.25, 0.5, 0.75, 1].map((t) => {
        const half = 40 + 30 * t;
        const shift = 30 * t;
        return [
          { id: 1, x: mid.x - half + shift, y: mid.y + shift },
          { id: 2, x: mid.x + half + shift, y: mid.y + shift },
        ];
      });
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

    it(`${profile.label}: pans the document under one finger`, async () => {
      const watched = await open();
      const { page } = watched;
      const box = await firstPage(page);
      const centre = { id: 1, x: box.left + box.width / 2, y: box.top + box.height / 2 };
      for (const _ of [0, 1]) await touches(page, [[centre], []]);
      await expect.poll(async () => (await firstPage(page)).width).toBeGreaterThan(box.width * 1.5);
      const room = await scrollTo(page, 0);
      const top = await page.evaluate(() => (document.querySelector(".lamina-viewer") as HTMLElement).scrollTop);
      const drag = [0, 1, 2, 3, 4].map((i) => [{ id: 3, x: 150, y: 500 - i * 60 }]);
      await touches(page, drag);
      const moved = await page.evaluate(() => (document.querySelector(".lamina-viewer") as HTMLElement).scrollTop);
      // 240 px of drag, or all the room the page has below, less a pixel for rounding.
      expect(moved - top).toBeGreaterThanOrEqual(Math.min(200, room - 1));
      await clean(watched);
    });

    it(`${profile.label}: passes axe`, async () => {
      const watched = await open(true);
      const { page } = watched;
      await page.addScriptTag({ path: AXE });
      const serious = await page.evaluate(async () => {
        const run = (
          window as unknown as { axe: { run: (n: Node) => Promise<{ violations: { id: string; impact: string }[] }> } }
        ).axe;
        const { violations } = await run.run(document.body);
        return violations.filter((v) => v.impact === "serious" || v.impact === "critical").map((v) => v.id);
      });
      expect(serious).toEqual([]);
      await clean(watched);
    });
  }
});
