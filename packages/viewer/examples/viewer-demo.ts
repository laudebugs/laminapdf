// C3b's demo (and C3c's harness page): `pnpm --filter @laminapdf/viewer exec vite examples`
// after `cargo xtask wasm`, open viewer-demo.html, pick a PDF. Scroll, ctrl+wheel or pinch to
// zoom, + / - / 0 on the keyboard. "Turn this page" edits the file: the page in view turns a
// quarter and redraws. `renderstats` goes to the console.
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

// An edit of the file, not the view: the page's own rotation turns, and only that page redraws.
document.querySelector("#turn-page")?.addEventListener("click", async () => {
  const index = viewer.getState().currentPage;
  const page = client.document?.pages[index];
  if (page === undefined) return;
  const rotate = ((page.rotation + 90) % 360) as 0 | 90 | 180 | 270;
  try {
    await viewer.applyEdit([{ op: "rotatePage", args: { page: page.id, rotate } }]);
  } catch (e) {
    say(`Couldn't turn it: ${e instanceof Error ? e.message : String(e)}`);
  }
});
