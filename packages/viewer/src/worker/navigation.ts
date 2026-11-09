// Moving through the open document: its outline and named destinations, read by Lamina PDF in the
// worker (the extract module, which the text layer loads too), each target a page and a view in
// page space. The outline is read when a reader first asks for it, since it can hold thousands of
// items.

import type { OutlineNode, Target } from "@laminapdf/sdk";
import { textModel } from "./document.js";

/** The outline's top-level items, each with its children; empty without one. */
export async function outline(signal: AbortSignal): Promise<OutlineNode[]> {
  const model = await textModel(signal);
  return model.ensure(() => model.outline(), { signal });
}

/** Where the named destination `name` goes, or `undefined` when the document has no such name. */
export async function namedDestination(name: string, signal: AbortSignal): Promise<Target | undefined> {
  const model = await textModel(signal);
  return model.ensure(() => model.namedDestination(name), { signal });
}
