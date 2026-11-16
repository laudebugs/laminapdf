// The export dialog's reads in the document worker: the open document's text, pictures, links
// and bookmarks as files, and how many of each it has. Each reads the extract module's document
// (`textModel`), which follows the edit session, so an export gives the document as it is now,
// edits included. Images → PDF builds a new document in the same module and needs none open.

import { imagesToPdf as buildPdf, type ExtractedImage, Image, LaminaPdfError, type OutlineNode } from "@laminapdf/sdk";
import { docsUrl } from "@laminapdf/sdk/explain";
import type {
  ExportSummary,
  ExtractedPicture,
  ExtractedPictures,
  ExtractedText,
  ExtractPicturesOptions,
  ExtractTextOptions,
  ImagesToPdfOptions,
  ImageToPdf,
  LinksAndBookmarks,
} from "../protocol.js";
import { current, extractModule, type Open, textModel } from "./document.js";

/** The open document's text, as `options` asks. */
export async function extractText(options: ExtractTextOptions, signal: AbortSignal): Promise<ExtractedText> {
  const model = await textModel(signal);
  const { format, pages, readingOrder } = options;
  const asked = { readingOrder, ...(pages === undefined ? {} : { pages }) };
  const text =
    format === "json"
      ? JSON.stringify(await model.ensure(() => model.extract({ format, ...asked }), { signal }), null, 2)
      : await model.ensure(() => model.extract({ format, ...asked }), { signal });
  const tagged = await model.ensure(() => model.isTagged, { signal });
  return { text, tagged };
}

/** The open document's pictures as files, as `options` asks. */
export async function extractPictures(
  options: ExtractPicturesOptions,
  signal: AbortSignal,
): Promise<ExtractedPictures> {
  const model = await textModel(signal);
  const { pages, keepOriginals, minimumSize } = options;
  const read = await model.ensure(() => model.extractImages(pages === undefined ? {} : { pages: [...pages] }), {
    signal,
  });
  const pictures: ExtractedPicture[] = [];
  const perPage = new Map<number, number>();
  let small = 0;
  for (const image of read.images) {
    if (Math.max(image.width, image.height) < minimumSize) {
      small++;
      continue;
    }
    const file = keepOriginals ? undefined : await asPng(image);
    signal.throwIfAborted();
    const n = (perPage.get(image.page) ?? 0) + 1;
    perPage.set(image.page, n);
    pictures.push({
      page: image.page,
      name: `page-${image.page + 1}-picture-${n}.${file === undefined ? image.extension : "png"}`,
      bytes: file ?? image.data,
      width: image.width,
      height: image.height,
    });
  }
  return { pictures, skipped: read.skipped.length + small };
}

/**
 * `image` decoded and written as PNG, for a JPEG where the worker can decode one; undefined
 * leaves it as stored: a PNG already, or JPEG 2000, which browsers don't decode.
 */
async function asPng(image: ExtractedImage): Promise<Uint8Array<ArrayBuffer> | undefined> {
  if (image.format !== "jpeg" || typeof createImageBitmap !== "function" || typeof OffscreenCanvas !== "function") {
    return undefined;
  }
  // The file's own orientation tags don't apply: a PDF draws the image's pixels as stored.
  const bitmap = await createImageBitmap(new Blob([image.data], { type: "image/jpeg" }), {
    imageOrientation: "none",
  });
  try {
    const canvas = new OffscreenCanvas(bitmap.width, bitmap.height);
    const context = canvas.getContext("2d");
    if (context === null) return undefined;
    context.drawImage(bitmap, 0, 0);
    const blob = await canvas.convertToBlob({ type: "image/png" });
    return new Uint8Array(await blob.arrayBuffer());
  } finally {
    bitmap.close();
  }
}

/** The last summary read, and the document it was read from: an edit forgets it. */
let summary: { readonly open: Open; readonly summary: ExportSummary } | undefined;

/**
 * How many links and bookmarks the open document has, and whether it is tagged, kept until
 * {@link forgetSummary}. Not its pictures: the SDK counts them only by decoding every one, which a
 * dialog opening shouldn't cost on a long scanned file.
 */
export async function exportSummary(signal: AbortSignal): Promise<ExportSummary> {
  const opened = current();
  if (summary?.open === opened) return summary.summary;
  const model = await textModel(signal);
  const links = (JSON.parse(await model.ensure(() => model.linksJson(), { signal })) as { links: unknown[] }).links
    .length;
  const bookmarks = count(await model.ensure(() => model.outline(), { signal }));
  const tagged = await model.ensure(() => model.isTagged, { signal });
  const read = { links, bookmarks, tagged };
  // An edit made while this read ran forgot the summary before it was kept: keep only a read
  // of the document still open, which the next edit forgets in turn.
  if (current() === opened) summary = { open: opened, summary: read };
  return read;
}

/** The items of `nodes`, at every level. */
function count(nodes: readonly OutlineNode[]): number {
  return nodes.reduce((sum, node) => sum + 1 + count(node.children ?? []), 0);
}

/** Forgets the summary: the document changed (an edit, undo or redo) or closed. */
export function forgetSummary(): void {
  summary = undefined;
}

/** The open document's links and bookmarks as JSON or CSV. */
export async function linksAndBookmarks(format: "json" | "csv", signal: AbortSignal): Promise<LinksAndBookmarks> {
  const model = await textModel(signal);
  const json = format === "json";
  const links = await model.ensure(() => (json ? model.linksJson() : model.linksCsv()), { signal });
  const bookmarks = await model.ensure(() => (json ? model.outlineJson() : model.outlineCsv()), { signal });
  return { links, bookmarks };
}

/** The SDK's page sizes for {@link ImagesToPdfOptions.pageSize}: none fits the page to the image. */
const FITS = { image: undefined, letter: "Letter", a4: "A4" } as const;

/** A new PDF's bytes, one page per image; the open document, if any, is untouched. */
export async function imagesToPdf(
  images: readonly ImageToPdf[],
  options: ImagesToPdfOptions,
  signal: AbortSignal,
): Promise<Uint8Array<ArrayBuffer>> {
  await extractModule();
  signal.throwIfAborted();
  const decoded = images.map(imageOf);
  try {
    const fit = FITS[options.pageSize];
    // Freed by hand, not `using`: some browsers the reader supports lack Symbol.dispose.
    const doc = buildPdf(decoded, { margin: options.margin, ...(fit === undefined ? {} : { fit }) });
    try {
      return doc.save();
    } finally {
      doc.free();
    }
  } finally {
    for (const image of decoded) image.free();
  }
}

/** `image` as the SDK's Image, by its MIME type. */
function imageOf(image: ImageToPdf): Image {
  if (image.type === "image/jpeg") return Image.jpeg(image.bytes);
  if (image.type === "image/png") return Image.png(image.bytes);
  throw new LaminaPdfError({
    code: "unsupported-image",
    kind: "unsupported",
    message: "a page can hold a JPEG or a PNG picture",
    hint: "convert the picture to JPEG or PNG",
    data: { type: image.type },
    docsUrl: docsUrl("unsupported-image"),
  });
}
