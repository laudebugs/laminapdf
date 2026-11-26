// Comments to and from other apps, and the comment summary, in the document worker (E5 plan §2.3).
// Export and the import preview read the extract module's document, which follows the edit
// session, so they give the document as it is now, edits included. Import is an edit (edit.ts's
// `importComments`). The summary is a new document: the worker writes the open document as a save
// would, opens that copy in the text module (which has the fonts), lays the summary out, saves it
// and lets both go, so the edit session is untouched and the page only ever holds the saved bytes.

import type { ExportCommentsOptions, ImportCommentsOptions, ImportSummary } from "@laminapdf/sdk";
import { Document, Font, loadText } from "@laminapdf/sdk";
import type { CommentSummaryRequest, MadeSummary } from "../protocol.js";
import { current, textModel } from "./document.js";
import { serial } from "./edit.js";

/** Document `id`'s comments as a file in the format `options` name, as its edits leave them; the PDF doesn't change. */
export function exportComments(options: ExportCommentsOptions, id: number, signal: AbortSignal): Promise<Uint8Array> {
  return serial(current(id), async () => {
    const model = await textModel(signal);
    return model.ensure(() => model.exportAnnotations(options), { signal });
  });
}

/**
 * What importing `bytes` into document `id` would do, without doing it.
 *
 * @throws `interchange-malformed`, `interchange-unknown-format`, `interchange-dtd`,
 *   `interchange-too-large`.
 */
export function previewImport(
  bytes: Uint8Array,
  options: ImportCommentsOptions,
  id: number,
  signal: AbortSignal,
): Promise<ImportSummary> {
  return serial(current(id), async () => {
    const model = await textModel(signal);
    return model.ensure(() => model.previewImport(bytes, options), { signal });
  });
}

/**
 * A summary of document `id`'s comments as its edits leave them. The worker runs it as one call
 * (E5f: not a job), so a cancel only drops the answer: the call stops waiting and the bytes are let go.
 *
 * @throws `missing-glyph` for text no given font draws, `font-not-embedded`, `invalid-font`,
 *   `limit-exceeded`; a save's refusal (an encrypted document's `encrypted-base`).
 */
export function commentSummary(request: CommentSummaryRequest, id: number, signal: AbortSignal): Promise<MadeSummary> {
  return serial(current(id), async () => {
    const { model } = current(id);
    const { bytes } = await model.ensure(() => model.saveCopy({ mode: "auto", invalidateSignatures: true }), {
      signal,
    });
    await loadText();
    signal.throwIfAborted();
    const scratch = Document.open(bytes);
    try {
      const [font, ...fallbacks] = request.fonts.map((face) => Font.fromBytes(face));
      if (font === undefined) throw new Error("a comment summary needs a font");
      const { fonts: _, ...asked } = request;
      const summary = scratch.commentSummary({ ...asked, font, fallbacks });
      try {
        const warnings = summary.warnings.map(({ code, data }) => ({ code, ...(data === undefined ? {} : { data }) }));
        signal.throwIfAborted();
        return { bytes: summary.save(), warnings };
      } finally {
        summary.free();
      }
    } finally {
      scratch.free();
    }
  });
}
