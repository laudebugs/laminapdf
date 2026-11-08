// The page-comparison example over the examples' reference PDFs, drawn by the built hayro module.
import { readFileSync } from "node:fs";
import type { RenderBackend } from "@laminapdf/render";
import { fromRoot } from "@laminapdf/test-utils/root";
import { beforeAll, describe, expect, it } from "vitest";
import { comparePages, loadRenderer } from "../examples/compare-pages.js";
import { read } from "./helpers.js";

const hello = read("fixtures/golden/examples/hello.pdf");
const invoice = read("fixtures/golden/examples/invoice.pdf");
const fourPages = read("fixtures/golden/examples/links_and_outlines.pdf");
/** A4 in points: hello's and the invoice's page. */
const A4 = { width: 595.28, height: 841.89 };

let renderer: RenderBackend;

beforeAll(async () => {
  renderer = await loadRenderer(readFileSync(fromRoot("packages/wasm/dist/lamina_pdf_render_hayro.wasm")));
});

describe("compare-pages example", () => {
  it("finds no differing pixel between a PDF and itself", () => {
    expect(comparePages(renderer, hello, hello)).toEqual({
      pageCounts: [1, 1],
      pages: [{ page: 0, pixels: 0, box: null }],
    });
  });

  it("boxes the differing pixels of two PDFs inside the page", () => {
    const [page] = comparePages(renderer, hello, invoice).pages;
    expect(page?.pixels).toBeGreaterThan(0);
    const box = page?.box;
    expect(box).not.toBeNull();
    if (box === null || box === undefined) return;
    expect(box.x).toBeGreaterThanOrEqual(0);
    expect(box.y).toBeGreaterThanOrEqual(0);
    expect(box.x + box.width).toBeLessThanOrEqual(A4.width);
    expect(box.y + box.height).toBeLessThanOrEqual(A4.height);
  });

  it("reports different page counts and compares the pages both have", () => {
    const comparison = comparePages(renderer, hello, fourPages);
    expect(comparison.pageCounts).toEqual([1, 4]);
    expect(comparison.pages).toHaveLength(1);
  });
});
