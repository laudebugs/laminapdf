// The editable Description tab of the document properties dialog (O04), apart from React: the
// form's values from the document's metadata, what Apply changes (one `setMetadata` patch and one
// `removeMetadata` list), the conformance warnings a field would cause, and the PDF dates the
// date field reads and writes. The metadata is a shape here, as the properties are: @laminapdf/ui
// depends on neither @laminapdf/sdk nor @laminapdf/viewer.

import type { LaterMessageId } from "../later-messages.js";

/** A metadata field, as `@laminapdf/sdk`'s `MetadataKey` names it. */
export type MetadataField =
  | "title"
  | "authors"
  | "subject"
  | "keywords"
  | "creator"
  | "producer"
  | "created"
  | "modified";

/** The fields the form edits: every one but the modification date, which a save sets. */
export type EditableField = Exclude<MetadataField, "modified">;

/** The editable fields, in the form's order (O04: title, author, subject, keywords, created, creator, producer). */
export const EDITABLE_FIELDS: readonly EditableField[] = [
  "title",
  "authors",
  "subject",
  "keywords",
  "created",
  "creator",
  "producer",
];

/** A document's metadata, as `@laminapdf/sdk`'s `doc.metadata()` gives it. */
export interface DocumentMetadataData {
  readonly title?: string | undefined;
  readonly authors: readonly string[];
  readonly subject?: string | undefined;
  readonly keywords?: string | undefined;
  readonly creator?: string | undefined;
  readonly producer?: string | undefined;
  /** PDF dates (`D:20261014102000+01'00'`). */
  readonly created?: string | undefined;
  readonly modified?: string | undefined;
  /** The fields where the Info dictionary and the XMP metadata hold different values. */
  readonly disagreements: readonly MetadataField[];
  /** What the XMP metadata is: none, read, or unreadable (Apply then replaces it). */
  readonly packet: "absent" | "parsed" | "malformed";
  /** The PDF/A part and conformance the file claims, e.g. `"2B"`. */
  readonly pdfa?: string | undefined;
  /** The PDF/UA part the file claims. */
  readonly pdfua?: number | undefined;
}

/** The form's values: text as typed, keywords as chips, the creation date as a `datetime-local` value. */
export interface MetadataForm {
  readonly title: string;
  /** Authors separated by semicolons, as the Info dictionary writes them. */
  readonly authors: string;
  readonly subject: string;
  readonly keywords: readonly string[];
  readonly creator: string;
  readonly producer: string;
  /** `YYYY-MM-DDTHH:mm` in the reader's time zone, or empty. */
  readonly created: string;
  /** The fields marked for removal with Remove metadata…; Keep takes one back. */
  readonly removed: readonly EditableField[];
}

/** What Apply does: the `setMetadata` patch (a value per changed field) and the fields to remove. */
export interface MetadataChange {
  readonly patch: {
    readonly title?: string;
    readonly authors?: readonly string[];
    readonly subject?: string;
    readonly keywords?: string;
    readonly creator?: string;
    readonly producer?: string;
    readonly created?: string;
  };
  readonly remove: readonly EditableField[];
}

/** The authors in `text`: split at semicolons, trimmed, empty ones dropped. */
export function authorsOf(text: string): string[] {
  return text
    .split(";")
    .map((author) => author.trim())
    .filter((author) => author !== "");
}

/** The keywords in `text`: split at commas and semicolons, trimmed, empty ones and repeats dropped. */
export function keywordsOf(text: string): string[] {
  const out: string[] = [];
  for (const word of text.split(/[,;]/)) {
    const keyword = word.trim();
    if (keyword !== "" && !out.includes(keyword)) out.push(keyword);
  }
  return out;
}

const PDF_DATE = /^D:(\d{4})(\d{2})?(\d{2})?(\d{2})?(\d{2})?(\d{2})?(?:([Zz+-])(\d{2})?'?(\d{2})?'?)?$/;

const two = (n: number) => String(n).padStart(2, "0");

/**
 * A PDF date as the instant it names, or undefined when it isn't one. A date without a time zone
 * is read in the reader's own, as viewers do (ISO 32000-2 §7.9.4 leaves it unknown).
 */
export function pdfDateInstant(date: string): Date | undefined {
  const m = PDF_DATE.exec(date.trim());
  if (m === null) return undefined;
  const [y, mo = "01", d = "01", h = "00", mi = "00", s = "00"] = m.slice(1, 7) as [string, ...(string | undefined)[]];
  const sign = m[7];
  const fields = [Number(y), Number(mo) - 1, Number(d), Number(h), Number(mi), Number(s)] as const;
  if (sign === undefined) return new Date(...fields);
  const offset =
    sign === "Z" || sign === "z" ? 0 : (sign === "-" ? -1 : 1) * (Number(m[8] ?? 0) * 60 + Number(m[9] ?? 0));
  return new Date(Date.UTC(...fields) - offset * 60_000);
}

/** A PDF date in the reader's locale (`14 Oct 2026, 10:20`); the text as written when it isn't one. */
export function formatPdfDate(date: string, locale?: string): string {
  const instant = pdfDateInstant(date);
  if (instant === undefined) return date;
  return new Intl.DateTimeFormat(locale, { dateStyle: "medium", timeStyle: "short" }).format(instant);
}

/** A PDF date as a `datetime-local` value in the reader's time zone; empty when it isn't one. */
export function pdfDateToLocalInput(date: string | undefined): string {
  const instant = date === undefined ? undefined : pdfDateInstant(date);
  if (instant === undefined) return "";
  return `${instant.getFullYear()}-${two(instant.getMonth() + 1)}-${two(instant.getDate())}T${two(instant.getHours())}:${two(instant.getMinutes())}`;
}

/** A `datetime-local` value as a PDF date with the reader's time zone at that moment; undefined when it isn't one. */
export function localInputToPdfDate(value: string): string | undefined {
  const m = /^(\d{4})-(\d{2})-(\d{2})T(\d{2}):(\d{2})(?::(\d{2}))?$/.exec(value);
  if (m === null) return undefined;
  const [y, mo, d, h, mi, s = "00"] = m.slice(1) as [string, string, string, string, string, string | undefined];
  const instant = new Date(Number(y), Number(mo) - 1, Number(d), Number(h), Number(mi), Number(s));
  if (Number.isNaN(instant.getTime())) return undefined;
  // getTimezoneOffset is minutes behind UTC: west is positive.
  const east = -instant.getTimezoneOffset();
  const zone =
    east === 0 ? "Z" : `${east < 0 ? "-" : "+"}${two(Math.floor(Math.abs(east) / 60))}'${two(Math.abs(east) % 60)}'`;
  return `D:${y}${mo}${d}${h}${mi}${s}${zone}`;
}

/** The form for `metadata`: its values as they are, nothing removed. */
export function formFrom(metadata: DocumentMetadataData): MetadataForm {
  return {
    title: metadata.title ?? "",
    authors: metadata.authors.join("; "),
    subject: metadata.subject ?? "",
    keywords: keywordsOf(metadata.keywords ?? ""),
    creator: metadata.creator ?? "",
    producer: metadata.producer ?? "",
    created: pdfDateToLocalInput(metadata.created),
    removed: [],
  };
}

/** Whether `field` has a value in `metadata`: what Remove metadata… lists. */
export function hasValue(metadata: DocumentMetadataData, field: EditableField): boolean {
  return field === "authors" ? metadata.authors.length > 0 : (metadata[field] ?? "") !== "";
}

/** Field `field`'s value in the form, as `setMetadata` takes it; undefined when the form leaves it empty. */
function formValue(form: MetadataForm, field: EditableField): string | readonly string[] | undefined {
  switch (field) {
    case "authors": {
      const authors = authorsOf(form.authors);
      return authors.length === 0 ? undefined : authors;
    }
    case "keywords":
      return form.keywords.length === 0 ? undefined : form.keywords.join(", ");
    case "created":
      return form.created === "" ? undefined : localInputToPdfDate(form.created);
    default: {
      const text = form[field].trim();
      return text === "" ? undefined : text;
    }
  }
}

/** Whether the form's `value` for `field` is the one `metadata` holds (dates compared as instants). */
function same(metadata: DocumentMetadataData, field: EditableField, value: string | readonly string[]): boolean {
  if (field === "authors") {
    const authors = value as readonly string[];
    return authors.length === metadata.authors.length && authors.every((a, i) => a === metadata.authors[i]);
  }
  if (field === "keywords") return (value as string) === keywordsOf(metadata.keywords ?? "").join(", ");
  if (field === "created") {
    const before = metadata.created === undefined ? undefined : pdfDateInstant(metadata.created);
    const after = pdfDateInstant(value as string);
    // Minutes: the field shows no seconds, so an untouched date keeps its own.
    return (
      before !== undefined &&
      after !== undefined &&
      Math.floor(before.getTime() / 60_000) === Math.floor(after.getTime() / 60_000)
    );
  }
  return value === (metadata[field] ?? "");
}

/**
 * What Apply changes, or undefined when it changes nothing. Only the fields the reader touched
 * change: one left as the dialog showed it is left alone, so a value the form can't show exactly
 * (a date it can't read, spaces around a title) stays as the file has it. A touched field emptied,
 * or one marked for removal, is removed; a touched one with a new value is set. In a PDF/A file, a
 * field where Info and XMP disagree is set to the value shown even when untouched, so Apply brings
 * the two back in step.
 */
export function changeOf(metadata: DocumentMetadataData, form: MetadataForm): MetadataChange | undefined {
  const start = formFrom(metadata);
  const patch: Record<string, string | readonly string[]> = {};
  const remove: EditableField[] = [];
  for (const field of EDITABLE_FIELDS) {
    const change = fieldChange(metadata, form, start, field);
    if (change === "remove") remove.push(field);
    else if (change !== undefined) patch[field] = change.value;
  }
  return Object.keys(patch).length === 0 && remove.length === 0 ? undefined : { patch, remove };
}

/** What Apply does to one field: remove it, set it to a value, or nothing (undefined). */
function fieldChange(
  metadata: DocumentMetadataData,
  form: MetadataForm,
  start: MetadataForm,
  field: EditableField,
): "remove" | { readonly value: string | readonly string[] } | undefined {
  const held = hasValue(metadata, field);
  if (form.removed.includes(field)) return held ? "remove" : undefined;
  const value = formValue(form, field);
  const touched = JSON.stringify(form[field]) !== JSON.stringify(start[field]);
  if (value === undefined) return touched && held ? "remove" : undefined;
  const outOfStep = metadata.pdfa !== undefined && metadata.disagreements.includes(field);
  return outOfStep || (touched && !same(metadata, field, value)) ? { value } : undefined;
}

/** A warning under a field: the message and its values. */
export interface FieldWarning {
  readonly id: LaterMessageId;
  readonly values?: Readonly<Record<string, string | number>>;
}

/**
 * The claims Apply would break, under the field that breaks them (the save planner's
 * `pdfa-one-author` and `pdfua-title-missing`, laminapdf-ops' `conformance_warnings`; the reader's
 * e2e/properties-warnings.e2e.test.ts holds them to the SDK's own on the same edits).
 */
export function fieldWarnings(
  metadata: DocumentMetadataData,
  form: MetadataForm,
): Partial<Record<EditableField, FieldWarning>> {
  const out: Partial<Record<EditableField, FieldWarning>> = {};
  const authors = form.removed.includes("authors") ? 0 : authorsOf(form.authors).length;
  if (metadata.pdfa?.startsWith("1") === true && authors > 1) {
    out.authors = { id: "properties.edit.warning.oneAuthor", values: { count: authors, level: metadata.pdfa } };
  }
  const title = form.removed.includes("title") ? "" : form.title.trim();
  if (metadata.pdfua === 1 && title === "") out.title = { id: "properties.edit.warning.title" };
  return out;
}

/**
 * Whether Apply removes a value that a save keeps in the file's earlier versions: a removal, saved
 * incrementally (anything but a save that rewrites the whole file).
 */
export function removalStaysInEarlierVersions(change: MetadataChange | undefined, rewritesWhole: boolean): boolean {
  return change !== undefined && change.remove.length > 0 && !rewritesWhole;
}
