// Overlays enter and leave in the built reader (todo U7c; motion plan §3, 3, 4 and 6): the More
// menu fades in with a slight scale and out faster, passing clicks through as it leaves; with
// reduced motion it only fades; the command palette opens and closes in the frame of its key.
// No CSP violation throughout.

import type { Browser, Page } from "playwright";
import { afterAll, beforeAll, describe, expect, inject, it } from "vitest";
import { BROWSER, launch } from "./browser.ts";
import { chooseFile, clean, load, opened, WITHOUT_PICKER } from "./reader.ts";

const origin = inject("origin");

/** The open popover's computed transition and scale, and whether it is leaving. */
function popover(page: Page) {
  return page.evaluate(() => {
    const element = document.querySelector<HTMLElement>(".paper-popover");
    if (element === null) return null;
    const style = getComputedStyle(element);
    return {
      duration: style.transitionDuration,
      property: style.transitionProperty,
      // A number: the built stylesheet is minified (".97").
      scale: Number(style.getPropertyValue("--paper-motion-scale")),
      exiting: element.hasAttribute("data-exiting"),
      pointerEvents: style.pointerEvents,
    };
  });
}

/** Opens the top bar's More menu and waits until it has faded in. */
async function openMore(page: Page): Promise<void> {
  await page.getByRole("button", { name: "More" }).first().click();
  await page.getByRole("menu").waitFor();
  await page.evaluate(() =>
    Promise.all(
      document
        .querySelector(".paper-popover")
        ?.getAnimations()
        .map((a) => a.finished) ?? [],
    ),
  );
}

describe(`overlay motion in ${BROWSER}`, () => {
  let browser: Browser;

  beforeAll(async () => {
    browser = await launch();
  });

  afterAll(async () => {
    await browser?.close();
  });

  it("fades a menu in with a scale and out faster, passing clicks through as it leaves", async () => {
    // No preference, whatever the machine running the test prefers.
    const watched = await load(browser, origin, "/", WITHOUT_PICKER, {
      ignoreHTTPSErrors: true,
      reducedMotion: "no-preference",
    });
    const { page } = watched;
    await chooseFile(watched, "hello.pdf");
    await opened(watched, "hello.pdf");
    await openMore(page);
    expect(await popover(page)).toMatchObject({ duration: "0.12s", property: "opacity, scale", scale: 0.97 });
    await page.keyboard.press("Escape");
    expect(await popover(page)).toMatchObject({ duration: "0.08s", exiting: true, pointerEvents: "none" });
    await expect.poll(() => popover(page)).toBeNull();
    await clean(watched);
  });

  it("only fades a menu with reduced motion", async () => {
    const watched = await load(browser, origin, "/", WITHOUT_PICKER, {
      ignoreHTTPSErrors: true,
      reducedMotion: "reduce",
    });
    const { page } = watched;
    await chooseFile(watched, "hello.pdf");
    await opened(watched, "hello.pdf");
    await openMore(page);
    // The fade stays; the scale it grows from is 1, so nothing moves.
    expect(await popover(page)).toMatchObject({ duration: "0.12s", scale: 1 });
    await page.keyboard.press("Escape");
    expect(await popover(page)).toMatchObject({ exiting: true });
    await expect.poll(() => popover(page)).toBeNull();
    await clean(watched);
  });

  it("opens and closes the command palette in the frame of its key", async () => {
    const watched = await load(browser, origin, "/", WITHOUT_PICKER, { ignoreHTTPSErrors: true });
    const { page } = watched;
    await chooseFile(watched, "hello.pdf");
    await opened(watched, "hello.pdf");
    await page.locator(".web-viewer").click();
    await page.keyboard.press("ControlOrMeta+K");
    const dialog = page.locator(".paper-palette-overlay");
    await dialog.waitFor();
    expect(await dialog.evaluate((e) => getComputedStyle(e).transitionDuration)).toBe("0s");
    await page.keyboard.press("Escape");
    expect(await page.locator(".paper-palette-overlay").count()).toBe(0);
    await clean(watched);
  });
});
