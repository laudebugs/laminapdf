// Other PDFs the open document takes pages from (the E2 plan §2.1): Insert from a file, Replace
// and Merge. A dialog first previews the file (its pages drawn as thumbnails by the backend, and
// the claims its metadata makes), then, once the user applies, the file becomes a source of the
// open document (`addSource`), whose ops name it by the SHA-256 of its bytes. The source is added
// to the text-capable copy too, so the copy can make the same edits.

import type { RenderDocument, RenderedTile } from "@laminapdf/render";
import { Document } from "@laminapdf/sdk";
import type { AddedSource, SourcePreview } from "../protocol.js";
import { current, currentBackend, followEdit } from "./document.js";
import { serial } from "./edit.js";

/** The previews open, by key: each a render document over a file's bytes. */
const previews = new Map<number, RenderDocument>();
let nextPreview = 1;

/**
 * Opens `bytes` to preview: its pages' sizes, its PDF/A and PDF/UA claims and its title, and a
 * key to draw its pages by (`renderSource`) until `closeSource`.
 *
 * @throws `password-required` or `wrong-password` for an encrypted file without its password;
 *   the errors of a file that doesn't open.
 */
export function previewSource(bytes: Uint8Array, password: string | undefined): SourcePreview {
  const options = password === undefined ? {} : { password };
  // Lamina PDF reads the claims; it opens the file first, so its errors are the ones reported.
  const model = Document.open(bytes, options);
  let claims: Pick<SourcePreview, "pdfa" | "pdfua" | "title">;
  let encrypted: boolean;
  try {
    const metadata = model.metadata();
    claims = {
      ...(metadata.pdfa === undefined ? {} : { pdfa: metadata.pdfa }),
      ...(metadata.pdfua === undefined ? {} : { pdfua: metadata.pdfua }),
      ...(metadata.title === undefined ? {} : { title: metadata.title }),
    };
    encrypted = model.isEncrypted;
  } finally {
    model.free();
  }
  const render = currentBackend().open(
    { size: bytes.length, readSync: (offset, length) => bytes.subarray(offset, offset + length) },
    options,
  );
  let pages: SourcePreview["pages"];
  try {
    pages = Array.from({ length: render.pageCount }, (_, i) => {
      const { width, height, rotate } = render.page(i);
      return { width, height, rotate };
    });
  } catch (error) {
    render.close();
    throw error;
  }
  const key = nextPreview++;
  previews.set(key, render);
  return { key, pages, encrypted, ...claims };
}

/** Page `page` of preview `key`, at `scale`. */
export function renderSource(key: number, page: number, scale: number, signal: AbortSignal): RenderedTile {
  const render = previews.get(key);
  if (render === undefined) throw new Error("that preview is closed");
  return render.render({ page, scale }, { signal });
}

/** Closes preview `key`; closing one that is closed does nothing. */
export function closeSource(key: number): undefined {
  previews.get(key)?.close();
  previews.delete(key);
  return undefined;
}

/**
 * Makes `bytes` a source of document `id`: its ID and page IDs, for the ops that copy its pages.
 * Adding the same file again gives the same ID and adds nothing.
 */
export function addSource(
  bytes: Uint8Array,
  password: string | undefined,
  id: number,
  signal: AbortSignal,
): Promise<AddedSource> {
  const opened = current(id);
  return serial(opened, async () => {
    const { model } = opened;
    const options = password === undefined ? {} : { password };
    const added = await model.ensure(() => model.addSource(bytes, options), { signal });
    followEdit(opened, { source: bytes, ...options });
    return { id: added.id, pages: added.pages };
  });
}
