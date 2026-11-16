// New files made from pages (the E2 plan §2.1): Extract and Export ranges take pages of the open
// document as its edits leave them, Split cuts it into parts, and Merge into a new document joins
// whole files. Each part is a new document, saved as a whole file; the open document doesn't
// change. Where the page can't write a folder, the parts come back as one zip (zip.ts), made here
// so the page never holds them twice.

import { Document } from "@laminapdf/sdk";
import type { CallContext } from "@laminapdf/sdk/worker/server";
import type { MergeFilesRequest, PartsOutput, WrittenParts } from "../protocol.js";
import { current } from "./document.js";
import { serial } from "./edit.js";
import { zip } from "./zip.js";

/**
 * One new file per list of `ranges` (page IDs of document `id`, each list in the order given),
 * named by `output`: separate files, or one zip of them. Reports `{ done, total }` after each.
 *
 * @throws `unknown-page` for a page the document doesn't have; reading errors.
 */
export function writeParts(
  ranges: readonly (readonly string[])[],
  output: PartsOutput,
  id: number,
  context: CallContext,
): Promise<WrittenParts> {
  const opened = current(id);
  return serial(opened, async () => {
    const { model } = opened;
    const { signal } = context;
    const files: Uint8Array[] = [];
    for (const pages of ranges) {
      signal.throwIfAborted();
      const part = await model.ensure(() => model.extractPages(pages), { signal });
      try {
        files.push(part.save());
      } finally {
        part.free();
      }
      context.progress({ done: files.length, total: ranges.length });
    }
    return packed(files, output);
  });
}

/**
 * One new document of `request.files` one after another, with the open document `request.open`
 * first when it names one (its pages as its edits leave them), merged as `request.options` say.
 *
 * @throws `password-required` or `wrong-password` for a file whose password is missing; the
 *   errors of a file that doesn't open.
 */
export async function mergeFiles(request: MergeFilesRequest, context: CallContext): Promise<Uint8Array> {
  const inputs: { bytes: Uint8Array; password?: string; title?: string }[] = [];
  if (request.open !== undefined) {
    const opened = current(request.open.id);
    const whole = await serial(opened, async () => {
      const { model } = opened;
      const pages = await model.ensure(() => model.pages, { signal: context.signal });
      const copy = await model.ensure(() => model.extractPages(pages.map((page) => page.id)), {
        signal: context.signal,
      });
      try {
        return copy.save();
      } finally {
        copy.free();
      }
    });
    inputs.push({ bytes: whole, title: request.open.title });
  }
  for (const file of request.files) inputs.push({ ...file });
  context.signal.throwIfAborted();
  const merged = Document.merge(inputs, request.options);
  try {
    return merged.save();
  } finally {
    merged.free();
  }
}

/** `files` as `output` asks: as they are, or zipped under its names. */
function packed(files: Uint8Array[], output: PartsOutput): WrittenParts {
  if (output.zip === undefined) return { files };
  const { names, modified } = output.zip;
  const entries = files.map((bytes, i) => ({ name: names[i] ?? `part ${i + 1}.pdf`, bytes }));
  return { zip: zip(entries, new Date(modified)) };
}
