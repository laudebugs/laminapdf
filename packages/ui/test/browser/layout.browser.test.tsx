import { afterEach, describe, expect, it } from "vitest";
import { Select } from "../../src/index.ts";
import { EXAMPLES } from "../states.tsx";
import { cleanup, renderIn, targets } from "./render.tsx";

afterEach(cleanup);

/** Every target under the minimum, with its size. */
function tooSmall(min: number, density: "desktop" | "touch"): string[] {
  return EXAMPLES.flatMap((example) => {
    const host = renderIn(example.element(), { density });
    return targets(host).flatMap((el) => {
      const { width, height } = el.getBoundingClientRect();
      const name = el.getAttribute("aria-label") ?? el.textContent ?? el.tagName;
      return width + 0.5 < min || height + 0.5 < min
        ? [`${example.component} ${example.state}: ${name} ${width}×${height}`]
        : [];
    });
  });
}

describe("target sizes (WCAG 2.5.8, design language §8.7)", () => {
  it("are at least 24 × 24 at desktop density", () => {
    expect(tooSmall(24, "desktop")).toEqual([]);
  });

  it("are at least 44 × 44 at touch density", () => {
    expect(tooSmall(44, "touch")).toEqual([]);
  });

  it("give fields 16 px text on touch", () => {
    const host = renderIn(
      <Select label="Size" options={[{ value: "a", label: "A" }]} value="a" onChange={() => {}} />,
      {
        density: "touch",
      },
    );
    expect(getComputedStyle(host.querySelector("select") as HTMLElement).fontSize).toBe("16px");
  });
});

describe("right to left", () => {
  it("puts the select's chevron at the start edge's opposite end, mirrored", () => {
    const box = (dir: "ltr" | "rtl") => {
      const host = renderIn(
        <Select label="Size" options={[{ value: "a", label: "A" }]} value="a" onChange={() => {}} />,
        {
          dir,
        },
      );
      const select = (host.querySelector("select") as HTMLElement).getBoundingClientRect();
      const chevron = (host.querySelector(".paper-select-chevron") as Element).getBoundingClientRect();
      return { select, chevron };
    };
    const ltr = box("ltr");
    expect(ltr.select.right - ltr.chevron.right).toBeCloseTo(8, 0);
    const rtl = box("rtl");
    expect(rtl.chevron.left - rtl.select.left).toBeCloseTo(8, 0);
  });
});
