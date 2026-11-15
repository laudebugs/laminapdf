// Overlays enter and leave (design language §7.0, motion plan §3, 3, 4 and 6): small overlays
// fade with a slight scale, panels fade, the palette stays instant, a leaving overlay passes
// clicks through, and reduced motion keeps the fade without the scale.

import { useState } from "react";
import { afterEach, describe, expect, it, onTestFinished, vi } from "vitest";
import { userEvent } from "vitest/browser";
import { Button, Menu, MenuTrigger, Presence, SelectionToolbar, ToastQueue, ToastRegion } from "../../src/index.ts";
import { cleanup, renderIn, settled } from "./render.tsx";

afterEach(cleanup);

const style = (el: Element) => getComputedStyle(el);
const popover = () => document.querySelector(".paper-popover") as HTMLElement | null;

function MoreMenu({ onBehind }: { onBehind: () => void }) {
  return (
    <div style={{ position: "relative", padding: "8px" }}>
      <MenuTrigger>
        <Button>More</Button>
        <Menu aria-label="More" items={[{ id: "a", label: "Rename" }]} onAction={() => {}} platform="other" />
      </MenuTrigger>
      {/* Under where the menu opens, so a click during its exit lands here. */}
      <button
        type="button"
        style={{ display: "block", marginBlockStart: "4px", blockSize: "120px", inlineSize: "240px" }}
        onClick={onBehind}
      >
        Behind
      </button>
    </div>
  );
}

/** A selection toolbar its button clears, inside Presence. */
function Toolbar() {
  const [open, setOpen] = useState(true);
  return (
    <>
      <button type="button" onClick={() => setOpen(false)}>
        Clear
      </button>
      <Presence value={open || null}>
        {() => (
          <SelectionToolbar
            aria-label="Selection"
            anchor={() => ({ top: 200, left: 100, width: 80, height: 20 })}
            onDismiss={() => {}}
          >
            <Button>Copy</Button>
          </SelectionToolbar>
        )}
      </Presence>
    </>
  );
}

describe("small overlays", () => {
  it("fade in with a scale over the small enter time, and out over the small exit time", async () => {
    renderIn(<MoreMenu onBehind={() => {}} />);
    await userEvent.click(document.querySelector("button") as HTMLElement);
    // Opened and faded in: closing in the frame it opens would leave nothing to fade out.
    await settled();
    const menu = popover() as HTMLElement;
    expect(style(menu).transitionProperty).toBe("opacity, scale");
    expect(style(menu).transitionDuration).toBe("0.12s");
    await userEvent.keyboard("{Escape}");
    expect(menu.hasAttribute("data-exiting")).toBe(true);
    expect(style(menu).transitionDuration).toBe("0.08s");
    expect(style(menu).pointerEvents).toBe("none");
    await expect.poll(popover).toBeNull();
  });

  it("let a click during their exit reach what is under them", async () => {
    const behind = vi.fn();
    renderIn(<MoreMenu onBehind={behind} />);
    await userEvent.click(document.querySelector("button") as HTMLElement);
    // Opened and faded in: closing in the frame it opens would leave nothing to fade out.
    await settled();
    const box = (popover() as HTMLElement).getBoundingClientRect();
    await userEvent.keyboard("{Escape}");
    const under = document.elementFromPoint(box.left + box.width / 2, box.top + box.height / 2);
    expect(under?.textContent).toBe("Behind");
    (under as HTMLElement).click();
    expect(behind).toHaveBeenCalledOnce();
  });

  it("only fade under reduced motion: they keep their scale at 1", async () => {
    document.documentElement.setAttribute("data-motion", "reduce");
    onTestFinished(() => document.documentElement.removeAttribute("data-motion"));
    renderIn(<MoreMenu onBehind={() => {}} />);
    await userEvent.click(document.querySelector("button") as HTMLElement);
    // Opened and faded in: closing in the frame it opens would leave nothing to fade out.
    await settled();
    await userEvent.keyboard("{Escape}");
    const menu = popover() as HTMLElement;
    expect(menu.hasAttribute("data-exiting")).toBe(true);
    // The exiting rule's target: opacity 0, scale motion-scale, which reduced motion makes 1.
    expect(style(menu).getPropertyValue("--paper-motion-scale").trim()).toBe("1");
  });
});

describe("Presence", () => {
  const toolbar = () => document.querySelector(".paper-selection-toolbar") as HTMLElement | null;

  it("keeps an overlay for its exit, marked exiting and inert, then removes it", async () => {
    renderIn(<Toolbar />);
    // Opened and faded in: closing in the frame it opens would leave nothing to fade out.
    await settled();
    expect(toolbar()?.hasAttribute("data-exiting")).toBe(false);
    // A click in the page, read a frame later: a test runner's click can take longer than the
    // exit itself (Firefox in CI), and React renders the exit in a microtask after the click.
    (document.querySelector("button") as HTMLElement).click();
    await new Promise(requestAnimationFrame);
    expect(toolbar()?.hasAttribute("data-exiting")).toBe(true);
    expect(toolbar()?.inert).toBe(true);
    await expect.poll(toolbar).toBeNull();
  });
});

describe("the command palette", () => {
  it("has no transition: it opens and closes in the frame of its shortcut", () => {
    const overlay = document.createElement("div");
    overlay.className = "paper-palette-overlay";
    renderIn(<span />).append(overlay);
    expect(style(overlay).transitionDuration).toBe("0s");
  });
});

describe("toasts", () => {
  it("leave the way they came: marked exiting, then gone, with focus back where it was", async () => {
    const queue = new ToastQueue();
    renderIn(
      <>
        <button type="button">Before</button>
        <ToastRegion queue={queue} aria-label="Notifications" />
      </>,
    );
    queue.add({ message: "Copied", action: { label: "Undo", onPress: () => {} }, timeout: Number.POSITIVE_INFINITY });
    await expect.poll(() => document.querySelector(".paper-toast")).not.toBeNull();
    (document.querySelector("button") as HTMLElement).focus();
    await userEvent.tab();
    expect(document.activeElement?.textContent).toBe("Undo");
    await userEvent.keyboard("{Enter}");
    const toast = document.querySelector(".paper-toast") as HTMLElement;
    expect(toast.hasAttribute("data-exiting")).toBe(true);
    expect(document.activeElement?.textContent).toBe("Before");
    await expect.poll(() => document.querySelector(".paper-toast")).toBeNull();
  });
});
