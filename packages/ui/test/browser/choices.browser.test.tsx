// Choices, fields and rows follow the shared states (design language §7.0, §7.19): the hand on
// what chooses or opens, the arrow when disabled, the I-beam in text, the switch's knob sliding
// (and jumping under reduced motion), and an invalid field keeping its focus ring's colour.

import { afterEach, describe, expect, it, onTestFinished } from "vitest";
import { commands, server, userEvent } from "vitest/browser";
import {
  Checkbox,
  OutlineRow,
  PageAppearanceChips,
  Radio,
  RadioGroup,
  Select,
  SwatchGroup,
  Switch,
  TextInput,
} from "../../src/index.ts";
import { cleanup, get, renderIn } from "./render.tsx";

afterEach(cleanup);

declare module "vitest/browser" {
  interface BrowserCommands {
    /** Makes the page prefer reduced motion or not, or follow the system again with `null`. */
    reducedMotion: (value: "reduce" | "no-preference" | null) => Promise<void>;
    /** Emulates forced colours (Windows high contrast), or stops with `null`. Chromium only. */
    forcedColors: (value: "active" | null) => Promise<void>;
  }
}

/** Turns transitions off for this test, so computed styles read each state's end value. */
const instant = () => {
  document.documentElement.setAttribute("data-motion-suppressed", "");
  onTestFinished(() => document.documentElement.removeAttribute("data-motion-suppressed"));
};

const noop = () => {};
const style = (el: Element) => getComputedStyle(el);
const SWATCHES = [
  { id: "yellow", label: "Yellow", color: "annotation-yellow" },
  { id: "blue", label: "Blue", color: "annotation-blue" },
] as const;
const LABELS = { original: "Original", sepia: "Sepia", dark: "Dark", "high-contrast": "High contrast" } as const;
/** The colour a token resolves to here, read through an element that paints with it. */
const token = (host: HTMLElement, name: string) => {
  const probe = document.createElement("span");
  probe.style.color = `var(--paper-${name})`;
  host.append(probe);
  return style(probe).color;
};

describe("cursors", () => {
  it("shows the hand on every choice, swatch, chip, row and the select", () => {
    const host = renderIn(
      <>
        <Checkbox>Highlight fields</Checkbox>
        <RadioGroup label="Pages" defaultValue="all">
          <Radio value="all">All pages</Radio>
        </RadioGroup>
        <Switch>Show thumbnails</Switch>
        <SwatchGroup label="Colour" swatches={SWATCHES} value="yellow" onChange={noop} />
        <PageAppearanceChips aria-label="Page appearance" value="original" onChange={noop} labels={LABELS} />
        <OutlineRow
          title="Results"
          page="3"
          onPress={noop}
          toggle={{ isExpanded: false, onExpandedChange: noop, label: "Sections in Results" }}
        />
        <Select label="Paper size" options={[{ value: "a4", label: "A4" }]} value="a4" onChange={noop} />
      </>,
    );
    const selectors = [
      ".paper-checkbox",
      ".paper-radio",
      ".paper-switch",
      ".paper-swatch",
      ".paper-appearance-chip",
      ".paper-outline-link",
      ".paper-outline-toggle",
      "select",
    ];
    expect(selectors.map((s) => [s, style(get(host, s)).cursor])).toEqual(selectors.map((s) => [s, "pointer"]));
  });

  it("shows the arrow on disabled choices and fields, and the I-beam in a text field", () => {
    const host = renderIn(
      <>
        <Checkbox isDisabled>Locked</Checkbox>
        <Switch isDisabled>Show thumbnails</Switch>
        <TextInput label="Producer" defaultValue="Lamina PDF" isDisabled />
        <TextInput label="Title" />
      </>,
    );
    expect(style(get(host, ".paper-checkbox")).cursor).toBe("default");
    expect(style(get(host, ".paper-switch")).cursor).toBe("default");
    const [disabled, text] = host.querySelectorAll("input.paper-input");
    expect(style(disabled as Element).cursor).toBe("default");
    // `auto` over editable text is the I-beam.
    expect(["auto", "text"]).toContain(style(text as Element).cursor);
  });
});

describe("the switch", () => {
  const knob = (host: HTMLElement) => get(host, ".paper-switch-knob");
  /** The knob's inset from the track's edge: the track's padding and border, as rendered. */
  const inset = (track: Element, side: "left" | "right") =>
    Number.parseFloat(style(track).getPropertyValue(`padding-${side}`)) +
    Number.parseFloat(style(track).getPropertyValue(`border-${side}-width`));

  it("slides its knob to the far end when turned on, by translate over the state-out time", async () => {
    // The runner's own setting may ask for reduced motion (CI's WebKit does), which drops the slide.
    await commands.reducedMotion("no-preference");
    onTestFinished(() => commands.reducedMotion(null));
    const host = renderIn(<Switch>Show thumbnails</Switch>);
    const track = get(host, ".paper-switch-track");
    expect(knob(host).getBoundingClientRect().left).toBeCloseTo(
      track.getBoundingClientRect().left + inset(track, "left"),
      0,
    );
    expect(style(knob(host)).transitionProperty).toBe("translate");
    expect(style(knob(host)).transitionDuration).toBe("0.15s");
    await userEvent.click(get(host, ".paper-switch"));
    // Long enough for the 150 ms slide to finish, however slow the runner.
    await new Promise((r) => setTimeout(r, 300));
    expect(knob(host).getBoundingClientRect().right).toBeCloseTo(
      track.getBoundingClientRect().right - inset(track, "right"),
      0,
    );
  });

  it("mirrors the knob right to left", () => {
    const host = renderIn(<Switch defaultSelected>Show thumbnails</Switch>, { dir: "rtl" });
    const track = get(host, ".paper-switch-track");
    expect(knob(host).getBoundingClientRect().left).toBeCloseTo(
      track.getBoundingClientRect().left + inset(track, "left"),
      0,
    );
  });

  it("jumps when the system asks for reduced motion", async () => {
    const host = renderIn(<Switch defaultSelected>Show thumbnails</Switch>);
    await commands.reducedMotion("reduce");
    onTestFinished(() => commands.reducedMotion(null));
    expect(style(knob(host)).transitionProperty).toBe("none");
  });

  it("jumps when Settings asks for reduced motion, with no transition at all", () => {
    const host = renderIn(<Switch defaultSelected>Show thumbnails</Switch>);
    document.documentElement.setAttribute("data-motion", "reduce");
    onTestFinished(() => document.documentElement.removeAttribute("data-motion"));
    expect(style(knob(host)).transitionProperty).toBe("none");
  });
});

describe("fields", () => {
  it("keeps the focus ring's colour on an invalid field", async () => {
    const host = renderIn(<TextInput label="Page" defaultValue="40" errorMessage="Enter a page from 1 to 24" />);
    await userEvent.tab();
    const input = get(host, "input");
    expect(document.activeElement).toBe(input);
    expect(style(input).outlineColor).toBe(token(host, "focus"));
    expect(style(input).borderColor).toBe(token(host, "danger"));
  });

  it("draws a read-only field on the ground with a hairline edge, its text still the text colour", () => {
    const host = renderIn(<TextInput label="Created" defaultValue="14 November 2026" isReadOnly />);
    const input = get(host, "input");
    expect(style(input).backgroundColor).toBe(token(host, "ground"));
    expect(style(input).borderColor).toBe(token(host, "hairline"));
    expect(style(input).color).toBe(token(host, "text"));
  });
});

describe("hover and press", () => {
  const layer = (el: Element) => style(el).getPropertyValue("--ui-state-layer").trim();

  it("lays the hover layer on a checkbox's box and a swatch, not on the label row", async () => {
    instant();
    const host = renderIn(
      <>
        <Checkbox>Highlight fields</Checkbox>
        <SwatchGroup label="Colour" swatches={SWATCHES} value="yellow" onChange={noop} />
      </>,
    );
    const hover = token(host, "hover");
    await userEvent.hover(get(host, ".paper-checkbox"));
    expect(layer(get(host, ".paper-checkbox-box"))).toBe(hover);
    expect(style(get(host, ".paper-checkbox")).backgroundImage).toBe("none");
    const swatch = host.querySelectorAll(".paper-swatch")[1] as HTMLElement;
    await userEvent.hover(swatch);
    expect(layer(swatch)).toBe(hover);
  });

  it("moves a checked box's fill to accent-strong on hover, with no layer", async () => {
    instant();
    const host = renderIn(<Checkbox defaultSelected>Highlight fields</Checkbox>);
    await userEvent.hover(get(host, ".paper-checkbox"));
    const box = get(host, ".paper-checkbox-box");
    expect(style(box).backgroundColor).toBe(token(host, "accent-strong"));
    expect(layer(box)).toBe("rgba(0, 0, 0, 0)");
  });

  it.runIf(server.browser === "chromium")(
    "keeps a checked box and an on switch Highlight in forced colours when hovered",
    async () => {
      instant();
      await commands.forcedColors("active");
      onTestFinished(() => commands.forcedColors(null));
      const host = renderIn(
        <>
          <Checkbox defaultSelected>Highlight fields</Checkbox>
          <Switch defaultSelected>Show thumbnails</Switch>
        </>,
      );
      const probe = document.createElement("span");
      probe.style.color = "Highlight";
      host.append(probe);
      for (const [row, drawing] of [
        [".paper-checkbox", ".paper-checkbox-box"],
        [".paper-switch", ".paper-switch-track"],
      ] as const) {
        await userEvent.hover(get(host, row));
        expect(style(get(host, drawing)).backgroundColor, row).toBe(style(probe).color);
      }
    },
  );
});
