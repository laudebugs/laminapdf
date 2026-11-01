// C3b's demo (and C3c's harness page): `pnpm --filter @laminapdf/viewer exec vite examples`
// after `cargo xtask wasm`, open viewer-demo.html, pick a PDF. Scroll, ctrl+wheel or pinch to
// zoom, + / - / 0 on the keyboard. `renderstats` goes to the console.
import pdfiumWasm from "@embedpdf/pdfium/pdfium.wasm?url";
import { createViewer, ViewerClient, type Zoom } from "@laminapdf/viewer";

const status = document.querySelector<HTMLSpanElement>("#status");
const host = document.querySelector<HTMLDivElement>("#viewer");
const input = document.querySelector<HTMLInputElement>("#file");

function say(text: string): void {
  if (status !== null) status.textContent = text;
}

const client = await ViewerClient.create({ backend: { backend: "pdfium", wasm: pdfiumWasm } });
const viewer = createViewer({ client });
if (host !== null) viewer.mount(host);
viewer.subscribe(() => {
  const { currentPage, pageCount, percent } = viewer.getState();
  if (pageCount > 0) say(`Page ${currentPage + 1} of ${pageCount} · ${Math.round(percent)} %`);
});
viewer.on("renderstats", (stats) => console.debug("renderstats", stats));
say("Ready: choose a PDF.");

input?.addEventListener("change", async () => {
  const file = input.files?.[0];
  if (file === undefined) return;
  try {
    viewer.open(await client.open(file));
    viewer.element.focus();
  } catch (e) {
    say(`Couldn't open it: ${e instanceof Error ? e.message : String(e)}`);
  }
});

for (const button of document.querySelectorAll<HTMLButtonElement>("button[data-zoom]")) {
  button.addEventListener("click", () => viewer.setZoom(button.dataset.zoom as Zoom));
}
document.querySelector("#rotate")?.addEventListener("click", () => viewer.rotate(90));
