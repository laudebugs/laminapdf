// A comment's kind in the words the summary uses, by its PDF subtype: "Note" for `Text`,
// "Highlight" for `Highlight`. A subtype the summary has no word for shows as the file names it.

import { type LaterMessageId, laterMessage } from "../later-messages.js";

/** The annotation kinds the summary names, by PDF subtype. */
export const SUMMARY_KINDS = [
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
] as const;

/** The name of annotation `subtype`, with a capital: "Sound". */
export function kindName(subtype: string): string {
  return (SUMMARY_KINDS as readonly string[]).includes(subtype)
    ? laterMessage(`summary.kind.${subtype}` as LaterMessageId)
    : subtype;
}
