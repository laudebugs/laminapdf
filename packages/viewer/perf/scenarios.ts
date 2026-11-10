// The harness's scenarios, each on a fresh page of the harness (page/main.ts), driven through
// `window.__laminaPerf` with Playwright's real input: wheel steps at fixed intervals, so two runs
// scroll the same way.

import { Buffer } from "node:buffer";
import { type Browser, type BrowserContext, devices, type Page } from "playwright";
// The built module: its own imports end in .js, which Node resolves only once compiled.
import { budgetBytes, viewerBudgets } from "../dist/view/budgets.js";
import type { BrowserResult } from "./report.ts";
import "./types.ts";

export const VIEWPORT = { width: 1280, height: 800 } as const;

/**
 * Chrome DevTools' Fast 4G (front_end/core/sdk/NetworkManager.ts in devtools-frontend): 9 Mbit/s
 * down and 1.5 Mbit/s up, each × 0.9, with 165 ms of latency (60 ms × 2.75).
 */
const FAST_4G = {
  offline: false,
  latency: 60 * 2.75,
  downloadThroughput: ((9 * 1000 * 1000) / 8) * 0.9,
  uploadThroughput: ((1.5 * 1000 * 1000) / 8) * 0.9,
};

/** A wheel step and the pause after it: about 20,000 CSS px a second, some 25 pages. */
const STEP_PX = 400;
const STEP_MS = 20;

async function harness(context: BrowserContext, url: string, query = ""): Promise<Page> {
  const page = await context.newPage();
  await page.goto(`${url}${query}`);
  await page.waitForFunction(() => "__laminaPerf" in globalThis, undefined, { timeout: 180_000 });
  await page.mouse.move(VIEWPORT.width / 2, VIEWPORT.height / 2);
  return page;
}

/** Opens `name` at `zoom` and waits until its first screen is sharp. */
async function opened(page: Page, name: string, zoom: string): Promise<void> {
  await page.evaluate((n) => globalThis.__laminaPerf.open(n), name);
  await page.evaluate((z) => globalThis.__laminaPerf.setZoom(z), zoom);
  await page.evaluate(() => globalThis.__laminaPerf.untilSharp());
}

const scrollTop = (page: Page) => page.evaluate(() => globalThis.__laminaPerf.scroll());

/**
 * Wheels down until `fraction` of the scroll height, checking the position every 10 steps; fails
 * if 50 steps in a row don't move the view (the wheel missed it, or the browser ignored it).
 */
async function wheelTo(page: Page, fraction: number): Promise<void> {
  let previous = -1;
  let stuck = 0;
  for (;;) {
    const { top, max } = await scrollTop(page);
    if (top >= max * fraction - 1) return;
    stuck = top === previous ? stuck + 1 : 0;
    if (stuck >= 5) throw new Error(`the wheel stopped scrolling the view at ${top} of ${max} px`);
    previous = top;
    for (let i = 0; i < 10; i++) {
      await page.mouse.wheel(0, STEP_PX);
      await page.waitForTimeout(STEP_MS);
    }
  }
}

async function firstPageWarm(context: BrowserContext, url: string): Promise<number> {
  const page = await harness(context, url);
  // The first open fills the HTTP cache and warms the JIT; the second, after a reload, is measured.
  await page.evaluate(() => globalThis.__laminaPerf.open("ten"));
  await page.reload();
  await page.waitForFunction(() => "__laminaPerf" in globalThis, undefined, { timeout: 180_000 });
  const ms = await page.evaluate(() => globalThis.__laminaPerf.open("ten"));
  await page.close();
  return ms;
}

/** How the cold scenario's file reaches the reader: from the device, or from its address. */
type ColdFrom = "device" | "link";

/**
 * Chromium only: a new context with the cache off and Fast 4G, opening the file during startup.
 * The throttling reaches the document Web Worker's downloads too. A file on the device (one the
 * user chose or dropped) costs no download: its bytes come from memory, past the throttled
 * network. A linked file is read from its address with Range requests, as the reader reads a
 * `#url=` link. Before C7d the page fetched the whole file over the throttled network, a case the
 * reader never has; the first paint then also waited for the extract module, which the link
 * layer's first call needed (5.2 s; 3.3 s for the device file after).
 */
async function firstPageCold(browser: Browser, url: string, from: ColdFrom): Promise<number> {
  const context = await browser.newContext({ viewport: VIEWPORT, deviceScaleFactor: 1 });
  const page = await context.newPage();
  const cdp = await context.newCDPSession(page);
  await cdp.send("Network.enable");
  await cdp.send("Network.setCacheDisabled", { cacheDisabled: true });
  await cdp.send("Network.emulateNetworkConditions", FAST_4G);
  if (from === "device") {
    const body = await (await fetch(new URL("ten.pdf", url))).arrayBuffer();
    await context.route("**/ten.pdf", (route) =>
      route.fulfill({ body: Buffer.from(body), contentType: "application/pdf" }),
    );
  }
  await page.goto(`${url}?open=ten${from === "link" ? "&remote" : ""}`);
  await page.waitForFunction(() => "__laminaPerf" in globalThis, undefined, { timeout: 180_000 });
  const ms = await page.evaluate(() => globalThis.__laminaPerf.firstPaintMs);
  await context.close();
  if (ms === null) throw new Error(`the cold page (${from}) never painted its first page`);
  return ms;
}

/** The 1,000-page file end to end, twice; the first pass stops at a quarter, half and three quarters. */
async function scroll(context: BrowserContext, url: string, quick: boolean): Promise<BrowserResult["scroll"]> {
  const page = await harness(context, url);
  await opened(page, "thousand", "fit-page");
  const end = quick ? 0.1 : 1;
  const toSharpMs: number[] = [];
  const pass = async (stops: readonly number[]) => {
    await page.evaluate(() => globalThis.__laminaPerf.record());
    for (const stop of [...stops.map((s) => s * end), end]) {
      await wheelTo(page, stop);
      toSharpMs.push(await page.evaluate(() => globalThis.__laminaPerf.untilSharp()));
    }
    const recording = await page.evaluate(() => globalThis.__laminaPerf.stop());
    return { recording, settled: await page.evaluate(() => globalThis.__laminaPerf.memory()) };
  };
  const first = await pass([0.25, 0.5, 0.75]);
  await page.evaluate(() => globalThis.__laminaPerf.goToPage(0));
  await page.evaluate(() => globalThis.__laminaPerf.untilSharp());
  const second = await pass([]);
  const counters = await page.evaluate(() => globalThis.__laminaPerf.counters());
  await page.close();
  return { passes: [first.recording, second.recording], settled: [first.settled, second.settled], toSharpMs, counters };
}

/** The 300-page image file: big wheel steps across it (100 pages when quick), then stop. */
async function fling(context: BrowserContext, url: string, quick: boolean): Promise<BrowserResult["fling"]> {
  const page = await harness(context, url);
  await opened(page, "images", "fit-page");
  const { max } = await scrollTop(page);
  const steps = 30;
  const distance = quick ? max / 3 : max;
  for (let i = 0; i < steps; i++) {
    await page.mouse.wheel(0, distance / steps);
    await page.waitForTimeout(10);
  }
  const toSharpMs = await page.evaluate(() => globalThis.__laminaPerf.untilSharp());
  const counters = await page.evaluate(() => globalThis.__laminaPerf.counters());
  await page.close();
  return { toSharpMs, counters };
}

/** Three bursts of ctrl+wheel on the 10-page file, in then out, each followed by a stop. */
async function zoom(context: BrowserContext, url: string): Promise<BrowserResult["zoom"]> {
  const page = await harness(context, url);
  await opened(page, "ten", "fit-width");
  await page.evaluate(() => globalThis.__laminaPerf.record());
  const toSharpMs: number[] = [];
  for (let burst = 0; burst < 3; burst++) {
    for (const direction of [-1, 1]) {
      await page.keyboard.down("Control");
      for (let i = 0; i < 5; i++) {
        await page.mouse.wheel(0, direction * 120);
        await page.waitForTimeout(30);
      }
      await page.keyboard.up("Control");
      toSharpMs.push(await page.evaluate(() => globalThis.__laminaPerf.untilSharp()));
    }
  }
  const recording = await page.evaluate(() => globalThis.__laminaPerf.stop());
  await page.close();
  return { recording, toSharpMs };
}

/** Each engine's phone profile: touch and a coarse pointer, so the viewer takes its touch budgets. */
const PHONES: Record<string, string> = { chromium: "Pixel 8", webkit: "iPhone 15" };

/**
 * The 1,000-page file stepped through ten pages at a time in a phone's profile (the 100-page
 * stretch when quick), with the peak memory against the budgets the viewer gives that device.
 * Pages are jumped to rather than scrolled by touch, so every engine moves through them the same way.
 */
async function mobile(browser: Browser, name: string, url: string, quick: boolean): Promise<BrowserResult["mobile"]> {
  const device = PHONES[name];
  if (device === undefined) return null;
  const context = await browser.newContext({ ...devices[device] });
  try {
    const page = await harness(context, url);
    await opened(page, "thousand", "fit-width");
    // The budgets the viewer chose from what the page reports, so a profile without a coarse pointer shows as such.
    const hints = await page.evaluate(() => ({
      coarse: matchMedia("(pointer: coarse)").matches,
      memoryGb: (globalThis.navigator as { deviceMemory?: number }).deviceMemory,
    }));
    const budget = viewerBudgets(hints);
    await page.evaluate(() => globalThis.__laminaPerf.record());
    for (let index = 0; index < (quick ? 100 : 1000); index += 10) {
      await page.evaluate((i) => globalThis.__laminaPerf.goToPage(i), index);
      await page.waitForTimeout(STEP_MS);
    }
    await page.evaluate(() => globalThis.__laminaPerf.untilSharp());
    const { peakMemory } = await page.evaluate(() => globalThis.__laminaPerf.stop());
    return { device: hints.coarse ? device : `${device} (fine pointer)`, peakMemory, budgetBytes: budgetBytes(budget) };
  } finally {
    await context.close();
  }
}

/**
 * Find on the 1,000-page file from page 1, while the view scrolls a page every step: "worker", a
 * word on nearly every page, then "Page 778", on one page only, so its first result needs most of
 * a pass. The recording covers both searches and the scrolling.
 */
async function find(context: BrowserContext, url: string): Promise<BrowserResult["find"]> {
  const page = await harness(context, url);
  await opened(page, "thousand", "fit-width");
  await page.evaluate(() => globalThis.__laminaPerf.record());
  const scrolling = (async () => {
    for (let index = 1; index < 40; index++) {
      await page.evaluate((i) => globalThis.__laminaPerf.goToPage(i), index);
      await page.waitForTimeout(STEP_MS * 5);
    }
  })();
  const common = await page.evaluate(() => globalThis.__laminaPerf.find("worker"));
  await scrolling;
  await page.evaluate(() => globalThis.__laminaPerf.goToPage(0));
  const rare = await page.evaluate(() => globalThis.__laminaPerf.find("Page 778"));
  const recording = await page.evaluate(() => globalThis.__laminaPerf.stop());
  return { common, rare, recording };
}

/** 20 citations hovered in a row on the paper: the time from dwell to crop, and the longest task. */
async function linkPreviews(context: BrowserContext, url: string): Promise<BrowserResult["linkPreviews"]> {
  const page = await harness(context, url);
  try {
    await opened(page, "paper", "fit-width");
    await page.evaluate(() => globalThis.__laminaPerf.record());
    const toCropMs = await page.evaluate(() => globalThis.__laminaPerf.hoverLinks(20));
    const recording = await page.evaluate(() => globalThis.__laminaPerf.stop());
    return { toCropMs, recording };
  } finally {
    await page.close();
  }
}

/** Every scenario in `browser`, against the harness served at `url`. */
export async function runScenarios(
  browser: Browser,
  name: string,
  url: string,
  quick: boolean,
): Promise<Omit<BrowserResult, "browser" | "version" | "quick" | "machine">> {
  const context = await browser.newContext({ viewport: VIEWPORT, deviceScaleFactor: 1 });
  try {
    const warmMs = await firstPageWarm(context, url);
    const coldMs = name === "chromium" ? await firstPageCold(browser, url, "device") : null;
    const coldLinkMs = name === "chromium" ? await firstPageCold(browser, url, "link") : null;
    return {
      firstPage: { warmMs, coldMs, coldLinkMs },
      scroll: await scroll(context, url, quick),
      fling: await fling(context, url, quick),
      zoom: await zoom(context, url),
      find: await find(context, url),
      linkPreviews: await linkPreviews(context, url),
      mobile: await mobile(browser, name, url, quick),
    };
  } finally {
    await context.close();
  }
}
