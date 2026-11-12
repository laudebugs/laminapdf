// The docs site's "Show a PDF" example (examples/show-a-pdf.ts) in Chromium, Firefox and WebKit:
// a PDF opens, its first page paints, and disposing of it empties the container.
import pdfiumWasm from "@embedpdf/pdfium/pdfium.wasm?url";
import { expect, it } from "vitest";
import { showPdf } from "../../examples/show-a-pdf.js";
import { read } from "./helpers.js";

/** Waits until `check` holds, up to `ms`. */
async function until(check: () => boolean, ms = 15_000): Promise<void> {
  const end = performance.now() + ms;
  while (!check()) {
    if (performance.now() > end) throw new Error("timed out");
    await new Promise((resolve) => setTimeout(resolve, 50));
  }
}

it("shows a PDF and takes it down", async () => {
  const container = document.createElement("div");
  Object.assign(container.style, { width: "600px", height: "400px" });
  document.body.append(container);
  const bytes = new Uint8Array(await read("fixtures/golden/examples/hello.pdf"));
  const file = new Blob([bytes], { type: "application/pdf" });
  {
    using shown = await showPdf(container, file, pdfiumWasm);
    expect(shown.pageCount).toBe(1);
    await until(() => {
      const canvas = container.querySelector("canvas");
      return canvas !== null && canvas.width > 0;
    });
  }
  expect(container.querySelector("canvas")).toBeNull();
  container.remove();
});

it("ends the Web Worker when the file won't open", async () => {
  const container = document.createElement("div");
  document.body.append(container);
  const notPdf = new Blob([new TextEncoder().encode("not a PDF")]);
  await expect(showPdf(container, notPdf, pdfiumWasm)).rejects.toThrow();
  expect(container.childElementCount).toBe(0);
  container.remove();
});
