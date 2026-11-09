import { describe, expect, it } from "vitest";
import type { PageInfo } from "./outline.ts";
import { printDpi } from "./output.tsx";

const letter = (index: number) => ({ index, width: 612, height: 792 }) as unknown as PageInfo;

describe("printDpi", () => {
  const pages = Array.from({ length: 500 }, (_, i) => letter(i));

  it("keeps the quality asked for while the pages fit the budget", () => {
    expect(printDpi({ pages: [0, 1, 2], scale: "fit", dpi: 300, annotations: true }, pages)).toEqual({
      dpi: 300,
      lowered: false,
    });
    expect(printDpi({ pages: [0, 1], scale: "fit", dpi: 150, annotations: true }, pages)).toEqual({
      dpi: 150,
      lowered: false,
    });
  });

  it("lowers High quality to Standard for a long print", () => {
    const all = pages.map((_, i) => i);
    expect(printDpi({ pages: all, scale: "fit", dpi: 300, annotations: true }, pages)).toEqual({
      dpi: 150,
      lowered: true,
    });
  });
});
