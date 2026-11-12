// The docs' render-page example in Chromium, Firefox and WebKit: the hello example's PDF drawn by
// hayro onto an OffscreenCanvas at the width asked for, with dark pixels where its text is.
import { initHayro } from "@laminapdf/render-hayro";
import { describe, expect, it } from "vitest";
import { renderPage } from "../../examples/render-page.js";

const WIDTH = 600;
const urls = import.meta.glob<string>("../../../../fixtures/golden/examples/hello.pdf", {
  query: "?url",
  import: "default",
  eager: true,
});

describe("render-page example", () => {
  it("draws the page onto the canvas at the width asked for, as tall as the page", async () => {
    const renderer = await initHayro();
    const url = urls["../../../../fixtures/golden/examples/hello.pdf"];
    if (url === undefined) throw new Error("hello.pdf isn't served");
    const bytes = new Uint8Array(await (await fetch(url)).arrayBuffer());
    const canvas = new OffscreenCanvas(1, 1);
    renderPage(renderer, bytes, 0, WIDTH, canvas);
    // hello is A4, 595.28 × 841.89 points: 600.5 / 595.28 pixels a point makes it 849 tall.
    expect([canvas.width, canvas.height]).toEqual([WIDTH, 849]);
    const { data } = canvas.getContext("2d")?.getImageData(0, 0, canvas.width, canvas.height) ?? { data: [] };
    expect(Array.from(data).some((v, i) => i % 4 !== 3 && v < 128)).toBe(true);
  });
});
