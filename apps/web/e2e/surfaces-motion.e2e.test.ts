// The reader's surfaces in motion (todo U7d; motion plan §3, 3, 4 and 10), in this run's browser: a
// page fades in from its paper rather than appearing in one frame, with reduced motion too, and
// shows at full opacity once it is sharp; the inspector slides and fades in and out while the page
// view keeps the width it took on the first frame, and with reduced motion it only fades. Each test
// pins the motion preference, since a machine's own (or CI's WebKit) may ask for reduced motion.

import type { Browser, Page } from "playwright";
import { afterAll, beforeAll, describe, expect, inject, it } from "vitest";
import { BROWSER, launch } from "./browser.ts";
import { chooseFile, clean, load, opened, WITHOUT_PICKER } from "./reader.ts";

const origin = inject("origin");

/**
 * Records the properties every page canvas starts a transition on, from the `transitionrun`
 * events, so no frame has to be caught.
 */
function recordCanvasTransitions(page: Page) {
  return page.evaluate(() => {
    const seen: string[] = [];
    Object.assign(window, { canvasSeen: seen });
    document.addEventListener("transitionrun", (event) => {
      if (event.target instanceof Element && event.target.matches(".lamina-page-canvas")) seen.push(event.propertyName);
    });
  });
}

const canvasTransitions = (page: Page) =>
  page.evaluate(() => [...new Set((window as unknown as { canvasSeen: string[] }).canvasSeen)]);

/**
 * Records the page view's width and the inspector's transitions, at the frame the inspector is put
 * in or marked leaving and at every frame after, until `stopInspectorLog()` stops it. The log is
 * read from the transitions' own records (CSSTransition), never from waits of a set length.
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
    const canvas = document.querySelector(".paper-reader-canvas") as HTMLElement;
    const now = () => {
      const inspector = document.querySelector(".paper-inspector");
      return inspector === null ? "closed" : inspector.hasAttribute("data-exiting") ? "exiting" : "open";
    };
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
    let frameId = 0;
    const frame = () => {
      const seen = now();
      note(seen === state ? "frame" : seen);
      state = seen;
      frameId = requestAnimationFrame(frame);
    };
    const observer = new MutationObserver(() => {
      const seen = now();
      if (seen !== state) {
        note(seen);
        state = seen;
      }
    });
    observer.observe(document.body, {
      subtree: true,
      childList: true,
      attributes: true,
      attributeFilter: ["data-exiting"],
    });
    frameId = requestAnimationFrame(frame);
    Object.assign(window, {
      inspectorLog: log,
      stopInspectorLog: () => {
        cancelAnimationFrame(frameId);
        observer.disconnect();
      },
    });
  });
}

type Logged = { at: string; canvas: number; transitions: string[] };
const inspectorLog = (page: Page) =>
  page.evaluate(() => (window as unknown as { inspectorLog: Logged[] }).inspectorLog);

/** Frames logged since the inspector was last seen gone, after it had been leaving. */
const framesSinceClosed = async (page: Page) => {
  const log = await inspectorLog(page);
  const exiting = log.findIndex((l) => l.at === "exiting");
  const closed = log.findIndex((l, i) => i > exiting && l.at === "closed");
  return exiting < 0 || closed < 0 ? 0 : log.length - 1 - closed;
};

/** Waits for the transitions running on the first element `selector` names to end. */
const transitionsEnd = (page: Page, selector: string) =>
  page
    .locator(selector)
    .first()
    .evaluate((element) => Promise.all(element.getAnimations().map((a) => a.finished)).then(() => undefined));

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
      await recordCanvasTransitions(page);
      await chooseFile(watched, "hello.pdf");
      await opened(watched, "hello.pdf");
      // The canvas fades: an opacity transition, and nothing else, starts on it.
      await expect.poll(() => canvasTransitions(page)).toEqual(["opacity"]);
      const sharp = page.locator('.lamina-page[data-render="sharp"] > .lamina-page-canvas').first();
      await sharp.waitFor({ state: "attached" });
      await expect
        .poll(() => sharp.evaluate((canvas) => [getComputedStyle(canvas).opacity, canvas.getAnimations().length]))
        .toEqual(["1", 0]);
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
      await transitionsEnd(page, ".paper-inspector");
      await indicator.click();
      await page.locator(".paper-inspector").waitFor({ state: "detached" });
      // A few frames with it gone, to see the page view get its width back and keep it.
      await expect.poll(() => framesSinceClosed(page)).toBeGreaterThanOrEqual(3);
      await page.evaluate(() => (window as unknown as { stopInspectorLog: () => void }).stopInspectorLog());
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
      // What ran while it came in, and while it left, over every frame of each, not one frame's.
      const ran = (from: number, to: number) => [...new Set(log.slice(from, to).flatMap((l) => l.transitions))].sort();
      expect(ran(opening, exiting)).toEqual(moved);
      expect(ran(exiting, closed)).toEqual(moved);
      await clean(watched);
    });
  }
});
