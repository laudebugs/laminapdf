// Bundled Lamina Sans TrueType font for PDFs (contents pages, flat-form check marks): fetched
// once per session and added as a document blob once per open document.

import laminaSansUrl from "@laminapdf/fonts/pdf/lamina-sans-regular.ttf?url";
import arabicUrl from "@laminapdf/fonts/pdf/noto-sans-arabic-regular.ttf?url";
import type { Viewer } from "@laminapdf/viewer";
import type { Engine } from "./session.ts";

let cachedBytes: Promise<Uint8Array> | undefined;

/** Fetches the bundled Lamina Sans TrueType bytes once (a failed fetch is tried again). */
export function readLaminaSans(): Promise<Uint8Array> {
  cachedBytes ??= fetch(laminaSansUrl)
    .then((answer) => {
      if (!answer.ok) throw new Error(`the bundled PDF font answered ${answer.status}`);
      return answer.arrayBuffer();
    })
    .then((bytes) => new Uint8Array(bytes));
  cachedBytes.catch(() => {
    cachedBytes = undefined;
  });
  return cachedBytes;
}

const documentBlobs = new WeakMap<Viewer, { readonly document: number; readonly blob: Promise<string> }>();

/** Adds Lamina Sans to `viewer`'s current document once and returns its blob ID. */
export async function laminaSansBlob(viewer: Viewer, engine: Engine): Promise<string> {
  const { client } = await engine.view();
  const document = client.document?.id ?? 0;
  const known = documentBlobs.get(viewer);
  if (known?.document === document) return known.blob;
  const blob = readLaminaSans().then((bytes) => engine.addBlob(bytes));
  documentBlobs.set(viewer, { document, blob });
  blob.catch(() => {
    if (documentBlobs.get(viewer)?.blob === blob) documentBlobs.delete(viewer);
  });
  return blob;
}

/** Noto Sans Arabic, for the characters Lamina Sans doesn't have: the summary's fallback font. */
let cachedArabic: Promise<Uint8Array> | undefined;

/** Fetches the bundled Arabic TrueType bytes once (a failed fetch is tried again). */
function readArabic(): Promise<Uint8Array> {
  cachedArabic ??= fetch(arabicUrl)
    .then((answer) => {
      if (!answer.ok) throw new Error(`the bundled Arabic font answered ${answer.status}`);
      return answer.arrayBuffer();
    })
    .then((bytes) => new Uint8Array(bytes));
  cachedArabic.catch(() => {
    cachedArabic = undefined;
  });
  return cachedArabic;
}

/**
 * The fonts a comment summary is set in, which it embeds and subsets: Lamina Sans (Latin, Latin
 * Extended and the punctuation of the interface), then Noto Sans Arabic for Arabic comments. A
 * script neither covers (Greek, Cyrillic, CJK, Hebrew, Indic) makes the summary fail with
 * `missing-glyph`, which the dialog says.
 */
export async function readSummaryFonts(): Promise<Uint8Array[]> {
  return Promise.all([readLaminaSans(), readArabic()]);
}
