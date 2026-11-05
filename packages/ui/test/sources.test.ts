import { readdirSync, readFileSync } from "node:fs";
import { describe, expect, it } from "vitest";
import * as ui from "../src/index.ts";
import { EXAMPLES } from "./states.tsx";

const dir = (path: string) => new URL(`../${path}`, import.meta.url);
const files = (path: string, ext: RegExp) =>
  readdirSync(dir(path), { recursive: true, encoding: "utf8" })
    .filter((f) => ext.test(f))
    .map((f) => [`${path}${f}`, readFileSync(new URL(f, dir(path)), "utf8")] as const);

/** The CSS outside forced-colours blocks, where system colour keywords belong. */
const outsideForcedColours = (css: string) =>
  css.replace(/@media \(forced-colors: active\) \{(?:[^{}]*\{[^{}]*\})*[^{}]*\}/g, "");

describe("component CSS", () => {
  // Comments describe the look in words ("a white field"); only declarations count.
  const css = files("src/styles/", /\.css$/).map(
    ([name, text]) => [name, text.replace(/\/\*[\s\S]*?\*\//g, "")] as const,
  );

  it("takes every colour from a token", () => {
    const literal =
      /#[0-9a-f]{3,8}\b|\b(?:rgba?|hsla?|oklch|lab|lch|color)\(|(?<![\w-])(?:white|black|red|grey|gray)(?![\w-])/gi;
    const found = css.flatMap(([name, text]) =>
      [...outsideForcedColours(text).matchAll(literal)].map((m) => `${name}: ${m[0]}`),
    );
    expect(found).toEqual([]);
  });

  it("takes every length from a token: no px values", () => {
    const found = css.flatMap(([name, text]) =>
      [...text.matchAll(/\b\d+(?:\.\d+)?px\b/g)].map((m) => `${name}: ${m[0]}`),
    );
    expect(found).toEqual([]);
  });

  it("names only tokens that exist", () => {
    const tokens = readFileSync(new URL("../../tokens/src/generated/tokens.css", import.meta.url), "utf8");
    const defined = new Set([...tokens.matchAll(/--(paper-[\w-]+):/g)].map((m) => m[1]));
    const used = css.flatMap(([name, text]) =>
      [...text.matchAll(/var\(--(paper-[\w-]+)/g)].map((m) => [name, m[1] ?? ""]),
    );
    expect(used.filter(([, token]) => !defined.has(token)).map(([name, token]) => `${name}: ${token}`)).toEqual([]);
  });
});

describe("component sources", () => {
  it("import only React, React Aria, the tokens and the icons: no viewer, store or command registry", () => {
    const allowed =
      /^(?:react|react\/jsx-runtime|react-dom|react-aria-components|@laminapdf\/tokens|@laminapdf\/icons(?:\/react)?)$/;
    const found = files("src/", /\.tsx?$/).flatMap(([name, text]) =>
      [...text.matchAll(/\bfrom\s+"([^"]+)"|\bimport\s*\(?\s*"([^"]+)"/g)]
        .map((m) => m[1] ?? m[2] ?? "")
        .filter((spec) => !spec.startsWith(".") && !allowed.test(spec))
        .map((spec) => `${name}: ${spec}`),
    );
    expect(found).toEqual([]);
  });
});

/** Each design-language §7 section this package builds, and the components that build it. */
const SECTIONS: Record<string, string[]> = {
  "7.1 Top bar": ["ReaderTopBar", "PageInput", "ZoomControl"],
  "7.2 Mode switcher": ["ModeSwitcher"],
  "7.3 Icon button": ["IconButton"],
  "7.4 Text buttons": ["Button"],
  "7.5 Segmented tabs": ["SegmentedTabs", "TabPanel"],
  "7.6 Thumbnail": ["Thumbnail"],
  "7.7 Outline row": ["OutlineRow"],
  "7.8 Status bar items": ["StatusBarItem", "NetworkIndicator", "RepairedNotice", "PageLabel"],
  "7.10 Command palette: the shortcut chip": ["ShortcutChip"],
  "7.12 Colour swatches": ["SwatchGroup"],
  "7.19 Form controls": ["TextInput", "TextArea", "Select", "Checkbox", "RadioGroup", "Radio", "Switch"],
  "7.20 Toast and notice": ["Notice", "Toast"],
  "7.21 Empty state": ["EmptyState"],
  "7.22 Progress": ["ProgressBar", "Skeleton"],
  "7.9 Selection mini toolbar": ["SelectionToolbar", "SelectionToolbarDivider"],
  "7.10 Command palette": ["CommandPalette"],
  "7.16 Bottom sheet": ["BottomSheet"],
  "7.18 Dialog": ["Dialog"],
  "7.20 Toast and notice: the region": ["ToastRegion"],
  "7.23 Badge and tag": ["Badge"],
  "7.24 Secondary-toolbar tool toggle": ["ToolToggle", "ToggleButton"],
  "7.25 Page-appearance chips": ["PageAppearanceChips"],
  "7.26 Menu": ["Menu", "MenuTrigger", "Popover", "PopoverTrigger"],
  "7.27 Tooltip": ["Tooltip"],
};

describe("the design language's §7", () => {
  it("has an exported component for every section in scope", () => {
    const missing = Object.entries(SECTIONS).flatMap(([section, names]) =>
      names.filter((n) => typeof (ui as Record<string, unknown>)[n] !== "function").map((n) => `${section}: ${n}`),
    );
    expect(missing).toEqual([]);
  });

  it("shows every component of a section in the states the tests render", () => {
    const shown = new Set(EXAMPLES.map((e) => e.component));
    // Rendered inside their parents' examples, or tested open in overlays.browser.test.tsx, since
    // overlays portal outside the host the static examples render into.
    const parts = new Set([
      "TabPanel",
      "Radio",
      "Dialog",
      "ToastRegion",
      "Menu",
      "MenuTrigger",
      "Popover",
      "PopoverTrigger",
      "Tooltip",
      "BottomSheet",
      "SelectionToolbar",
      "SelectionToolbarDivider",
      "CommandPalette",
    ]);
    const missing = Object.values(SECTIONS)
      .flat()
      .filter((n) => !shown.has(n) && !parts.has(n));
    expect(missing).toEqual([]);
  });
});
