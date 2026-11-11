// "View original page" in the approximate reader mode: once the page view is back, a block's box
// is outlined on its page for a moment, so the reader sees which part of the page the text came
// from. A page layer of the viewer's (its own slot above the highlights), placed in shares of the
// page's size so zooming keeps it in place.
import type { PageLayer, Viewer } from "@laminapdf/viewer";

/** How long the outline stays, in ms. */
const OUTLINE_MS = 2000;

/** A box on a page, `[x, y, width, height]` in points from the top left, and the page's size. */
export interface OriginBox {
  readonly page: number;
  readonly bbox: readonly [number, number, number, number];
  readonly size: { readonly width: number; readonly height: number };
}

/** `n` of `whole` as a CSS percentage. */
const share = (n: number, whole: number) => `${(100 * n) / whole}%`;

/** Outlines `box` on its page for {@link OUTLINE_MS}; the returned function removes it sooner. */
export function outlineOrigin(viewer: Viewer, box: OriginBox): () => void {
  const [x, y, width, height] = box.bbox;
  const outline = document.createElement("div");
  outline.className = "web-reader-origin";
  outline.style.left = share(x, box.size.width);
  outline.style.top = share(y, box.size.height);
  outline.style.width = share(width, box.size.width);
  outline.style.height = share(height, box.size.height);
  const layer: PageLayer = {
    name: "reader-origin",
    onMount(page, element) {
      if (page === box.page) element.append(outline);
    },
    onUnmount(page) {
      if (page === box.page) outline.remove();
    },
  };
  const remove = viewer.addLayer(layer);
  const done = () => {
    window.clearTimeout(timer);
    outline.remove();
    remove();
  };
  const timer = window.setTimeout(done, OUTLINE_MS);
  return done;
}
