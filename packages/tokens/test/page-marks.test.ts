// The viewer's marks over a page (docs/bugs/selection-on-dark-pages): on the Original, Sepia
// and Dark pages, with every accent, the selection and the find marks land on their designed
// colours, the page's ink stays readable under them, and the selection shows over any picture.

import { describe, expect, it } from "vitest";
import { ACCENTS, resolve, rgba, type Token } from "../scripts/model.ts";
import { blended, blendFill, distance, MARKED as PAGES, selectionOverlay } from "../scripts/page-marks.ts";
import { contrastRatio, hex, composite as over } from "../src/contrast.ts";
import { loadTokens } from "./helpers.ts";

const tokens = loadTokens();
const at = (path: string) => rgba(resolve(tokens, path) as Token);
const WHITE = hex("#FFFFFF");
const BLACK = hex("#000000");

describe("the selection overlay", () => {
  for (const p of PAGES) {
    for (const a of ACCENTS) {
      const paper = at(`appearance.${p}.paper`);
      const ink = at(`appearance.${p}.ink`);
      const tint = at(`accent.${a}.page.${p}.selection-tint`);
      const overlay = selectionOverlay(paper, at(`accent.${a}.page.${p}.accent`), tint);
      const landed = over(overlay, paper);

      it(`lands on the ${a} tint over the ${p} paper`, () => {
        expect(distance(landed, tint)).toBeLessThanOrEqual(1);
      });

      it(`leaves the ${p} page's ink readable under the ${a} selection`, () => {
        expect(contrastRatio(over(overlay, ink), landed)).toBeGreaterThanOrEqual(4.5);
      });

      // A picture keeps its colours in every appearance; multiply gave 1.0 over black.
      it(`shows over a white picture on a ${p} page (${a})`, () => {
        expect(contrastRatio(over(overlay, WHITE), WHITE)).toBeGreaterThanOrEqual(1.1);
      });

      it(`shows over a black picture on a ${p} page (${a})`, () => {
        expect(contrastRatio(over(overlay, BLACK), BLACK)).toBeGreaterThanOrEqual(1.1);
      });
    }
  }
});

describe("the find marks", () => {
  for (const p of PAGES) {
    const paper = at(`appearance.${p}.paper`);
    const ink = at(`appearance.${p}.ink`);
    for (const mark of ["find-match", "find-current"] as const) {
      const colour = at(`appearance.${p}.${mark}`);
      const { blend, fill } = blendFill(paper, colour);
      const landed = blended(blend, paper, fill);

      it(`lands ${mark} on its token on the ${p} paper`, () => {
        // Dark's find-current has a blue below the paper's, which screen can't reach: 7 steps off.
        expect(distance(landed, colour)).toBeLessThanOrEqual(8);
      });

      it(`leaves the ${p} page's ink readable under ${mark}`, () => {
        expect(contrastRatio(blended(blend, ink, fill), landed)).toBeGreaterThanOrEqual(4.5);
      });
    }

    it(`blends to darken the ${p} paper only where it is light`, () => {
      expect(blendFill(paper, paper).blend).toBe(p === "dark" ? "screen" : "multiply");
    });

    // The edge and the outline are drawn unblended, in their own colours.
    it(`draws the find-match edge on the ${p} paper at 3:1 or more`, () => {
      expect(contrastRatio(at(`appearance.${p}.find-match-edge`), paper)).toBeGreaterThanOrEqual(3);
    });

    for (const a of ACCENTS) {
      it(`draws the ${a} find-current outline on the ${p} paper at 3:1 or more`, () => {
        expect(contrastRatio(at(`accent.${a}.page.${p}.accent`), paper)).toBeGreaterThanOrEqual(3);
      });
    }
  }
});

describe("blended", () => {
  const grey = hex("#808080");

  it("multiplies as the CSS blend mode does: white leaves a colour as it is", () => {
    expect([blended("multiply", WHITE, grey), blended("multiply", grey, WHITE)]).toEqual([grey, grey]);
  });

  it("screens as the CSS blend mode does: black leaves a colour as it is", () => {
    expect([blended("screen", BLACK, grey), blended("screen", grey, BLACK)]).toEqual([grey, grey]);
  });
});

describe("selectionOverlay", () => {
  it("finds the opacity an overlay was mixed at", () => {
    // Teal at 22% over white is the Original page's tint, as the design language derives it.
    expect(selectionOverlay(WHITE, hex("#1F5C63"), hex("#CEDBDD")).a).toBe(0.22);
  });
});
