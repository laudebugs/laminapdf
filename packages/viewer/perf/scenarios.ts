// The harness's scenarios, each on a fresh page of the harness (page/main.ts), driven through
// `window.__laminaPerf` with Playwright's real input: wheel steps at fixed intervals, so two runs
// scroll the same way.

import type { Browser, BrowserContext, Page } from "playwright";
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

/** Wheels down until `fraction` of the scroll height, checking the position every 10 steps. */
async function wheelTo(page: Page, fraction: number): Promise<void> {
  for (;;) {
    const { top, max } = await scrollTop(page);
    if (top >= max * fraction - 1) return;
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

/** Chromium only: a new context with the cache off and Fast 4G, opening the file during startup. */
async function firstPageCold(browser: Browser, url: string): Promise<number> {
  const context = await browser.newContext({ viewport: VIEWPORT, deviceScaleFactor: 1 });
  const page = await context.newPage();
  const cdp = await context.newCDPSession(page);
  await cdp.send("Network.enable");
  await cdp.send("Network.setCacheDisabled", { cacheDisabled: true });
  await cdp.send("Network.emulateNetworkConditions", FAST_4G);
  await page.goto(`${url}?open=ten`);
  await page.waitForFunction(() => "__laminaPerf" in globalThis, undefined, { timeout: 180_000 });
  const ms = await page.evaluate(() => globalThis.__laminaPerf.firstPaintMs);
  await context.close();
  if (ms === null) throw new Error("the cold page never painted its first page");
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
    const coldMs = name === "chromium" ? await firstPageCold(browser, url) : null;
    return {
      firstPage: { warmMs, coldMs },
      scroll: await scroll(context, url, quick),
      fling: await fling(context, url, quick),
      zoom: await zoom(context, url),
    };
  } finally {
    await context.close();
  }
}
