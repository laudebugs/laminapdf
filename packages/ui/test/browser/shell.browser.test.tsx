// The inverted-L shell on the demo page (ADR 0024 §1, UI/UX plan §5.1): only the top bar, the canvas
// and the status bar until the rail is opened; the inspector takes its width from the canvas and
// never covers it; the rail resizes by pointer and keys; focus mode by its shortcut and the palette;
// the drawer below the rail breakpoint; one landmark of each kind, and axe.

import axe from "axe-core";
import { afterEach, describe, expect, it } from "vitest";
import { page, userEvent } from "vitest/browser";
import { Demo } from "../../examples/ui-demo.tsx";
import { ModeRegistry, PlatformProvider, RAIL_DEFAULT_WIDTH, ReaderShell, ShellLayout } from "../../src/index.ts";
import { fakePlatform } from "../fakes.ts";
import { cleanup, renderIn } from "./render.tsx";

afterEach(async () => {
  cleanup();
  await page.viewport(1280, 800);
});

const frame = () => new Promise((resolve) => requestAnimationFrame(resolve));

/** The demo with `report.pdf` open (the platform's storage given, to keep the rail between renders). */
async function openDemo(platform = pickerFor("report.pdf")) {
  renderIn(
    <div style={{ blockSize: "700px" }}>
      <PlatformProvider platform={platform}>
        <Demo platform={platform} />
      </PlatformProvider>
    </div>,
  );
  await userEvent.click(button("Choose a file"));
  await expect.poll(() => document.querySelector(".demo-page")).not.toBeNull();
  return platform;
}

function pickerFor(name: string) {
  return fakePlatform({
    picked: [{ name, size: 5, read: async () => new Uint8Array([37, 80, 68, 70, 45]), writable: false }],
  });
}

function button(name: string): HTMLElement {
  const found = [...document.querySelectorAll<HTMLElement>("button, [role=button]")].find(
    (b) => b.getAttribute("aria-label") === name || b.textContent?.trim() === name,
  );
  if (found === undefined) throw new Error(`no button "${name}"`);
  return found;
}

const canvas = () => document.querySelector("main") as HTMLElement;
/** Focus mode's status-bar item, by its name: the label and the shortcut chip's text. */
const focusItem = () => button("Focus mode Ctrl \\");
const landmarks = (selector: string) => document.querySelectorAll(selector).length;

async function seriousViolations(): Promise<string[]> {
  const { violations } = await axe.run(document.body, { resultTypes: ["violations"] });
  return violations
    .filter((v) => v.impact === "serious" || v.impact === "critical")
    .map((v) => `${v.id}: ${v.nodes.map((n) => n.target.join(" ")).join(", ")}`);
}

describe("the shell", () => {
  it("opens a document with only the top bar, the canvas and the status bar", async () => {
    await openDemo();
    expect([landmarks("header"), landmarks("main"), landmarks("footer"), landmarks("nav"), landmarks("aside")]).toEqual(
      [1, 1, 1, 0, 0],
    );
  });

  it("opens the rail where the user left it open, at the width they gave it", async () => {
    const platform = pickerFor("report.pdf");
    await platform.storage.set("shell.rail", { open: true, width: 300, tab: "outline" });
    await openDemo(platform);
    await expect.poll(() => document.querySelector("nav.paper-left-rail")).not.toBeNull();
    const rail = document.querySelector("nav.paper-left-rail") as HTMLElement;
    expect([Math.round(rail.getBoundingClientRect().width), rail.textContent?.includes("Regions")]).toEqual([
      300,
      true,
    ]);
  });

  it("docks the inspector by narrowing the canvas, never over it", async () => {
    await openDemo();
    const before = canvas().getBoundingClientRect();
    await userEvent.click(button("Select the heading"));
    await expect.poll(() => document.querySelector("aside")).not.toBeNull();
    // Wait out the inspector's growing animation, where motion is allowed.
    await new Promise((resolve) => setTimeout(resolve, 250));
    const after = canvas().getBoundingClientRect();
    const inspector = (document.querySelector("aside") as HTMLElement).getBoundingClientRect();
    expect(after.width).toBeLessThan(before.width);
    expect(inspector.left).toBeGreaterThanOrEqual(after.right - 0.5);
  });

  it("closes a selection's inspector on Escape, and gives the canvas its width back", async () => {
    await openDemo();
    const before = canvas().getBoundingClientRect().width;
    await userEvent.click(button("Select the heading"));
    await expect.poll(() => document.querySelector("aside")).not.toBeNull();
    await userEvent.keyboard("{Escape}");
    await expect.poll(() => document.querySelector("aside")).toBeNull();
    expect(canvas().getBoundingClientRect().width).toBe(before);
  });

  it("resizes the rail with the arrow keys on its handle", async () => {
    await openDemo();
    await userEvent.click(button("Sidebar"));
    await expect.poll(() => document.querySelector("[role=separator]")).not.toBeNull();
    const separator = document.querySelector<HTMLElement>("[role=separator]") as HTMLElement;
    separator.focus();
    await userEvent.keyboard("{ArrowRight}{ArrowRight}");
    expect(separator.getAttribute("aria-valuenow")).toBe(String(RAIL_DEFAULT_WIDTH + 32));
    await userEvent.keyboard("{Home}");
    expect(Number(separator.getAttribute("aria-valuenow"))).toBeLessThan(RAIL_DEFAULT_WIDTH);
  });

  it("resizes the rail by dragging its handle", async () => {
    await openDemo();
    await userEvent.click(button("Sidebar"));
    await expect.poll(() => document.querySelector("[role=separator]")).not.toBeNull();
    const separator = document.querySelector<HTMLElement>("[role=separator]") as HTMLElement;
    const box = separator.getBoundingClientRect();
    const at = { clientX: box.left + box.width / 2, clientY: box.top + 50, pointerId: 1, button: 0, bubbles: true };
    separator.dispatchEvent(new PointerEvent("pointerdown", at));
    separator.dispatchEvent(new PointerEvent("pointermove", { ...at, clientX: at.clientX + 40 }));
    separator.dispatchEvent(new PointerEvent("pointerup", { ...at, clientX: at.clientX + 40 }));
    await frame();
    expect(separator.getAttribute("aria-valuenow")).toBe(String(RAIL_DEFAULT_WIDTH + 40));
  });

  it("turns focus mode on by its shortcut, hiding the rail, and Escape turns it off", async () => {
    await openDemo();
    await userEvent.click(button("Sidebar"));
    await expect.poll(() => document.querySelector("nav.paper-left-rail")).not.toBeNull();
    (document.querySelector("main") as HTMLElement).focus();
    await userEvent.keyboard("{Control>}\\{/Control}");
    await expect.poll(() => document.querySelector("nav.paper-left-rail")).toBeNull();
    expect(focusItem().getAttribute("aria-pressed")).toBe("true");
    await userEvent.keyboard("{Escape}");
    await expect.poll(() => document.querySelector("nav.paper-left-rail")).not.toBeNull();
  });

  it("names the focus mode item by all its visible text, its shortcut included", async () => {
    await openDemo();
    canvas().focus();
    await userEvent.keyboard("{Control>}\\{/Control}");
    await expect.poll(() => document.querySelector(".paper-reader-shell[data-focus]")).not.toBeNull();
    // Found by its place, not its name, so the name is what the rule checks.
    const item = document.querySelector("footer [aria-pressed=true]") as HTMLElement;
    // label-content-name-mismatch is experimental in axe, so off unless asked for.
    const { violations } = await axe.run(item, {
      runOnly: { type: "rule", values: ["label-content-name-mismatch"] },
    });
    expect(violations.map((v) => v.id)).toEqual([]);
    // A space keeps the label and the keys apart in the name.
    expect(item.textContent).toBe("Focus mode Ctrl \\");
  });

  it("turns focus mode on from the palette", async () => {
    await openDemo();
    await userEvent.keyboard("{Control>}k{/Control}");
    await expect.poll(() => document.querySelector("[role=dialog]")).not.toBeNull();
    await userEvent.keyboard("focus mode");
    await userEvent.keyboard("{Enter}");
    await expect.poll(() => document.querySelector(".paper-reader-shell[data-focus]")).not.toBeNull();
  });

  it("keeps the selection's inspector in focus mode, and Escape deselects before leaving focus mode", async () => {
    await openDemo();
    (document.querySelector("main") as HTMLElement).focus();
    await userEvent.keyboard("{Control>}\\{/Control}");
    await userEvent.click(button("Select the heading"));
    await expect.poll(() => document.querySelector("aside")).not.toBeNull();
    await userEvent.keyboard("{Escape}");
    await expect.poll(() => document.querySelector("aside")).toBeNull();
    expect(document.querySelector(".paper-reader-shell[data-focus]")).not.toBeNull();
  });

  it("clears a selection made before focus mode on the first Escape, and leaves focus mode on the second", async () => {
    await openDemo();
    await userEvent.click(button("Select the heading"));
    await expect.poll(() => document.querySelector("aside")).not.toBeNull();
    await userEvent.keyboard("{Control>}\\{/Control}");
    await expect.poll(() => document.querySelector(".paper-reader-shell[data-focus]")).not.toBeNull();
    await userEvent.keyboard("{Escape}");
    await expect.poll(() => document.querySelector("aside")).toBeNull();
    expect(document.querySelector(".paper-reader-shell[data-focus]")).not.toBeNull();
    await userEvent.keyboard("{Escape}");
    await expect.poll(() => document.querySelector(".paper-reader-shell[data-focus]")).toBeNull();
  });

  it("hides a panel opened on purpose in focus mode, and brings it back after", async () => {
    await openDemo(pickerFor("damaged.pdf"));
    await expect.poll(() => document.querySelector("aside")).not.toBeNull();
    (document.querySelector("main") as HTMLElement).focus();
    await userEvent.keyboard("{Control>}\\{/Control}");
    await expect.poll(() => document.querySelector("aside")).toBeNull();
    await userEvent.click(focusItem());
    await expect.poll(() => document.querySelector("aside")).not.toBeNull();
  });

  it("shows the current mode's toolbar under the top bar, and hides it in focus mode", async () => {
    const layout = new ShellLayout();
    const modes = new ModeRegistry();
    modes.register({
      id: "comment",
      title: "Comment",
      toolbar: () => <div role="toolbar" aria-label="Comment tools" />,
    });
    renderIn(
      <ReaderShell layout={layout} modes={modes} topBar={<header />}>
        <div />
      </ReaderShell>,
    );
    await expect.poll(() => document.querySelector(".paper-reader-secondary [role=toolbar]")).not.toBeNull();
    layout.toggleFocus();
    await expect.poll(() => document.querySelector(".paper-reader-secondary")).toBeNull();
  });

  it("shows the rail as a drawer over the canvas below the rail breakpoint, closed by Escape", async () => {
    await page.viewport(800, 800);
    await openDemo();
    await userEvent.click(button("Sidebar"));
    await expect.poll(() => document.querySelector("nav[data-drawer]")).not.toBeNull();
    expect(document.querySelector("[role=separator]")).toBeNull();
    await userEvent.keyboard("{Escape}");
    await expect.poll(() => document.querySelector("nav[data-drawer]")).toBeNull();
  });

  it("has one landmark of each kind with the rail and inspector open, and passes axe", async () => {
    await openDemo();
    await userEvent.click(button("Sidebar"));
    await userEvent.click(button("Select the heading"));
    await expect.poll(() => document.querySelector("aside")).not.toBeNull();
    // The inspector grows in, overflowing its text until it is full width: check it settled.
    await new Promise((resolve) => setTimeout(resolve, 250));
    expect([landmarks("header"), landmarks("nav"), landmarks("main"), landmarks("aside"), landmarks("footer")]).toEqual(
      [1, 1, 1, 1, 1],
    );
    expect(await seriousViolations()).toEqual([]);
  });
});
