// The action controls' shared states (design language §7.0): the cursor for each state, hover
// fading in and out, press at once, a selected control still answering hover, and selection that
// leaves the row's layout alone.

import { afterEach, describe, expect, it, onTestFinished } from "vitest";
import { userEvent } from "vitest/browser";
import {
  Button,
  IconButton,
  ModeSwitcher,
  SegmentedTabs,
  StatusBarItem,
  TabPanel,
  ToggleButton,
  ToolToggle,
} from "../../src/index.ts";
import { cleanup, get, renderIn } from "./render.tsx";

afterEach(cleanup);

const noop = () => {};
const style = (el: Element) => getComputedStyle(el);
/** Turns transitions off for this test, so computed styles read each state's end value. */
const instant = () => {
  document.documentElement.setAttribute("data-motion-suppressed", "");
  onTestFinished(() => document.documentElement.removeAttribute("data-motion-suppressed"));
};
/** Holds Space down on `el` (React Aria presses on key down) until the returned function lets go. */
const pressDown = async (el: HTMLElement) => {
  el.focus();
  await userEvent.keyboard("{Space>}");
  return async () => {
    await userEvent.keyboard("{/Space}");
  };
};
/** A computed colour as linear-ready sRGB channels and alpha, from rgb(), rgba() or color(srgb …). */
const channels = (css: string): [number, number, number, number] => {
  const num = String.raw`(\d*\.?\d+(?:e-?\d+)?)`;
  const srgb = css.match(new RegExp(String.raw`^color\(srgb ${num} ${num} ${num}(?: / ${num})?\)$`));
  if (srgb) return [Number(srgb[1]), Number(srgb[2]), Number(srgb[3]), Number(srgb[4] ?? 1)];
  const rgb = css.match(new RegExp(String.raw`^rgba?\(${num}, ${num}, ${num}(?:, ${num})?\)$`));
  if (rgb) return [Number(rgb[1]) / 255, Number(rgb[2]) / 255, Number(rgb[3]) / 255, Number(rgb[4] ?? 1)];
  throw new Error(`a colour this test can't read: ${css}`);
};
/** WCAG contrast of `el`'s text on its fill with the state layer composited over it. */
const contrast = (el: HTMLElement) => {
  const [fr, fg, fb] = channels(style(el).backgroundColor);
  const [lr, lg, lb, la] = channels(layer(el));
  const fill = [fr * (1 - la) + lr * la, fg * (1 - la) + lg * la, fb * (1 - la) + lb * la];
  const lum = ([r, g, b]: number[]) =>
    [r, g, b]
      .map((c = 0) => (c <= 0.04045 ? c / 12.92 : ((c + 0.055) / 1.055) ** 2.4))
      .reduce((sum, c, i) => sum + c * [0.2126, 0.7152, 0.0722][i]!, 0);
  const [a, b] = [lum(fill), lum(channels(style(el).color))].sort((x, y) => y - x);
  return ((a ?? 0) + 0.05) / ((b ?? 0) + 0.05);
};
/** The colour of the state layer the control draws over its fill. */
const layer = (el: Element) => style(el).getPropertyValue("--ui-state-layer").trim();

describe("cursors", () => {
  const controls = [
    ["Button", <Button key="b">Save</Button>],
    ["IconButton", <IconButton key="i" label="Undo" icon="undo" />],
    ["ToggleButton", <ToggleButton key="t">Sidebar</ToggleButton>],
    [
      "ToolToggle",
      <ToolToggle key="tt" icon="highlight">
        Highlight
      </ToolToggle>,
    ],
    ["StatusBarItem", <StatusBarItem key="s">Page 1</StatusBarItem>],
  ] as const;

  for (const [name, element] of controls) {
    it(`${name} shows the hand`, () => {
      expect(style(get(renderIn(element), "button")).cursor).toBe("pointer");
    });
  }

  it("shows the arrow on a disabled control and the busy arrow while pending", () => {
    const host = renderIn(
      <>
        <Button isDisabled>Off</Button>
        <IconButton label="Undo" icon="undo" isDisabled />
        <Button isPending>Saving</Button>
        <IconButton label="Save" icon="download" isPending />
      </>,
    );
    expect([...host.querySelectorAll("button")].map((b) => style(b).cursor)).toEqual([
      "default",
      "default",
      "progress",
      "progress",
    ]);
  });

  it("shows the hand on modes and tabs", () => {
    const host = renderIn(
      <>
        <ModeSwitcher aria-label="Mode" modes={[{ id: "read", label: "Read" }]} value="read" onChange={noop} />
        <SegmentedTabs
          aria-label="Sidebar"
          tabs={[{ id: "pages", label: "Pages" }]}
          selectedKey="pages"
          onSelectionChange={noop}
        >
          <TabPanel id="pages">Pages</TabPanel>
        </SegmentedTabs>
      </>,
    );
    expect(style(get(host, ".paper-mode")).cursor).toBe("pointer");
    expect(style(get(host, ".paper-tab")).cursor).toBe("pointer");
  });
});

describe("hover and press", () => {
  it("fades hover in quickly and out more slowly, and presses at once", async () => {
    const button = get(renderIn(<Button variant="ghost">Save</Button>), "button");
    expect(style(button).transitionDuration.split(", ")[0]).toBe("0.15s");
    await userEvent.hover(button);
    expect(button.hasAttribute("data-hovered")).toBe(true);
    expect(style(button).transitionDuration.split(", ")[0]).toBe("0.08s");
    expect(style(button).transitionProperty).not.toContain("all");
    await userEvent.unhover(button);
    expect(button.hasAttribute("data-hovered")).toBe(false);
  });

  it("presses at once, with no transition, and releases back to hover over the hover-in time", async () => {
    const button = get(renderIn(<Button variant="ghost">Save</Button>), "button");
    await userEvent.hover(button);
    const lift = await pressDown(button);
    expect(button.hasAttribute("data-pressed")).toBe(true);
    expect(style(button).transitionProperty).toBe("none");
    await lift();
    expect(button.hasAttribute("data-pressed")).toBe(false);
    expect(style(button).transitionDuration.split(", ")[0]).toBe("0.08s");
  });

  it("lays hover over a selected toggle's fill instead of replacing it", async () => {
    instant();
    const toggle = get(renderIn(<ToggleButton isSelected>Sidebar</ToggleButton>), "button");
    const fill = style(toggle).backgroundColor;
    await userEvent.hover(toggle);
    expect(style(toggle).backgroundColor).toBe(fill);
    expect(layer(toggle)).not.toBe("rgba(0, 0, 0, 0)");
  });

  it("swaps the filled buttons' solid fills with no layer", async () => {
    instant();
    const host = renderIn(
      <>
        <Button variant="primary">Save</Button>
        <Button variant="danger-filled">Delete</Button>
      </>,
    );
    for (const button of host.querySelectorAll("button")) {
      const rest = style(button).backgroundColor;
      await userEvent.hover(button);
      expect(style(button).backgroundColor).not.toBe(rest);
      expect(layer(button)).toBe("rgba(0, 0, 0, 0)");
      await userEvent.unhover(button);
    }
  });
});

describe("contrast of the state fills", () => {
  // One test per theme and accent, so each stays well inside its time even when a whole suite
  // runs at once in Firefox (16 hovers and presses in one test ran out there).
  for (const theme of ["light", "dark"] as const) {
    for (const accent of ["teal", "blue", "plum", "green"] as const) {
      it(`keeps every label at 4.5:1 or more when hovered and pressed: ${theme} theme, ${accent} accent`, async () => {
        instant();
        const host = renderIn(
          <>
            <Button variant="primary">Save</Button>
            <Button variant="danger-filled">Delete</Button>
            <ToggleButton isSelected>Sidebar</ToggleButton>
            <Button variant="secondary">Open</Button>
          </>,
          { theme },
        );
        host.dataset.accent = accent;
        for (const button of host.querySelectorAll("button")) {
          await userEvent.hover(button);
          expect(contrast(button), `${button.innerText} hovered`).toBeGreaterThanOrEqual(4.5);
          const lift = await pressDown(button);
          expect(contrast(button), `${button.innerText} pressed`).toBeGreaterThanOrEqual(4.5);
          await lift();
        }
      });
    }
  }
});

describe("selection", () => {
  const widths = (host: HTMLElement, selector: string) =>
    // To a hundredth of a pixel: Firefox lays out bold and regular text a few millionths apart.
    [...host.querySelectorAll(selector)].map((el) => Math.round(el.getBoundingClientRect().width * 100) / 100);

  it("moves no tab when another is selected", () => {
    const tabs = [
      { id: "pages", label: "Pages" },
      { id: "outline", label: "Outline" },
      { id: "attachments", label: "Attachments" },
    ];
    const at = (selectedKey: string) =>
      widths(
        renderIn(
          <SegmentedTabs aria-label="Sidebar" tabs={tabs} selectedKey={selectedKey} onSelectionChange={noop}>
            {tabs.map((tab) => (
              <TabPanel key={tab.id} id={tab.id}>
                {tab.label}
              </TabPanel>
            ))}
          </SegmentedTabs>,
        ),
        ".paper-tab",
      );
    expect(at("outline")).toEqual(at("pages"));
  });

  it("centres an unselected label in the bold width it holds", () => {
    const host = renderIn(
      <SegmentedTabs
        aria-label="Sidebar"
        tabs={[
          { id: "pages", label: "Pages" },
          { id: "attachments", label: "Attachments" },
        ]}
        selectedKey="pages"
        onSelectionChange={noop}
      >
        <TabPanel id="pages">Pages</TabPanel>
        <TabPanel id="attachments">Attachments</TabPanel>
      </SegmentedTabs>,
    );
    const centre = (r: DOMRect) => r.left + r.width / 2;
    const box = get(host, "[role=tab][aria-selected=false] .paper-stable-label").getBoundingClientRect();
    const text = get(host, "[role=tab][aria-selected=false] .paper-stable-label > span").getBoundingClientRect();
    expect(text.width).toBeLessThan(box.width);
    expect(centre(text)).toBeCloseTo(centre(box), 0);
  });

  it("moves no mode when another is selected", () => {
    const modes = [
      { id: "read", label: "Read" },
      { id: "comment", label: "Comment" },
    ];
    const at = (value: string) =>
      widths(renderIn(<ModeSwitcher aria-label="Mode" modes={modes} value={value} onChange={noop} />), ".paper-mode");
    expect(at("comment")).toEqual(at("read"));
  });

  it("keeps a toggle's width when it is selected, and names it once", () => {
    const host = renderIn(
      <>
        <ToolToggle icon="highlight">Highlight</ToolToggle>
        <ToolToggle icon="highlight" isSelected>
          Highlight
        </ToolToggle>
      </>,
    );
    const [off, on] = widths(host, "button");
    expect(on).toBe(off);
    // The bold copy is hidden from assistive technology, so the name is the label alone.
    expect(get(host, ".paper-stable-label-bold").getAttribute("aria-hidden")).toBe("true");
  });
});
