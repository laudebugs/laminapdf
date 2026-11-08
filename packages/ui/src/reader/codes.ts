// The reader's plain sentence for each repair warning and each error that stops a document opening
// (ADR 0003, codes in fixtures/codes.json; R04). Content-stream and extraction warnings share one
// sentence per family; an error that isn't about the file (a network or memory failure) says to
// try again; a code this table doesn't know reads as a generic repair or failure, so a new code
// never shows a raw identifier in place of words. Sentences live under their own keys, apart from
// the panel's labels, so no code can name a label.

import { ENGLISH, type MessageId, message } from "../messages.js";

/** The families whose codes share one sentence: drawing instructions, and text extraction. */
const FAMILIES: readonly [pattern: RegExp, id: MessageId][] = [
  [/^content-|^inline-image-|^xobject-/, "repairCode.content"],
  [/^extract-/, "repairCode.extract"],
];

/** Errors about the session rather than the file: trying again may work. */
const TRANSIENT = new Set([
  "fetch-failed",
  "out-of-memory",
  "wasm-trap",
  "worker-crashed",
  "instance-reset",
  "render-backend-unavailable",
  "unknown",
]);

/** The message for repair warning `code`, its own or its family's; undefined when it has neither. */
function repairKey(code: string): MessageId | undefined {
  const own = `repairCode.${code}`;
  if (own in ENGLISH) return own as MessageId;
  return FAMILIES.find(([pattern]) => pattern.test(code))?.[1];
}

/** The sentence for repair warning `code`. */
export function repairSentence(code: string): string {
  return message(repairKey(code) ?? "repairCode.other");
}

/** Whether `code` has a sentence of its own or its family's, rather than the generic one. */
export function hasRepairSentence(code: string): boolean {
  return repairKey(code) !== undefined;
}

/** The sentence for error `code` when a document can't open. */
export function failureSentence(code: string): string {
  const own = `failureCode.${code}`;
  if (own in ENGLISH) return message(own as MessageId);
  return message(TRANSIENT.has(code) ? "failureCode.retry" : "failureCode.other");
}
