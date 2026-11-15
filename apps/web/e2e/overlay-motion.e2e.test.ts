// Overlays enter and leave in the built reader (todo U7c; motion plan §3, 3, 4 and 6): the More
// menu fades in with a slight scale and out faster, passing clicks through as it leaves; with
// reduced motion it only fades; the command palette opens and closes in the frame of its key.
// No CSP violation throughout.

import type { Browser, Page } from "playwright";
import { afterAll, beforeAll, describe, expect, inject, it } from "vitest";
import { BROWSER, launch } from "./browser.ts";
import { chooseFile, clean, load, opened, WITHOUT_PICKER } from "./reader.ts";

const origin = inject("origin");

/**
 * Records the first computed exit of an element matching `selector`: its transition duration and
 * pointer-events when it is marked data-exiting. A single read after the key press could miss a
 * 150 ms exit on a slow runner; the recorder sees it whenever it happens.
 */
function recordExit(page: Page, selector: string) {
  return page.evaluate((selector) => {
    const seen: { duration: string; pointerEvents: string }[] = [];
    Object.assign(window, { exitSeen: seen });
    new MutationObserver(() => {
      const element = document.querySelector(`${selector}[data-exiting]`);
      if (element === null || seen.length > 0) return;
      const style = getComputedStyle(element);
      seen.push({ duration: style.transitionDuration.split(", ")[0] ?? "", pointerEvents: style.pointerEvents });
    }).observe(document.body, { subtree: true, attributes: true, attributeFilter: ["data-exiting"] });
  }, selector);
}

/** The exit `recordExit` saw, once there was one. */
const exitSeen = (page: Page) =>
  page.evaluate(
    () => (window as unknown as { exitSeen: { duration: string; pointerEvents: string }[] }).exitSeen[0] ?? null,
  );

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
    await recordExit(page, ".paper-popover");
    await page.keyboard.press("Escape");
    await expect.poll(() => exitSeen(page)).toEqual({ duration: "0.08s", pointerEvents: "none" });
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
    await recordExit(page, ".paper-popover");
    await page.keyboard.press("Escape");
    await expect.poll(() => exitSeen(page)).not.toBeNull();
    await expect.poll(() => popover(page)).toBeNull();
    await clean(watched);
  });

  for (const reduced of [false, true]) {
    it(`fades a dialog in and out over its scrim${reduced ? ", with no scale under reduced motion" : ", growing from 0.97"}`, async () => {
      const watched = await load(browser, origin, "/", WITHOUT_PICKER, {
        ignoreHTTPSErrors: true,
        reducedMotion: reduced ? "reduce" : "no-preference",
      });
      const { page } = watched;
      await chooseFile(watched, "hello.pdf");
      await opened(watched, "hello.pdf");
      await page.getByRole("button", { name: "More" }).first().click();
      await page.getByRole("menuitem", { name: "Settings" }).click();
      await page.getByRole("dialog", { name: "Settings" }).waitFor();
      // Faded in before it closes: closing in the frame it opens would leave nothing to fade out.
      await page.evaluate(() =>
        Promise.all(
          document
            .getAnimations()
            .filter((a) => a.effect?.getComputedTiming().endTime !== Number.POSITIVE_INFINITY)
            // A cancelled transition (its target changed) rejects: it has stopped all the same.
            .map((a) => a.finished.catch(() => undefined)),
        ),
      );
      const frame = () =>
        page.evaluate(() => {
          const overlay = document.querySelector(".paper-modal-overlay");
          const frame = document.querySelector(".paper-dialog-frame");
          if (overlay === null || frame === null) return null;
          return {
            fade: getComputedStyle(overlay).transitionDuration,
            exiting: overlay.hasAttribute("data-exiting"),
            scale: Number(getComputedStyle(frame).getPropertyValue("--paper-motion-scale")),
          };
        });
      expect(await frame()).toMatchObject({ fade: "0.2s", scale: reduced ? 1 : 0.97 });
      await recordExit(page, ".paper-modal-overlay");
      await page.keyboard.press("Escape");
      await expect.poll(() => exitSeen(page)).toEqual({ duration: "0.15s", pointerEvents: "none" });
      await expect.poll(frame).toBeNull();
      await clean(watched);
    });
  }

  it("slides the rail drawer in and out over its fading scrim on a narrow window", async () => {
    const watched = await load(browser, origin, "/", WITHOUT_PICKER, {
      ignoreHTTPSErrors: true,
      reducedMotion: "no-preference",
      viewport: { width: 900, height: 700 },
    });
    const { page } = watched;
    await chooseFile(watched, "hello.pdf");
    await opened(watched, "hello.pdf");
    const drawer = () =>
      page.evaluate(() => {
        const rail = document.querySelector(".paper-left-rail[data-drawer]");
        const scrim = document.querySelector(".paper-rail-scrim");
        if (rail === null || scrim === null) return null;
        return {
          property: getComputedStyle(rail).transitionProperty,
          scrim: getComputedStyle(scrim).transitionProperty,
          exiting: rail.hasAttribute("data-exiting") && scrim.hasAttribute("data-exiting"),
        };
      });
    // The rail may start open: close it first, then open it as a person would.
    if ((await drawer()) !== null) {
      await page.keyboard.press("Escape");
      await expect.poll(drawer).toBeNull();
    }
    await page.getByRole("button", { name: "Sidebar" }).click();
    await expect.poll(drawer).toMatchObject({ property: "translate, opacity", scrim: "opacity", exiting: false });
    await recordExit(page, ".paper-left-rail");
    await page.keyboard.press("Escape");
    await expect.poll(() => exitSeen(page)).toMatchObject({ pointerEvents: "none" });
    await expect.poll(drawer).toBeNull();
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
