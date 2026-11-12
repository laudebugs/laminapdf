// The words of parts the reader loads after startup: the properties dialog's editing and the
// capability page. They sit in a catalogue of their own so that they load with those parts, not
// with the reader's first paint (#380). Only code in a lazily loaded chunk may import this file;
// the size budget (apps/web/size-budget.json) is what catches a slip. messages.ts holds every
// other word, including the few the apps show from their own lazy code (the undo step's name, the
// page's Back labels), since this catalogue is in no package entry. A translation is a second
// copy of both.

import { type CountIds, countId, ENGLISH, fill, type MessageId } from "./messages.js";

const EN = {
  "capabilities.title": "What Lamina PDF can read, write and round-trip",
  "capabilities.lede":
    "Each operation shows the best status a passing test proves on each part of Lamina PDF: the Rust core, the TypeScript library, the web reader and the desktop app. A status appears only when a test proves it, so this page grows as the work does.",
  "capabilities.status.roundTrip": "Round-trip",
  "capabilities.status.roundTrip.means": "Open, change, save and reopen keeps the feature and everything else.",
  "capabilities.status.write": "Write",
  "capabilities.status.write.means": "Created or changed, and the file passes qpdf, pdf.js and PDFium.",
  "capabilities.status.read": "Read",
  "capabilities.status.read.means": "Shown or extracted faithfully.",
  "capabilities.status.unsupported": "Unsupported",
  "capabilities.status.unsupported.means": "Found and reported, never dropped without a word.",
  "capabilities.status.none": "No evidence yet",
  "capabilities.status.notTargeted": "n/a",
  "capabilities.filters": "Filter operations",
  "capabilities.search": "Search operations",
  "capabilities.group": "Group",
  "capabilities.group.all": "All groups",
  "capabilities.show": "Show",
  "capabilities.surface.core": "Core",
  "capabilities.surface.pdf": "Library",
  "capabilities.surface.web": "Web",
  "capabilities.surface.desktop": "Desktop",
  "capabilities.caption": "{shown} of {total} operations in {groups}",
  "capabilities.groups.one": "1 group",
  "capabilities.groups.many": "{count} groups",
  "capabilities.empty": "No operations match “{query}”",
  "capabilities.clear": "Clear filters",
  "capabilities.column.id": "ID",
  "capabilities.column.operation": "Operation",
  "capabilities.column.target": "Target",
  "capabilities.target": "{status} in {round}",
  "capabilities.provenance":
    "Built from the capability manifest when this version of Lamina PDF was made · {version} ·",
  "capabilities.howTested": "How statuses are tested",
  "properties.tab.description": "Description",
  "properties.tab.file": "File",
  "properties.creator": "Creator",
  "properties.producer": "Producer",
  "properties.edit.authors.help": "Separate several authors with semicolons.",
  "properties.edit.creator.help": "The app the content came from.",
  "properties.edit.producer.help": "The app that wrote the PDF.",
  "properties.edit.keywords.help": "Press Enter or comma to add.",
  "properties.edit.keyword.remove": "Remove keyword {keyword}",
  "properties.edit.keywords.one": "1 keyword",
  "properties.edit.keywords.many": "{count} keywords",
  "properties.edit.modified.help": "Each save sets this to the time it saves.",
  "properties.edit.modified.none": "Not set",
  "properties.edit.willRemove": "Will be removed",
  "properties.edit.keep": "Keep",
  "properties.edit.keep.label": "Keep {field}",
  "properties.edit.removeMetadata": "Remove metadata…",
  "properties.edit.remove.title": "Remove metadata",
  "properties.edit.remove.body":
    "Choose the values to remove. Apply removes them from the document, and Save from the file.",
  "properties.edit.remove.confirm": "Mark for removal",
  "properties.edit.cancel": "Cancel",
  "properties.edit.apply": "Apply",
  "properties.edit.failed": "Lamina PDF couldn't change the properties.",
  "properties.edit.notice.earlier": "Removed values stay in earlier versions.",
  "properties.edit.notice.earlier.body":
    "Save adds your changes after the original, so the old values can still be read from the file. Save as optimised drops them.",
  "properties.edit.notice.pdfa": "{level} file.",
  "properties.edit.notice.pdfa.body":
    "Lamina PDF writes these values to both the Info dictionary and XMP, and keeps the PDF/A identification.",
  "properties.edit.notice.mismatch":
    "The Info dictionary and XMP disagree on {fields}. Apply writes the values shown here to both.",
  "properties.edit.notice.malformed": "This file's XMP metadata can't be read.",
  "properties.edit.notice.malformed.body": "Apply replaces it with new XMP metadata, losing whatever the old one held.",
  "properties.edit.warning.oneAuthor":
    "{level} allows one author. With {count}, the file no longer meets the standard it claims.",
  "properties.edit.warning.title":
    "PDF/UA needs a title. Without one, the file no longer meets the standard it claims.",
} as const;

/** A message only the later parts show. */
type OwnId = keyof typeof EN;

/** A message a later part can show: one of its own, or one of the reader's. */
export type LaterMessageId = MessageId | OwnId;

/** The English later catalogue: the later parts' own messages. */
export const LATER_ENGLISH: { readonly [Id in OwnId]: string } = EN;

/** Both English catalogues, for lookups by `LaterMessageId`. */
const BOTH: Readonly<Record<LaterMessageId, string>> = { ...ENGLISH, ...LATER_ENGLISH };

/** The message `id` from either catalogue, filled in as `message` does. */
export function laterMessage(id: LaterMessageId, values: Readonly<Record<string, string | number>> = {}): string {
  return fill(BOTH[id], values);
}

/** The message for `count` from either catalogue, picking its form as `countMessage` does. */
export function laterCountMessage(
  count: number,
  ids: CountIds<LaterMessageId>,
  values: Readonly<Record<string, string | number>> = {},
): string {
  return laterMessage(countId(count, ids), { count, ...values });
}
