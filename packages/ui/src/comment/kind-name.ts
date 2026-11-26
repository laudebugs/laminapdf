// A comment's kind in the words the summary uses, by its PDF subtype: "Note" for `Text`,
// "Highlight" for `Highlight`. A subtype the summary has no word for shows as the file names it.

import { type LaterMessageId, laterMessage } from "../later-messages.js";

/** The subtypes `summary.kind.*` names. */
const NAMED = new Set([
  "Text",
  "FreeText",
  "Line",
  "Square",
  "Circle",
  "Polygon",
  "PolyLine",
  "Highlight",
  "Underline",
  "Squiggly",
  "StrikeOut",
  "Caret",
  "Stamp",
  "Ink",
  "FileAttachment",
  "Sound",
  "Redact",
]);

/** The name of annotation `subtype`, with a capital: "Sound". */
export function kindName(subtype: string): string {
  return NAMED.has(subtype) ? laterMessage(`summary.kind.${subtype}` as LaterMessageId) : subtype;
}
