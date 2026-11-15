// Choices, fields and rows follow the shared states (design language §7.0, §7.19): the hand on
// what chooses or opens, the arrow when disabled, the I-beam in text, the switch's knob sliding
// (and jumping under reduced motion), and an invalid field keeping its focus ring's colour.

import { afterEach, describe, expect, it, onTestFinished } from "vitest";
import { userEvent } from "vitest/browser";
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

  it("slides its knob to the far end over the state-out time, mirrored right to left", () => {
    const on = renderIn(<Switch defaultSelected>Show thumbnails</Switch>);
    const track = get(on, ".paper-switch-track").getBoundingClientRect();
    const k = knob(on).getBoundingClientRect();
    expect(k.right).toBeCloseTo(track.right - 3, 0);
    expect(style(knob(on)).transitionDuration).toBe("0.15s");
    const rtl = renderIn(<Switch defaultSelected>Show thumbnails</Switch>, { dir: "rtl" });
    const rtlTrack = get(rtl, ".paper-switch-track").getBoundingClientRect();
    expect(knob(rtl).getBoundingClientRect().left).toBeCloseTo(rtlTrack.left + 3, 0);
  });

  it("jumps under reduced motion", () => {
    const host = renderIn(<Switch defaultSelected>Show thumbnails</Switch>);
    document.documentElement.setAttribute("data-motion", "reduce");
    onTestFinished(() => document.documentElement.removeAttribute("data-motion"));
    expect(style(knob(host)).transitionDuration).toBe("0s");
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

describe("hover", () => {
  it("lays the hover layer on a checkbox's box and a swatch, not on the label row", async () => {
    // Transitions off, so the layer reads its end value rather than a frame of its fade.
    document.documentElement.setAttribute("data-motion-suppressed", "");
    onTestFinished(() => document.documentElement.removeAttribute("data-motion-suppressed"));
    const host = renderIn(
      <>
        <Checkbox>Highlight fields</Checkbox>
        <SwatchGroup label="Colour" swatches={SWATCHES} value="yellow" onChange={noop} />
      </>,
    );
    const layer = (el: Element) => style(el).getPropertyValue("--ui-state-layer").trim();
    await userEvent.hover(get(host, ".paper-checkbox"));
    expect(layer(get(host, ".paper-checkbox-box"))).not.toBe("rgba(0, 0, 0, 0)");
    expect(layer(get(host, ".paper-checkbox"))).toBe("rgba(0, 0, 0, 0)");
    const swatch = host.querySelectorAll(".paper-swatch")[1] as HTMLElement;
    await userEvent.hover(swatch);
    expect(layer(swatch)).not.toBe("rgba(0, 0, 0, 0)");
  });
});
