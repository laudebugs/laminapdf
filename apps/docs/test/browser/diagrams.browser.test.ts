// Every diagram in a real browser, in the light, dark and high-contrast themes, and with forced
// colours emulated (Chromium and Firefox; WebKit can't emulate them, and the high-contrast theme
// maps the tokens to the same system colours): each label has 4.5:1 against what is behind it,
// each line that carries meaning 3:1 against the page, no label is under 12 px at a desktop
// column, the narrowest column that shows the wide layout or a phone's, and the picture is an
// image named by its caption and described by its text. axe-core doesn't measure contrast inside
// SVG, so these tests compute it themselves.

import "@laminapdf/tokens/tokens.css";
import "@laminapdf/fonts/fonts.css";
import "../../src/styles/diagrams.css";
import { afterEach, describe, expect, inject, it, onTestFinished } from "vitest";
import { commands, server } from "vitest/browser";

declare module "vitest/browser" {
  interface BrowserCommands {
    forcedColors: (value: "active" | "none") => Promise<void>;
  }
}

const DIAGRAMS = inject("diagrams");
const THEMES = ["light", "dark", "high-contrast"] as const;
/**
 * The docs' content column at a 1280 px window; the narrowest column that still shows the wide
 * layout (just over 40rem), where its labels are smallest; and a phone's (390 px less the gutters).
 */
const WIDTHS = { desktop: 708, "narrowest wide": 641, phone: 358 } as const;

const ground = document.createElement("style");
ground.textContent = "body { margin: 0; background: var(--paper-ground); color: var(--paper-text); }";
document.head.append(ground);

afterEach(() => {
  document.body.innerHTML = "";
});

/** The diagram's figure, mounted at `width` in `theme`. */
function mount(html: string, theme: (typeof THEMES)[number], width: number): HTMLElement {
  document.documentElement.dataset.theme = theme;
  document.body.innerHTML = `<main style="width:${width}px">${html}</main>`;
  const figure = document.querySelector<HTMLElement>("figure.dg");
  if (figure === null) throw new Error("no figure");
  return figure;
}

/** The drawing the figure shows at its width (the other layout is display: none). */
function shown(figure: HTMLElement): SVGSVGElement {
  const svg = [...figure.querySelectorAll<SVGSVGElement>("svg")].find((s) => getComputedStyle(s).display !== "none");
  if (svg === undefined) throw new Error("no drawing shown");
  return svg;
}

const canvas = document.createElement("canvas").getContext("2d", { willReadFrequently: true });

/** Any CSS colour as sRGB 0–255, through a canvas, so system colours and modern syntax resolve. */
function rgb(color: string): [number, number, number] {
  if (canvas === null) throw new Error("no canvas");
  canvas.clearRect(0, 0, 1, 1);
  canvas.fillStyle = color;
  canvas.fillRect(0, 0, 1, 1);
  const [r = 0, g = 0, b = 0] = canvas.getImageData(0, 0, 1, 1).data;
  return [r, g, b];
}

function luminance([r, g, b]: [number, number, number]): number {
  const lin = (c: number) => {
    const s = c / 255;
    return s <= 0.03928 ? s / 12.92 : ((s + 0.055) / 1.055) ** 2.4;
  };
  return 0.2126 * lin(r) + 0.7152 * lin(g) + 0.0722 * lin(b);
}

function contrast(a: string, b: string): number {
  const [l1, l2] = [luminance(rgb(a)), luminance(rgb(b))].sort((x, y) => y - x) as [number, number];
  return (l1 + 0.05) / (l2 + 0.05);
}

/** The fill behind `text`: the filled shape under its centre, else the page. */
function behind(text: SVGTextElement): string {
  const box = text.getBoundingClientRect();
  const under = document.elementsFromPoint(box.x + box.width / 2, box.y + box.height / 2);
  // Nothing at all under a label means it is outside the window, not on the page.
  if (!under.includes(text)) throw new Error(`"${text.textContent}" is outside the window`);
  for (const element of under) {
    if (element === text || !(element instanceof SVGGeometryElement)) continue;
    const fill = getComputedStyle(element).fill;
    if (fill !== "none" && !fill.startsWith("url(")) return fill;
  }
  return getComputedStyle(document.body).backgroundColor;
}

/** Every line drawn on a PageSheet has 3:1 against the sheet, not the page around it. */
function expectSheetLines(svg: SVGSVGElement, where: string): void {
  for (const shape of svg.querySelectorAll(".dg-page .dg-page-axis, .dg-page .dg-ink-line, .dg-page .dg-order")) {
    const sheet = shape.closest(".dg-page")?.querySelector(".dg-sheet");
    if (sheet === null || sheet === undefined) throw new Error("a page line with no sheet");
    const ratio = contrast(getComputedStyle(shape).stroke, getComputedStyle(sheet).fill);
    expect(ratio, `${shape.getAttribute("class")} in ${where}`).toBeGreaterThanOrEqual(3);
  }
}

for (const [name, html] of Object.entries(DIAGRAMS)) {
  for (const theme of THEMES) {
    describe(`${name}, ${theme}`, () => {
      it("gives every label 4.5:1 against what is behind it", async () => {
        await document.fonts.ready;
        const svg = shown(mount(html, theme, WIDTHS.desktop));
        for (const text of svg.querySelectorAll("text")) {
          const ratio = contrast(getComputedStyle(text).fill, behind(text));
          expect(ratio, `"${text.textContent}" in ${theme}`).toBeGreaterThanOrEqual(4.5);
        }
      });

      it("gives every line drawn on a page sheet 3:1 against the sheet", () => {
        expectSheetLines(shown(mount(html, theme, WIDTHS.desktop)), theme);
      });

      it("gives every border and arrow 3:1 against the page", () => {
        const svg = shown(mount(html, theme, WIDTHS.desktop));
        const page = getComputedStyle(document.body).backgroundColor;
        for (const shape of svg.querySelectorAll(".dg-box, .dg-focus, .dg-fresh, .dg-arrow, .dg-lane, .dg-sheet")) {
          const ratio = contrast(getComputedStyle(shape).stroke, page);
          expect(ratio, `${shape.getAttribute("class")} in ${theme}`).toBeGreaterThanOrEqual(3);
        }
      });
    });
  }

  it(`${name}: with forced colours, gives every label 4.5:1 and every line 3:1`, async (context) => {
    if (server.browser === "webkit") context.skip("WebKit can't emulate forced colours");
    await commands.forcedColors("active");
    onTestFinished(() => commands.forcedColors("none"));
    // Otherwise a browser that ignored the emulation would pass as the light theme.
    expect(matchMedia("(forced-colors: active)").matches).toBe(true);
    const svg = shown(mount(html, "light", WIDTHS.desktop));
    const page = getComputedStyle(document.body).backgroundColor;
    for (const text of svg.querySelectorAll("text")) {
      expect(contrast(getComputedStyle(text).fill, behind(text)), `"${text.textContent}"`).toBeGreaterThanOrEqual(4.5);
    }
    for (const shape of svg.querySelectorAll(".dg-box, .dg-focus, .dg-fresh, .dg-arrow, .dg-lane, .dg-sheet")) {
      expect(contrast(getComputedStyle(shape).stroke, page), `${shape.getAttribute("class")}`).toBeGreaterThanOrEqual(
        3,
      );
    }
    expectSheetLines(svg, "forced colours");
  });

  for (const [where, width] of Object.entries(WIDTHS)) {
    it(`${name}: no label under 12 px in a ${where} column`, async () => {
      await document.fonts.ready;
      const svg = shown(mount(html, "light", width));
      const scale = svg.getBoundingClientRect().width / svg.viewBox.baseVal.width;
      for (const text of svg.querySelectorAll("text")) {
        const px = Number.parseFloat(getComputedStyle(text).fontSize) * scale;
        expect(px, `"${text.textContent}" at ${width} px`).toBeGreaterThanOrEqual(12);
      }
    });
  }

  it(`${name}: is an image named by its caption and described by its text`, () => {
    const figure = mount(html, "light", WIDTHS.desktop);
    const svg = shown(figure);
    expect(svg.getAttribute("role")).toBe("img");
    const caption = document.getElementById(svg.getAttribute("aria-labelledby") ?? "");
    const text = document.getElementById(svg.getAttribute("aria-describedby") ?? "");
    expect(caption?.tagName).toBe("FIGCAPTION");
    expect(caption?.textContent?.trim().length).toBeGreaterThan(0);
    expect(text?.textContent?.trim().length).toBeGreaterThan(0);
  });
}
