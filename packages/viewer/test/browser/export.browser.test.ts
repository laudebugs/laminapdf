// Page export in the worker: a page drawn at the resolution asked for, as PNG or JPEG, its size
// in pixels what the resolution gives and the resolution written into the file.
import { init } from "@laminapdf/sdk";
import type { ViewerClient } from "@laminapdf/viewer";
import { afterEach, beforeAll, describe, expect, it } from "vitest";
import { read, viewer } from "./helpers.js";

/** One US Letter page (612 × 792 points). */
const LETTER = "fixtures/corpus/hello-xrefstm.pdf";

beforeAll(() => init());

let client: ViewerClient | undefined;
afterEach(async () => {
  await client?.close();
  client = undefined;
});

async function opened(backend: "pdfium" | "hayro"): Promise<ViewerClient> {
  client = await viewer(backend);
  await client.open(await read(LETTER));
  return client;
}

/** The PNG's size from its IHDR, and its `pHYs` pixels per metre. */
function png(bytes: Uint8Array): { width: number; height: number; perMetre: number | undefined } {
  const view = new DataView(bytes.buffer, bytes.byteOffset);
  const at = bytes.findIndex((_, i) => String.fromCharCode(...bytes.subarray(i, i + 4)) === "pHYs");
  return {
    width: view.getUint32(16),
    height: view.getUint32(20),
    perMetre: at < 0 ? undefined : view.getUint32(at + 4),
  };
}

describe.each(["pdfium", "hayro"] as const)("exportPage with %s", (backend) => {
  it("draws Letter at 150 dpi as a PNG of 1,275 × 1,650 pixels, its resolution recorded", async () => {
    const page = await (await opened(backend)).exportPage(0, { format: "png", dpi: 150 });
    expect([page.width, page.height, page.dpi]).toEqual([1275, 1650, 150]);
    expect(png(page.image)).toEqual({ width: 1275, height: 1650, perMetre: Math.round(150 / 0.0254) });
  });

  it("writes a JPEG with its density in dots per inch", async () => {
    const page = await (await opened(backend)).exportPage(0, { format: "jpeg", dpi: 96, quality: 80 });
    const jpeg = page.image;
    expect([jpeg[0], jpeg[1], jpeg[2], jpeg[3]]).toEqual([0xff, 0xd8, 0xff, 0xe0]);
    const view = new DataView(jpeg.buffer, jpeg.byteOffset);
    expect([jpeg[13], view.getUint16(14), view.getUint16(16)]).toEqual([1, 96, 96]);
    expect([page.width, page.height]).toEqual([816, 1056]);
  });
});
