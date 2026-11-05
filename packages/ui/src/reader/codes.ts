// The reader's plain sentence for each repair warning and each error that stops a document opening
// (ADR 0003, codes in fixtures/codes.json; R04). Content-stream and extraction warnings share one
// sentence per family; a code this table doesn't know reads as a generic repair or failure, so a
// new code never shows a raw identifier in place of words.

import { ENGLISH, type MessageId, message } from "../messages.js";

/** The families whose codes share one sentence: drawing instructions, and text extraction. */
const FAMILIES: readonly [pattern: RegExp, id: MessageId][] = [
  [/^content-|^inline-image-|^xobject-/, "repair.content"],
  [/^extract-/, "repair.extract"],
];

/** The sentence for repair warning `code`. */
export function repairSentence(code: string): string {
  const own = `repair.${code}`;
  if (own in ENGLISH) return message(own as MessageId);
  const family = FAMILIES.find(([pattern]) => pattern.test(code));
  return message(family?.[1] ?? "repair.other");
}

/** The sentence for error `code` when a document can't open. */
export function failureSentence(code: string): string {
  const own = `failure.${code}`;
  return message(own in ENGLISH ? (own as MessageId) : "failure.other");
}

/** Whether `code` has a sentence of its own or its family's, rather than the generic one. */
export function hasRepairSentence(code: string): boolean {
  return `repair.${code}` in ENGLISH || FAMILIES.some(([pattern]) => pattern.test(code));
}
