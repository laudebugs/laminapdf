// The draw-page example over the hello example's PDF, drawn by the built hayro module.
import { readFileSync } from "node:fs";
import type { RenderBackend } from "@laminapdf/render";
import { initHayro } from "@laminapdf/render-hayro";
import { init } from "@laminapdf/sdk";
import { fromRoot } from "@laminapdf/test-utils/root";
import { beforeAll, describe, expect, it } from "vitest";
import { drawPage } from "../examples/draw-page.js";
import { read } from "./helpers.js";

let renderer: RenderBackend;

beforeAll(async () => {
  await init({ text: true });
  renderer = await initHayro(readFileSync(fromRoot("packages/wasm/dist/lamina_pdf_render_hayro.wasm")));
});

describe("draw-page example", () => {
  it("draws the page at twice its size in points and reads its text", () => {
    const page = drawPage(renderer, read("fixtures/golden/examples/hello.pdf"), 0);
    // hello is A4: 595.28 × 841.89 points.
    expect([page.width, page.height]).toEqual([1190, 1683]);
    expect(page.pixels.length).toBe(page.width * page.height * 4);
    expect(page.pixels.some((v, i) => i % 4 !== 3 && v < 128)).toBe(true);
    expect(page.text).toContain("Hello");
  });
});
