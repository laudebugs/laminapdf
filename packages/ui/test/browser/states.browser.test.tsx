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
