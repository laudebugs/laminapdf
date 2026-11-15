// The reader's surfaces in motion (todo U7d; motion plan §3, 3, 4 and 10), in this run's browser: a
// page fades in from its paper rather than appearing in one frame, with reduced motion too, and
// shows at full opacity once it is sharp; the inspector slides and fades in and out while the page
// view keeps the width it took on the first frame, and with reduced motion it only fades.

import type { Browser, Page } from "playwright";
import { afterAll, beforeAll, describe, expect, inject, it } from "vitest";
import { BROWSER, launch } from "./browser.ts";
import { chooseFile, clean, load, opened, WITHOUT_PICKER } from "./reader.ts";

const origin = inject("origin");

/** Records, as the first page canvas goes onto its page, the transitions it starts with. */
function recordFirstCanvas(page: Page) {
  return page.evaluate(() => {
    const seen: string[][] = [];
    Object.assign(window, { canvasSeen: seen });
    new MutationObserver((_, observer) => {
      const canvas = document.querySelector(".lamina-page-canvas");
      if (canvas === null) return;
      observer.disconnect();
      getComputedStyle(canvas).opacity;
      seen.push(
        canvas
          .getAnimations()
          .filter((a): a is CSSTransition => a instanceof CSSTransition)
          .map((a) => a.transitionProperty),
      );
    }).observe(document.body, { subtree: true, childList: true });
  });
}

/**
 * Records the page view's width and the inspector's transitions, at the frame the inspector is put
 * in or marked leaving and at every frame after, until it has been gone for a while.
 */
function recordInspector(page: Page) {
  return page.evaluate(() => {
    // The properties the element's running transitions animate.
    const of = (element: Element) =>
      element
        .getAnimations()
        .filter((a): a is CSSTransition => a instanceof CSSTransition)
        .map((a) => a.transitionProperty)
        .sort();
    const log: { at: string; canvas: number; transitions: string[] }[] = [];
    Object.assign(window, { inspectorLog: log });
    const canvas = document.querySelector(".paper-reader-canvas") as HTMLElement;
    const note = (at: string) => {
      const inspector = document.querySelector(".paper-inspector");
      if (inspector !== null) getComputedStyle(inspector).opacity;
      log.push({
        at,
        canvas: canvas.getBoundingClientRect().width,
        transitions: inspector === null ? [] : of(inspector),
      });
    };
    let state = "closed";
    const frame = () => {
      const inspector = document.querySelector(".paper-inspector");
      const now = inspector === null ? "closed" : inspector.hasAttribute("data-exiting") ? "exiting" : "open";
      note(now === state ? "frame" : now);
      state = now;
      requestAnimationFrame(frame);
    };
    new MutationObserver(() => {
      const inspector = document.querySelector(".paper-inspector");
      const now = inspector === null ? "closed" : inspector.hasAttribute("data-exiting") ? "exiting" : "open";
      if (now !== state) {
        note(now);
        state = now;
      }
    }).observe(document.body, { subtree: true, childList: true, attributes: true, attributeFilter: ["data-exiting"] });
    requestAnimationFrame(frame);
  });
}

type Logged = { at: string; canvas: number; transitions: string[] };
const inspectorLog = (page: Page) =>
  page.evaluate(() => (window as unknown as { inspectorLog: Logged[] }).inspectorLog);

describe(`the reader's surfaces in motion in ${BROWSER}`, () => {
  let browser: Browser;

  beforeAll(async () => {
    browser = await launch();
  });

  afterAll(async () => {
    await browser?.close();
  });

  for (const reduced of [false, true]) {
    it(`fades a page in from its paper${reduced ? ", with reduced motion too" : ""}, then shows it whole once it is sharp`, async () => {
      const watched = await load(browser, origin, "/", WITHOUT_PICKER, {
        ignoreHTTPSErrors: true,
        reducedMotion: reduced ? "reduce" : "no-preference",
      });
      const { page } = watched;
      await recordFirstCanvas(page);
      await chooseFile(watched, "hello.pdf");
      await opened(watched, "hello.pdf");
      const seen = await page.evaluate(() => (window as unknown as { canvasSeen: string[][] }).canvasSeen);
      expect(seen).toEqual([["opacity"]]);
      await page.locator('.lamina-page[data-render="sharp"]').first().waitFor({ state: "attached" });
      const sharp = await page
        .locator('.lamina-page[data-render="sharp"] > .lamina-page-canvas')
        .first()
        .evaluate((canvas) => [getComputedStyle(canvas).opacity, canvas.getAnimations().length]);
      expect(sharp).toEqual(["1", 0]);
      await clean(watched);
    });
  }

  for (const reduced of [false, true]) {
    it(`opens and closes the inspector${reduced ? " by a fade alone" : " by a slide and a fade"}, moving nothing in the page view meanwhile`, async () => {
      const watched = await load(browser, origin, "/", WITHOUT_PICKER, {
        ignoreHTTPSErrors: true,
        viewport: { width: 1280, height: 900 },
        reducedMotion: reduced ? "reduce" : "no-preference",
      });
      const { page } = watched;
      await chooseFile(watched, "hello.pdf");
      await opened(watched, "hello.pdf");
      const indicator = page.getByRole("button", { name: /network requests? for this document/ });
      await recordInspector(page);
      await indicator.click();
      await page.locator(".paper-inspector:not([data-exiting])").waitFor();
      await page.waitForTimeout(400);
      await indicator.click();
      await page.locator(".paper-inspector").waitFor({ state: "detached" });
      await page.waitForTimeout(100);
      const log = await inspectorLog(page);
      const opening = log.findIndex((l) => l.at === "open");
      const exiting = log.findIndex((l) => l.at === "exiting");
      const closed = log.findIndex((l, i) => i > exiting && l.at === "closed");
      expect([opening, exiting, closed].every((i) => i > 0)).toBe(true);
      const before = log[opening - 1]?.canvas ?? 0;
      const docked = log[opening]?.canvas ?? 0;
      // The page view takes its new width on the inspector's first frame, and keeps it until the
      // inspector has gone; then it has its width back at once.
      expect(docked).toBeLessThan(before);
      expect(new Set(log.slice(opening, closed).map((l) => l.canvas))).toEqual(new Set([docked]));
      expect(log[closed]?.canvas).toBe(before);
      // It slides as well as fades only where motion is allowed.
      const moved = reduced ? ["opacity"] : ["opacity", "translate"];
      expect(log[opening]?.transitions).toEqual(moved);
      expect(log[exiting]?.transitions).toEqual(moved);
      await clean(watched);
    });
  }
});
