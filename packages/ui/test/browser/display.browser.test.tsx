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
import { cleanup, focused, get, renderIn } from "./render.tsx";

afterEach(cleanup);

declare module "vitest/browser" {
  interface BrowserCommands {
    /** Makes the page prefer reduced motion, or stop preferring it with `null`. */
    reducedMotion: (value: "reduce" | null) => Promise<void>;
  }
}

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
    const host = renderIn(<NetworkIndicator count="0 network requests" explanation="· Offline-safe" />);
    expect(get(host, "button").textContent).toBe("0 network requests · Offline-safe");
  });

  it("swap the shield for the warning sign, not only its colour, when a request was unexpected", () => {
    const shape = (warning: boolean) =>
      get(
        renderIn(
          <NetworkIndicator count="2 network requests" explanation="· to files.example.org" warning={warning} />,
        ),
        "svg",
      ).innerHTML;
    expect(shape(true)).not.toBe(shape(false));
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
  it("leaves announcing to the toast region, and its action is a button", async () => {
    const onPress = vi.fn();
    const host = renderIn(<Toast action={{ label: "Undo", onPress }}>Highlight deleted</Toast>);
    expect(get(host, ".paper-toast").hasAttribute("role")).toBe(false);
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

  it("writes the Windows key, named keys and unknown modifiers as they are", () => {
    expect(text("Meta+L", "other")).toBe("Win L");
    expect(text("Mod+Space", "mac")).toBe("⌘Space");
    expect(text("Shift+f3", "mac")).toBe("⇧F3");
    expect(text("Hyper+K", "other")).toBe("Hyper K");
  });

  // docs/bugs/shortcut-chip-cramped-keys: ⌘ touched the key after it and sat below the letters.
  it("keeps every macOS key 4 px from the next and lifts ⌘ and the arrows to the letters", () => {
    const problems = chipBindings().flatMap((binding) => {
      const found = macChipProblems(binding);
      cleanup();
      return found;
    });
    expect(problems).toEqual([]);
  });
});

/** Every modifier combination with a letter, an arrow and a named key: 45 bindings. */
function chipBindings(): string[] {
  const modifiers = ["Ctrl", "Alt", "Shift", "Mod"];
  const bindings: string[] = [];
  for (let mask = 1; mask < 1 << modifiers.length; mask++) {
    const held = modifiers.filter((_, i) => mask & (1 << i));
    for (const key of ["V", "ArrowUp", "Escape"]) bindings.push([...held, key].join("+"));
  }
  return bindings;
}

/** What is wrong with `binding`'s macOS chip: keys closer than 4 px, ⌘ or an arrow not lifted. */
function macChipProblems(binding: string): string[] {
  const host = renderIn(<ShortcutChip keys={{ mac: binding, other: binding }} platform="mac" />);
  const spans = [...host.querySelectorAll<HTMLElement>(".paper-shortcut-chip-key")];
  const boxes = spans.map((s) => s.getBoundingClientRect());
  const problems = boxes.slice(1).flatMap((box, i) => {
    const gap = box.left - (boxes[i]?.right ?? 0);
    return gap < 3.99 ? [`${binding}: ${gap.toFixed(2)} px after ${spans[i]?.textContent}`] : [];
  });
  const low = spans.filter((s) => s.hasAttribute("data-low"));
  const letters = spans.filter((s) => !s.hasAttribute("data-low") && /^[A-Z]$/.test(s.textContent ?? ""));
  for (const glyph of low) {
    for (const letter of letters) {
      const lift = letter.getBoundingClientRect().top - glyph.getBoundingClientRect().top;
      if (lift < 0.5) problems.push(`${binding}: ${glyph.textContent} lifted ${lift.toFixed(2)} px`);
    }
  }
  const expected = ["⌘", "↑"].filter((g) => spans.some((s) => s.textContent === g));
  const lifted = low.map((s) => s.textContent);
  if (expected.join() !== lifted.join() || (binding.includes("Mod") && !lifted.includes("⌘"))) {
    problems.push(`${binding}: lifted ${lifted.join("") || "nothing"}, expected ${expected.join("") || "⌘"}`);
  }
  return problems;
}
