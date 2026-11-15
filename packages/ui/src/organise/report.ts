// What the Organise mode's toasts say after an edit (Paper O06): how many pages a delete took and
// what it did to the links and bookmarks that pointed at them, what a duplicate did to form fields,
// how many blank pages went in. Each is the whole toast's text; the toast adds Undo.

import { laterCountMessage } from "../later-messages.js";

/** What a deletion did to one reference to a deleted page (the edit result's `references`). */
interface ReferenceOutcome {
  readonly outcome: "retargeted" | "removed";
}

/** What a duplicate did with one form field (the edit result's `fields`). */
interface FieldOutcome {
  readonly outcome: "renamed" | "linked" | "notCopied";
}

/** "Deleted 2 pages. 1 link or bookmark now points to the nearest page that stayed." */
export function deletedMessage(count: number, references: readonly ReferenceOutcome[] = []): string {
  const retargeted = references.filter((r) => r.outcome === "retargeted").length;
  const removed = references.length - retargeted;
  return [
    laterCountMessage(count, { one: "organise.deleted.one", many: "organise.deleted" }),
    retargeted === 0
      ? ""
      : laterCountMessage(retargeted, { one: "organise.deleted.retargeted.one", many: "organise.deleted.retargeted" }),
    removed === 0
      ? ""
      : laterCountMessage(removed, { one: "organise.deleted.removed.one", many: "organise.deleted.removed" }),
  ]
    .filter((part) => part !== "")
    .join(" ");
}

/** "Duplicated 1 page. Its field was copied as a new field. The signature field wasn't copied." */
export function duplicatedMessage(count: number, fields: readonly FieldOutcome[] = []): string {
  const skipped = fields.filter((f) => f.outcome === "notCopied").length;
  const copied = fields.length - skipped;
  return [
    laterCountMessage(count, { one: "organise.duplicated.one", many: "organise.duplicated" }),
    copied === 0
      ? ""
      : laterCountMessage(copied, { one: "organise.duplicated.fields.one", many: "organise.duplicated.fields" }),
    skipped === 0
      ? ""
      : laterCountMessage(skipped, { one: "organise.duplicated.signature", many: "organise.duplicated.signature" }),
  ]
    .filter((part) => part !== "")
    .join(" ");
}

/** "Inserted 3 blank pages." */
export function insertedMessage(count: number): string {
  return laterCountMessage(count, { one: "organise.inserted.one", many: "organise.inserted" });
}
