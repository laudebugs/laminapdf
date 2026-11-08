// What the specs visit: every route, once per overlay state, and how to settle a page before it is
// screenshot or checked.

import type { Page } from "@playwright/test";
import { ROUTES, type Settings, settingsQuery } from "../src/routes.ts";

/** One thing to look at: a route, and an overlay state when the route has them. */
export interface View {
  route: string;
  state: string;
  pageRelated: boolean;
  /** A file-name-safe name: `buttons`, `dialogs-password`. */
  name: string;
}

/** Every view of the gallery. */
export const VIEWS: readonly View[] = ROUTES.flatMap((route) =>
  (route.states ?? [""]).map((state) => ({
    route: route.id,
    state,
    pageRelated: route.pageRelated === true,
    name: state === "" ? route.id : `${route.id}-${state}`,
  })),
);

/**
 * Opens `view` with `settings`, at a fixed time, once its fonts have loaded and the page has
 * settled; the tooltip state is shown by moving keyboard focus onto its button. A page error or a
 * page that never renders its stage fails the test, so a crash can't become a blank screenshot.
 */
export async function open(page: Page, view: View, settings: Partial<Settings> = {}): Promise<void> {
  const errors: string[] = [];
  page.on("pageerror", (error) => errors.push(error.message));
  await page.clock.setFixedTime(new Date("2026-01-15T09:30:00Z"));
  await page.goto(settingsQuery({ ...settings, route: view.route, state: view.state }) || "/");
  await page.locator("#stage").waitFor();
  await page.evaluate(() => document.fonts.ready);
  if (view.name === "menus-tooltip") {
    await page.locator("#g-state").focus();
    await page.keyboard.press("Tab");
    await page.getByRole("tooltip").waitFor();
  }
  if (view.route === "menus" && view.state !== "tooltip")
    await page.getByRole(view.state === "menu" ? "menu" : "dialog").waitFor();
  await page.evaluate(() => new Promise((resolve) => requestAnimationFrame(() => requestAnimationFrame(resolve))));
  if (errors.length > 0) throw new Error(`${view.name}: the page threw: ${errors.join("; ")}`);
}
