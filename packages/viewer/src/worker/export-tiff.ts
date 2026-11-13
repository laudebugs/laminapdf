// TIFF export in the document worker: pages drawn one at a time, as for PNG and JPEG, and handed
// to the TIFF module (`@laminapdf/wasm/tiff`, laminapdf-render's TiffWriter), which this file
// loads the first time a TIFF export starts, so the reader's startup never fetches it. Several
// pages go into one multi-page file while their pixels fit the export's memory budget, else into
// one file each.

import type { ExportedTiff, ExportTiffOptions } from "../protocol.js";
import { current } from "./document.js";
import { drawForExport, exportDpi, scaleFor } from "./export.js";

/** The most pixel bytes one multi-page TIFF holds (256 MiB): past it, a file per page. */
const MAX_MULTI_PAGE_BYTES = 256 * 1024 * 1024;

type TiffGlue = typeof import("@laminapdf/wasm/tiff");

/** The TIFF module's load: the one under way, or a new one after a failed one. */
let loading: Promise<TiffGlue> | undefined;

function tiffModule(): Promise<TiffGlue> {
  if (loading === undefined) {
    const load = import("@laminapdf/wasm/tiff").then(async (glue) => {
      await glue.default();
      return glue;
    });
    loading = load;
    // A failed load is forgotten (the next export tries again), while its callers still see it fail.
    load.catch(() => {
      if (loading === load) loading = undefined;
    });
  }
  return loading;
}

/**
 * Pages `indexes` (0-based, in order) drawn at `options.dpi` on white, as TIFF: one file when
 * `options.multiPage` asks for it and the pages' pixels fit MAX_MULTI_PAGE_BYTES, else a file
 * per page.
 */
export async function exportTiff(
  indexes: readonly number[],
  options: ExportTiffOptions,
  signal: AbortSignal,
): Promise<ExportedTiff> {
  if (indexes.length === 0) throw new RangeError("a TIFF export needs at least one page");
  const glue = await tiffModule();
  signal.throwIfAborted();
  const together = options.multiPage && pixelBytes(indexes, options.dpi) <= MAX_MULTI_PAGE_BYTES;
  const files: Uint8Array<ArrayBuffer>[] = [];
  const dpis: number[] = [];
  let file: InstanceType<TiffGlue["TiffFile"]> | undefined;
  try {
    for (const index of indexes) {
      const { tile, dpi } = await drawForExport(index, options.dpi, signal);
      file ??= new glue.TiffFile();
      const pixels = new Uint8Array(tile.pixels.buffer, tile.pixels.byteOffset, tile.pixels.byteLength);
      file.addPage(tile.width, tile.height, pixels, dpi);
      dpis.push(dpi);
      if (!together) {
        files.push(finished(file));
        file = undefined;
      }
    }
    if (file !== undefined) files.push(finished(file));
    file = undefined;
  } finally {
    file?.free();
  }
  return { files, dpis };
}

/** `file`'s bytes, in a buffer of their own, and `file` freed. */
function finished(file: { finish(): Uint8Array; free(): void }): Uint8Array<ArrayBuffer> {
  try {
    return new Uint8Array(file.finish());
  } finally {
    file.free();
  }
}

/** The RGBA bytes `indexes` hold drawn at `dpi` (each page lowered as {@link exportDpi} lowers it). */
function pixelBytes(indexes: readonly number[], dpi: number): number {
  const { render } = current();
  let total = 0;
  for (const index of indexes) {
    const page = render.page(index);
    const scale = scaleFor(exportDpi(index, dpi));
    total += Math.floor(page.width * scale) * Math.floor(page.height * scale) * 4;
  }
  return total;
}
