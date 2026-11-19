// Each page's annotation records, read by Lamina PDF in the worker (the extract module, as the
// links are), for the viewer's annotation layer. The last pages' records are kept, since a page
// scrolled back into view asks again, until an edit changes the page (`forgetAnnotations`): the
// records are those of the page as it is now.

import type { PageAnnotation, TextBox } from "@laminapdf/sdk";
import { textModel } from "./document.js";
import { CACHED_PAGES, RecentPages } from "./recent-pages.js";

/** The last pages' annotation records. */
const records = new RecentPages<readonly PageAnnotation[]>(CACHED_PAGES);

/** Page `index`'s annotations, in the order of its `/Annots` (cached). */
export async function pageAnnotations(index: number, signal: AbortSignal): Promise<readonly PageAnnotation[]> {
  const cached = records.get(index);
  if (cached !== undefined) return cached;
  const model = await textModel(signal);
  const read = await model.ensure(() => model.pageAnnotations(index), { signal });
  records.set(index, read);
  return read;
}

/** The text box `id` read back as it is now (never cached: an edit may have changed it). */
export async function textBox(id: string, signal: AbortSignal): Promise<TextBox | undefined> {
  const model = await textModel(signal);
  return model.ensure(() => model.textBox(id), { signal });
}

/** Sound comment `id`'s recording as a WAV file, which every browser's `<audio>` plays. */
export async function soundAsWav(id: string, signal: AbortSignal): Promise<Uint8Array> {
  const model = await textModel(signal);
  return model.ensure(() => model.soundAsWav(id), { signal });
}

/** Forgets `pages`' annotations (an edit changed them), or every page's (the document closed). */
export function forgetAnnotations(pages?: Iterable<number>): void {
  records.forget(pages);
}
