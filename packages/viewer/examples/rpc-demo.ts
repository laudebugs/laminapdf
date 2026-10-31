// The C3a manual check: `pnpm --filter @lamina/viewer exec vite examples` (after `cargo xtask
// wasm`), open the page, pick a PDF. The worker starts and loads PDFium before a file is chosen.
import pdfiumWasm from "@embedpdf/pdfium/pdfium.wasm?url";
import { ViewerClient } from "@lamina/viewer";

const status = document.querySelector<HTMLDivElement>("#status");
const canvas = document.querySelector<HTMLCanvasElement>("#page");
const input = document.querySelector<HTMLInputElement>("#file");

function say(text: string): void {
  if (status !== null) status.textContent = text;
}

const client = await ViewerClient.create({ backend: { backend: "pdfium", wasm: pdfiumWasm } });
client.on("warning", (w) => console.warn(w.code, w.message));
client.on("render-slow", () => say("This page is taking a long time…"));
say("Ready: choose a PDF.");

input?.addEventListener("change", async () => {
  const file = input.files?.[0];
  if (file === undefined || canvas === null) return;
  try {
    const opened = await client.open(file);
    say(`${opened.pageCount} page(s), drawn by ${opened.backend.name} ${opened.backend.version}.`);
    const { bitmap, width, height } = await client.render({ page: 0, scale: devicePixelRatio });
    canvas.width = width;
    canvas.height = height;
    canvas.style.width = `${width / devicePixelRatio}px`;
    canvas.getContext("2d")?.drawImage(bitmap, 0, 0);
    bitmap.close();
  } catch (e) {
    say(`Couldn't open it: ${e instanceof Error ? e.message : String(e)}`);
  }
});
