// Printing on the web (R09): the pages drawn for printing go into a hidden, same-origin frame, one
// image per page, and the frame's own print() opens the browser's print window, so the reader
// around it never prints. The frame inherits the reader's Content Security Policy, which allows no
// inline style: page sizes go into a constructable stylesheet made in the frame's own realm and
// adopted by its document (CSSOM, which the policy doesn't govern), images are `blob:` URLs, and
// sizes are set with `style.setProperty`. Each page gets a named page (`@page p<N>`) of its own
// size, so a document that mixes sizes and orientations prints each page on its own paper where
// the browser honours named pages, and fits it to the paper where it doesn't.

import type { PrintJob } from "./types.js";

/** The parts of a window printing uses: tests pass a fake. */
export interface PrintScope {
  readonly document: Document;
  readonly URL: { createObjectURL(blob: Blob): string; revokeObjectURL(url: string): void };
}

/** The frame's window, with the constructor its stylesheets must come from. */
type FrameWindow = Window & { readonly CSSStyleSheet: typeof CSSStyleSheet };

/** The CSS rules for `pages` (sizes in points), page `i` the named page `p<i>`. */
export function printRules(pages: readonly { width: number; height: number }[], scale: PrintJob["scale"]): string[] {
  const rules = [
    "html, body { margin: 0; padding: 0; background: white; }",
    "img.lamina-print-page { display: block; break-after: page; }",
    "img.lamina-print-page:last-child { break-after: auto; }",
  ];
  // Fit to paper: a page the paper can't hold (a browser that ignores named page sizes, or paper
  // the print window changed) shrinks to it, keeping its shape, rather than being cut off.
  if (scale === "fit") {
    rules.push("img.lamina-print-page { max-inline-size: 100vw; max-block-size: 100vh; object-fit: contain; }");
  }
  pages.forEach(({ width, height }, i) => {
    const size = scale === "fit" ? `size: ${round(width)}pt ${round(height)}pt; ` : "";
    rules.push(`@page p${i} { ${size}margin: 0; }`, `.lamina-print-page-${i} { page: p${i}; }`);
  });
  return rules;
}

const round = (n: number) => Math.round(n * 100) / 100;

/** How long the frame waits for `afterprint` before letting the pages go. */
const AFTERPRINT_WAIT_MS = 10 * 60_000;

/** Prepares `job`'s pages in a hidden frame, then opens the print window. */
export async function printPages(scope: PrintScope, job: PrintJob): Promise<void> {
  const signal = job.signal ?? new AbortController().signal;
  signal.throwIfAborted();
  const frame = scope.document.createElement("iframe");
  frame.setAttribute("aria-hidden", "true");
  frame.tabIndex = -1;
  for (const [property, value] of [
    ["position", "fixed"],
    ["right", "0"],
    ["bottom", "0"],
    ["width", "0"],
    ["height", "0"],
    ["border", "0"],
  ]) {
    frame.style.setProperty(property as string, value as string);
  }
  scope.document.body.append(frame);
  const urls: string[] = [];
  const cleanUp = () => {
    for (const url of urls) scope.URL.revokeObjectURL(url);
    frame.remove();
  };
  try {
    const win = frame.contentWindow as FrameWindow | null;
    const doc = frame.contentDocument;
    if (win === null || doc === null) throw new Error("the print frame has no document");
    const sizes: { width: number; height: number }[] = [];
    for (const [i, page] of job.pages.entries()) {
      signal.throwIfAborted();
      const { image, width, height } = await job.render(page, signal);
      const url = scope.URL.createObjectURL(image);
      urls.push(url);
      const img = doc.createElement("img");
      img.className = `lamina-print-page lamina-print-page-${i}`;
      img.alt = "";
      img.style.setProperty("width", `${round(width)}pt`);
      img.style.setProperty("height", `${round(height)}pt`);
      img.src = url;
      doc.body.append(img);
      // Every image is decoded before print(): Safari prints what it has when print() is called.
      await img.decode();
      sizes.push({ width, height });
      job.onProgress?.(i + 1, job.pages.length);
    }
    signal.throwIfAborted();
    const sheet = new win.CSSStyleSheet();
    for (const rule of printRules(sizes, job.scale)) sheet.insertRule(rule, sheet.cssRules.length);
    doc.adoptedStyleSheets = [sheet];
    await new Promise<void>((resolve) => {
      // Chromium's print() returns once the print window closes; Firefox's and Safari's at once,
      // then `afterprint` follows the window. Whichever comes last lets the frame go.
      let pending = 2;
      const done = () => {
        pending -= 1;
        if (pending === 0) resolve();
      };
      win.addEventListener("afterprint", done, { once: true });
      win.focus();
      win.print();
      done();
      // A browser that never sends afterprint for a frame mustn't keep the pages forever. Firefox
      // and Safari return from print() at once, so the window may stay open a while: ten minutes.
      setTimeout(() => {
        if (pending > 0) resolve();
      }, AFTERPRINT_WAIT_MS);
    });
  } finally {
    cleanUp();
  }
}
