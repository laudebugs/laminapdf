// Print comment summary (C07): the summary's pages drawn one at a time in the document worker and
// handed to the browser's print window, since the web has no native print of a PDF's bytes
// (R09's page-appearance warning applies). The summary is a new file, so it is opened as a
// preview source, which draws a file that isn't the open document, and let go once printed.

import type { Platform } from "@laminapdf/ui";
import { closeSource, previewSource, renderSource, type ViewerClient } from "@laminapdf/viewer";

/** The resolution the summary prints at: 150 dpi, which is plain text and thumbnails. */
const DPI = 150;

/** `bitmap` as a PNG file. */
async function png(bitmap: ImageBitmap, width: number, height: number): Promise<Blob> {
  const canvas = new OffscreenCanvas(width, height);
  const context = canvas.getContext("2d");
  if (context === null) throw new Error("no 2D context for printing");
  context.drawImage(bitmap, 0, 0);
  bitmap.close();
  return canvas.convertToBlob({ type: "image/png" });
}

/** Prints the PDF `bytes` through `platform`, page by page; resolves once the print window has opened. */
export async function printSummary(client: ViewerClient, platform: Platform, bytes: Uint8Array): Promise<void> {
  const preview = await previewSource(client, bytes);
  try {
    await platform.print({
      pages: preview.pages.map((_, i) => i),
      scale: "fit",
      render: async (page, signal) => {
        const size = preview.pages[page];
        if (size === undefined) throw new Error(`the summary has no page ${page + 1}`);
        const { bitmap, width, height } = await renderSource(client, preview.key, page, DPI / 72, { signal });
        return { image: await png(bitmap, width, height), width: size.width, height: size.height };
      },
    });
  } finally {
    await closeSource(client, preview.key);
  }
}
