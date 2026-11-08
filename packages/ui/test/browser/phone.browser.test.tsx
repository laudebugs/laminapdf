// The phone shell on the demo page (design language §6.2, R18): below the phone breakpoint the
// bottom bar replaces the status bar, Pages opens the rail as the Contents sheet, More opens a sheet
// of the More items, the top bar keeps only the file name and More and slides away while the
// document scrolls down; nothing scrolls sideways at 320 wide, every target is 44 px at touch
// density, and axe finds nothing serious.

import axe from "axe-core";
import { afterEach, beforeEach, describe, expect, it } from "vitest";
import { page, userEvent } from "vitest/browser";
import { Demo } from "../../examples/ui-demo.tsx";
import {
  CommandRegistry,
  CommandsProvider,
  PlatformProvider,
  ReaderController,
  ReaderTopBar,
} from "../../src/index.ts";
import { fakePlatform } from "../fakes.ts";
import { cleanup, renderIn, targets } from "./render.tsx";

beforeEach(async () => {
  await page.viewport(390, 844);
});

afterEach(async () => {
  cleanup();
  await page.viewport(1280, 800);
});

async function openDemo() {
  const platform = fakePlatform({
    picked: [{ name: "report.pdf", size: 5, read: async () => new Uint8Array([37, 80, 68, 70, 45]), writable: false }],
  });
  const host = renderIn(
    <div style={{ blockSize: "844px" }}>
      <PlatformProvider platform={platform}>
        <Demo platform={platform} />
      </PlatformProvider>
    </div>,
    { density: "touch" },
  );
  await userEvent.click(button("Choose a file"));
  await expect.poll(() => document.querySelector(".demo-page")).not.toBeNull();
  return host;
}

function button(name: string, within: ParentNode = document): HTMLElement {
  const found = [...within.querySelectorAll<HTMLElement>("button, [role=button]")].find(
    (b) => b.getAttribute("aria-label") === name || b.textContent?.trim() === name,
  );
  if (found === undefined) throw new Error(`no button "${name}"`);
  return found;
}

const bottomBar = () => document.querySelector<HTMLElement>("[role=toolbar][aria-label=Document]");

describe("the phone shell", () => {
  it("puts Pages and More in a bottom bar in place of the status bar, and trims the top bar to the file and More", async () => {
    await openDemo();
    const bar = bottomBar();
    const header = document.querySelector("header") as HTMLElement;
    expect([
      [...(bar?.querySelectorAll("button") ?? [])].map((b) => b.textContent),
      document.body.textContent?.includes("0 network requests"),
      header.querySelector(".paper-commands-button"),
      header.textContent?.includes("report.pdf"),
    ]).toEqual([["Pages", "More"], false, null, true]);
  });

  it("opens the rail as the Contents sheet from Pages, and closes it with Escape", async () => {
    await openDemo();
    await userEvent.click(button("Pages", bottomBar() as HTMLElement));
    await expect.poll(() => document.querySelector("[role=dialog]")?.textContent?.includes("Contents")).toBe(true);
    expect(document.querySelector("[role=dialog] [role=tablist]")).not.toBeNull();
    await userEvent.keyboard("{Escape}");
    await expect.poll(() => document.querySelector("[role=dialog]")).toBeNull();
  });

  it("opens the More items as a sheet from the bottom bar and from the top bar, and runs the one chosen", async () => {
    await openDemo();
    for (const within of [bottomBar() as HTMLElement, document.querySelector("header") as HTMLElement]) {
      await userEvent.click(button("More", within));
      await expect.poll(() => document.querySelector("[role=dialog] [role=menu]")).not.toBeNull();
      const items = [...document.querySelectorAll("[role=dialog] [role=menuitem]")].map((i) => i.textContent);
      expect(items[0]).toContain("Open another file");
      await userEvent.keyboard("{Escape}");
      await expect.poll(() => document.querySelector("[role=dialog]")).toBeNull();
    }
    await userEvent.click(button("More", bottomBar() as HTMLElement));
    await expect.poll(() => document.querySelector("[role=dialog] [role=menuitem]")).not.toBeNull();
    await userEvent.click(document.querySelector<HTMLElement>("[role=dialog] [role=menuitem]") as HTMLElement);
    await expect.poll(() => document.querySelector(".demo-page")).toBeNull();
  });

  it("shows the inspector as a sheet that leaves the page in use, closed by Escape", async () => {
    await openDemo();
    await userEvent.click(button("Select the heading"));
    const sheet = () => [...document.querySelectorAll(".paper-sheet")].find((s) => s.textContent?.includes("Heading"));
    await expect.poll(sheet).toBeDefined();
    expect(document.querySelector("[role=dialog]")).toBeNull();
    await userEvent.keyboard("{Escape}");
    await expect.poll(sheet).toBeUndefined();
  });

  it("never scrolls sideways at 320 wide, with or without a sheet open", async () => {
    await page.viewport(320, 640);
    await openDemo();
    // Clipping would hide an overflow: nothing in the bars may reach past the window either.
    const wide = () => document.documentElement.scrollWidth;
    const past = () =>
      [...document.querySelectorAll("header *, footer *, .paper-sheet *")]
        .filter((el) => el.getBoundingClientRect().right > 320.5)
        .map((el) => el.className || el.tagName);
    const closed = [wide(), past()];
    await userEvent.click(button("More", bottomBar() as HTMLElement));
    await expect.poll(() => document.querySelector("[role=dialog]")).not.toBeNull();
    expect([closed, [wide(), past()]]).toEqual([
      [320, []],
      [320, []],
    ]);
  });

  it("makes every target in the bars at least 44 × 44 at touch density", async () => {
    await openDemo();
    const bars = [document.querySelector("header"), document.querySelector("footer")] as HTMLElement[];
    const small = bars.flatMap(targets).flatMap((el) => {
      const { width, height } = el.getBoundingClientRect();
      return width + 0.5 < 44 || height + 0.5 < 44
        ? [`${el.getAttribute("aria-label") ?? el.textContent}: ${width}×${height}`]
        : [];
    });
    expect(small).toEqual([]);
  });

  it("passes axe with the bars and with the More sheet open", async () => {
    await openDemo();
    const serious = async () =>
      (await axe.run(document.body)).violations
        .filter((v) => v.impact === "serious" || v.impact === "critical")
        .map((v) => v.id);
    const bars = await serious();
    await userEvent.click(button("More", bottomBar() as HTMLElement));
    await expect.poll(() => document.querySelector("[role=dialog]")).not.toBeNull();
    expect([bars, await serious()]).toEqual([[], []]);
  });
});

describe("the phone's top bar", () => {
  it("slides away while the document scrolls down, and comes back as soon as it scrolls up", async () => {
    const reader = new ReaderController(fakePlatform(), async () => ({ pageCount: 3 }) as never);
    let state = { pageCount: 3, currentPage: 0, zoom: "fit-width" as const, percent: 100, scrollTop: 0 };
    const listeners = new Set<() => void>();
    const viewer = {
      getState: () => state,
      subscribe: (l: () => void) => {
        listeners.add(l);
        return () => listeners.delete(l);
      },
      goToPage: () => undefined,
      setZoom: () => undefined,
      rotate: () => undefined,
    };
    const scroll = async (top: number) => {
      state = { ...state, scrollTop: top };
      for (const l of listeners) l();
      await new Promise((resolve) => requestAnimationFrame(resolve));
    };
    renderIn(
      <CommandsProvider registry={new CommandRegistry()} context={{ platform: fakePlatform(), reader }} keys="other">
        <ReaderTopBar reader={reader} viewer={viewer} keys="other" more={{ items: [], onAction: () => undefined }} />
      </CommandsProvider>,
    );
    const hidden = () => document.querySelector("header")?.hasAttribute("data-hidden");
    const settle = () => new Promise((resolve) => setTimeout(resolve, 450));
    await scroll(10);
    const small = hidden();
    await scroll(200);
    await expect.poll(hidden).toBe(true);
    const down = hidden();
    // The viewer re-fitting to the taller canvas moves the scroll up a little: the bar stays hidden.
    await scroll(172);
    const refit = hidden();
    await settle();
    await scroll(160);
    await expect.poll(hidden).toBe(false);
    const up = hidden();
    expect([small, down, refit, up, document.querySelector("header")?.inert]).toEqual([
      false,
      true,
      true,
      false,
      false,
    ]);
    // A zoom that scrolls (a pinch keeping its point still) leaves the bar where it is.
    await settle();
    state = { ...state, percent: 175 };
    await scroll(600);
    expect(hidden()).toBe(false);
  });
});
