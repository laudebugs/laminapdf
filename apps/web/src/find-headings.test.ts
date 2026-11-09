// Find's result headings: the page's label, then the section in effect at the page's top.
import type { OutlineTreeItem } from "@laminapdf/ui";
import { describe, expect, it } from "vitest";
import { resultHeadings } from "./find-headings.ts";
import type { PageInfo } from "./outline.ts";

const page = (index: number, label?: string): PageInfo =>
  ({
    index,
    id: `p${index}.0`,
    width: 400,
    height: 800,
    rotation: 0,
    ...(label === undefined ? {} : { label }),
  }) as PageInfo;
const item = (id: string, title: string, at: { page: number; y: number }, children: OutlineTreeItem[] = []) => ({
  id,
  title,
  at,
  children,
});

describe("resultHeadings", () => {
  const pages = [page(0, "i"), page(1), page(2, "A-1"), page(3, "A-2")];
  const outline = [
    item("0", "Preface", { page: 1, y: 0 }),
    item("1", "Appendix", { page: 2, y: 0.5 }, [item("1.0", "Tables", { page: 3, y: 0 })]),
  ];

  it("names the page by its label, or its number without one", () => {
    const heading = resultHeadings(pages, []);
    expect([0, 1, 2].map(heading)).toEqual(["Page i", "Page 2", "Page A-1"]);
  });

  it("adds the section in effect at the page's top, nested items included", () => {
    const heading = resultHeadings(pages, outline);
    // Page 1 is above the first item; page 3's section starts half way down, so its top is still the Preface.
    expect([0, 1, 2, 3].map(heading)).toEqual([
      "Page i",
      "Page 2 · Preface",
      "Page A-1 · Preface",
      "Page A-2 · Tables",
    ]);
  });
});
