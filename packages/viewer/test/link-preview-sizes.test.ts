// The link preview's sizes in the viewer are the design tokens the preview card takes (§7.27): the
// viewer can't import @laminapdf/tokens, so this pins the two homes of each number together. The
// file is read through Vite (`?raw`), so the test needs no Node API.
import { describe, expect, it } from "vitest";
import { PAGE_PREVIEW, PREVIEW_MAX } from "../src/layers/link-preview.ts";

describe("the link preview's sizes", () => {
  it("are the design tokens the preview card takes", () => {
    const raw = Object.values(
      import.meta.glob<string>("../../tokens/tokens.json", { query: "?raw", import: "default", eager: true }),
    )[0];
    const tokens = JSON.parse(raw ?? "{}") as {
      component: Record<string, { $value: { value: number } }>;
    };
    const px = (name: string) => tokens.component[name]?.$value.value;
    expect([PREVIEW_MAX.width, PAGE_PREVIEW.width, PAGE_PREVIEW.height]).toEqual([
      px("link-preview-crop-max-width"),
      px("link-preview-crop-width"),
      px("link-preview-crop-height"),
    ]);
  });
});
