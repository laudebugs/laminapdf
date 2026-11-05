import { useState } from "react";
import { afterEach, describe, expect, it, onTestFinished, vi } from "vitest";
import { commands, userEvent } from "vitest/browser";
import {
  NetworkIndicator,
  OutlineRow,
  ProgressBar,
  RepairedNotice,
  ShortcutChip,
  StatusBarItem,
  Thumbnail,
  Toast,
} from "../../src/index.ts";
import { cleanup, renderIn } from "./render.tsx";

afterEach(cleanup);

declare module "vitest/browser" {
  interface BrowserCommands {
    /** Makes the page prefer reduced motion, or stop preferring it with `null`. */
    reducedMotion: (value: "reduce" | null) => Promise<void>;
  }
}

const get = (host: HTMLElement, selector: string) => host.querySelector(selector) as HTMLElement;
const focused = () => document.activeElement as HTMLElement;

describe("status bar items", () => {
  it("are Tab stops that press on Enter and Space", async () => {
    const onPress = vi.fn();
    renderIn(
      <>
        <NetworkIndicator count="0 network requests" explanation="· Offline-safe" onPress={() => onPress("network")} />
        <RepairedNotice label="Repaired on open" details="Details" onPress={() => onPress("repaired")} />
        <StatusBarItem onPress={() => onPress("sync")}>Synced</StatusBarItem>
      </>,
    );
    await userEvent.tab();
    await userEvent.keyboard("{Enter}");
    await userEvent.tab();
    await userEvent.keyboard(" ");
    await userEvent.tab();
    expect(focused().textContent).toBe("Synced");
    await userEvent.keyboard("{Enter}");
    expect(onPress.mock.calls.flat()).toEqual(["network", "repaired", "sync"]);
  });

  it("name the network indicator by its count and explanation", () => {
    const host = renderIn(<NetworkIndicator count="2 network requests" explanation="· to files.example.org" warning />);
    const button = get(host, "button");
    expect(button.textContent).toBe("2 network requests· to files.example.org");
    expect(button.dataset.warning).toBe("true");
  });
});

describe("Thumbnail", () => {
  it("marks only the current page and goes to a page on Enter", async () => {
    const onPress = vi.fn();
    const host = renderIn(
      <>
        <Thumbnail label="Page 1, 1 of 2" number="1" isCurrent onPress={() => onPress(1)} />
        <Thumbnail label="Page 2, 2 of 2" number="2" onPress={() => onPress(2)} />
      </>,
    );
    const [first, second] = [...host.querySelectorAll("button")];
    expect(first?.getAttribute("aria-current")).toBe("page");
    expect(second?.hasAttribute("aria-current")).toBe(false);
    expect(second?.getAttribute("aria-label")).toBe("Page 2, 2 of 2");
    await userEvent.tab();
    await userEvent.tab();
    await userEvent.keyboard("{Enter}");
    expect(onPress).toHaveBeenCalledWith(2);
  });

  it("draws the viewer's image inside the page box", () => {
    const host = renderIn(
      <Thumbnail label="Page 1" number="1">
        <canvas width={58} height={75} />
      </Thumbnail>,
    );
    const box = get(host, ".paper-thumbnail-page").getBoundingClientRect();
    const canvas = get(host, "canvas").getBoundingClientRect();
    expect([canvas.width, canvas.height]).toEqual([box.width, box.height]);
  });
});

function Section() {
  const [open, setOpen] = useState(false);
  return (
    <OutlineRow
      title="2. Repairs backlog"
      page="3"
      href="#repairs"
      toggle={{ isExpanded: open, onExpandedChange: setOpen, label: "Sections in 2. Repairs backlog" }}
    />
  );
}

describe("OutlineRow", () => {
  it("toggles aria-expanded from the chevron, then reaches the link", async () => {
    const host = renderIn(<Section />);
    const chevron = get(host, "button");
    expect(chevron.getAttribute("aria-expanded")).toBe("false");
    await userEvent.tab();
    expect(focused()).toBe(chevron);
    await userEvent.keyboard("{Enter}");
    expect(chevron.getAttribute("aria-expanded")).toBe("true");
    await userEvent.keyboard(" ");
    expect(chevron.getAttribute("aria-expanded")).toBe("false");
    await userEvent.tab();
    expect(focused().getAttribute("href")).toBe("#repairs");
  });

  it("marks the current row as the location and follows a link on Enter", async () => {
    const onPress = vi.fn();
    const host = renderIn(<OutlineRow title="Budget" page="7" isCurrent onPress={onPress} />);
    expect(get(host, "[role=link]").getAttribute("aria-current")).toBe("location");
    await userEvent.tab();
    await userEvent.keyboard("{Enter}");
    expect(onPress).toHaveBeenCalledOnce();
  });

  it("mirrors the closed chevron right to left", () => {
    const transform = (dir: "ltr" | "rtl") =>
      getComputedStyle(get(renderIn(<Section />, { dir }), ".paper-outline-toggle svg")).transform;
    expect(transform("ltr")).toBe("none");
    expect(transform("rtl")).not.toBe("none");
  });
});

describe("ProgressBar", () => {
  it("reports its value and shows the percentage", () => {
    const host = renderIn(<ProgressBar label="Saving…" value={62} />);
    const bar = get(host, "[role=progressbar]");
    expect(bar.getAttribute("aria-valuenow")).toBe("62");
    expect(bar.getAttribute("aria-valuemin")).toBe("0");
    expect(bar.getAttribute("aria-valuemax")).toBe("100");
    expect(bar.getAttribute("aria-labelledby")).toBeTruthy();
    expect(host.textContent).toContain("62%");
    const track = get(host, ".paper-progress-track").getBoundingClientRect().width;
    expect(get(host, ".paper-progress-fill").getBoundingClientRect().width).toBeCloseTo(track * 0.62, 0);
  });

  it("leaves aria-valuenow out while indeterminate, and stands still under reduced motion", async () => {
    const host = renderIn(<ProgressBar label="Opening…" />);
    expect(get(host, "[role=progressbar]").hasAttribute("aria-valuenow")).toBe(false);
    const fill = get(host, ".paper-progress-fill");
    expect(getComputedStyle(fill).animationName).toBe("paper-progress-slide");
    await commands.reducedMotion("reduce");
    onTestFinished(() => commands.reducedMotion(null));
    expect(getComputedStyle(fill).animationName).toBe("none");
  });
});

describe("Toast", () => {
  it("is a status message whose action is a button", async () => {
    const onPress = vi.fn();
    const host = renderIn(<Toast action={{ label: "Undo", onPress }}>Highlight deleted</Toast>);
    expect(get(host, ".paper-toast").getAttribute("role")).toBe("status");
    await userEvent.tab();
    await userEvent.keyboard("{Enter}");
    expect(onPress).toHaveBeenCalledOnce();
  });
});

describe("ShortcutChip", () => {
  const text = (keys: string, platform: "mac" | "other") =>
    get(renderIn(<ShortcutChip keys={{ mac: keys, other: keys }} platform={platform} />), "kbd").textContent;

  it("runs macOS glyphs together in ⌃⌥⇧⌘ order", () => {
    expect(text("Mod+Alt+E", "mac")).toBe("⌥⌘E");
    expect(text("Shift+Mod+k", "mac")).toBe("⇧⌘K");
    expect(text("Ctrl+Escape", "mac")).toBe("⌃esc");
  });

  it("spells modifiers out with spaces elsewhere", () => {
    expect(text("Mod+Shift+E", "other")).toBe("Ctrl Shift E");
    expect(text("Mod+Ctrl+K", "other")).toBe("Ctrl K");
    expect(text("Mod++", "other")).toBe("Ctrl +");
    expect(text("F3", "other")).toBe("F3");
    expect(text("?", "mac")).toBe("?");
  });
});
