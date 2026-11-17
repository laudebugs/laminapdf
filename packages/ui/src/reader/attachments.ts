// What the Attachments panel (O14) says about a file, worked out from what the document states:
// the kind of file for people ("Spreadsheet"), whether it is a program (which gets O14's warning
// before it is saved), the badge naming its relationship and what it belongs to ("Source of page
// 3"), and the header's count and total size. Lamina PDF never opens an attached file; these read
// only its name and MIME type.

import { laterMessage } from "../later-messages.js";
import { formatBytes } from "./bytes.js";

/** How an attached file relates to what it belongs to, as the attachment edits name it. */
export type AttachmentRelationshipData =
  | "source"
  | "data"
  | "alternative"
  | "supplement"
  | "encryptedPayload"
  | "formData"
  | "schema"
  | "unspecified";

/** What holds an attached file, as `@laminapdf/sdk`'s `attachments()` lists it. */
export interface AttachmentHolderData {
  readonly kind: "embeddedFiles" | "document" | "page" | "annotation" | "fileAttachment";
  /** The page, 0-based, for a page, an annotation or a file attachment annotation. */
  readonly page?: number;
  readonly annotation?: string;
}

/** One attached file, as `@laminapdf/sdk`'s `attachments()` lists it. */
export interface AttachmentData {
  readonly id: string;
  readonly key?: string;
  readonly name?: string;
  readonly description?: string;
  readonly mime?: string;
  readonly size?: number;
  /** PDF dates (`D:20261027091200Z`). */
  readonly created?: string;
  readonly modified?: string;
  readonly hasChecksum: boolean;
  readonly relationship?: AttachmentRelationshipData;
  readonly embedded: boolean;
  readonly holders: readonly AttachmentHolderData[];
}

/** What an attached file belongs to, besides the document's list. */
export type AttachmentOwner =
  | { readonly kind: "document" }
  | { readonly kind: "page"; readonly page: number }
  | { readonly kind: "annotation"; readonly page: number };

/** The relationships the Organise select offers, in O14's order. */
export const EDITABLE_RELATIONSHIPS: readonly AttachmentRelationshipData[] = [
  "source",
  "data",
  "alternative",
  "supplement",
  "unspecified",
];

/** File name endings of programs and scripts, which O14 warns about before saving one. */
const PROGRAM_ENDINGS = new Set([
  "app",
  "apk",
  "bat",
  "bin",
  "cmd",
  "com",
  "cpl",
  "dll",
  "dmg",
  "exe",
  "hta",
  "jar",
  "js",
  "jse",
  "lnk",
  "msi",
  "pkg",
  "ps1",
  "py",
  "scr",
  "sh",
  "vbe",
  "vbs",
  "wsf",
]);

/** MIME types of programs and scripts. */
const PROGRAM_TYPES = [
  "application/x-msdownload",
  "application/x-msdos-program",
  "application/x-executable",
  "application/x-sh",
  "application/x-bat",
  "application/javascript",
  "text/javascript",
  "application/vnd.microsoft.portable-executable",
  "application/java-archive",
];

/** The file's name for people: its name, else its key, else "Untitled file". */
export function attachmentName(file: AttachmentData): string {
  const name = (file.name ?? file.key)?.trim();
  return name === undefined || name === "" ? laterMessage("attachments.untitled") : name;
}

/** The part of `name` after its last dot, lower case; empty without one. */
function ending(name: string): string {
  const dot = name.lastIndexOf(".");
  return dot < 0 ? "" : name.slice(dot + 1).toLowerCase();
}

/** Whether the file is a program or a script, by its name's ending or its MIME type. */
export function isProgram(file: AttachmentData): boolean {
  const mime = file.mime?.toLowerCase();
  return PROGRAM_ENDINGS.has(ending(attachmentName(file))) || (mime !== undefined && PROGRAM_TYPES.includes(mime));
}

/** The kind of file for people: "PDF", "Spreadsheet", "Data", "Image", "Text", "Program" or "File". */
export function attachmentKind(file: AttachmentData): string {
  if (isProgram(file)) return laterMessage("attachments.kind.program");
  const mime = file.mime?.toLowerCase() ?? "";
  const end = ending(attachmentName(file));
  if (mime === "application/pdf" || end === "pdf") return laterMessage("attachments.kind.pdf");
  if (mime.includes("spreadsheet") || mime.includes("excel") || ["xlsx", "xls", "ods", "numbers"].includes(end)) {
    return laterMessage("attachments.kind.spreadsheet");
  }
  if (mime.includes("xml") || mime.includes("json") || mime === "text/csv" || ["xml", "json", "csv"].includes(end)) {
    return laterMessage("attachments.kind.data");
  }
  if (mime.startsWith("image/")) return laterMessage("attachments.kind.image");
  if (mime.startsWith("text/") || end === "txt") return laterMessage("attachments.kind.text");
  return laterMessage("attachments.kind.file");
}

/** What the file belongs to besides the document's list: the first page, annotation or the document. */
export function attachmentOwner(file: AttachmentData): AttachmentOwner | undefined {
  for (const holder of file.holders) {
    if (holder.kind === "page" && holder.page !== undefined) return { kind: "page", page: holder.page };
    if (holder.kind === "annotation" && holder.page !== undefined) return { kind: "annotation", page: holder.page };
    if (holder.kind === "document") return { kind: "document" };
  }
  return undefined;
}

/** The page a file attachment annotation holding the file is on, if one does. */
export function annotationPage(file: AttachmentData): number | undefined {
  return file.holders.find((h) => h.kind === "fileAttachment")?.page;
}

/** The relationship's name: "Source", "Data", … */
export function relationshipName(relationship: AttachmentRelationshipData): string {
  return laterMessage(`attachments.relationship.${relationship}`);
}

/** The relationship's one-line meaning: "The file this page was made from." */
export function relationshipMeaning(
  relationship: AttachmentRelationshipData,
  owner: AttachmentOwner | undefined,
): string {
  const where = owner?.kind === "page" ? "page" : owner?.kind === "annotation" ? "annotation" : "document";
  return laterMessage(`attachments.meaning.${relationship}`, { where: laterMessage(`attachments.where.${where}`) });
}

/**
 * The neutral badge for an associated file: "Source of page 3", "Data for the document"; none for a
 * file only in the document's list, or whose relationship is unspecified.
 */
export function relationshipBadge(file: AttachmentData, pageName: (index: number) => string): string | undefined {
  const owner = attachmentOwner(file);
  const relationship = file.relationship;
  if (owner === undefined || relationship === undefined || relationship === "unspecified") return undefined;
  const target =
    owner.kind === "document"
      ? laterMessage("attachments.target.document")
      : laterMessage("attachments.target.page", { page: pageName(owner.page) });
  return laterMessage(`attachments.badge.${relationship}`, { target });
}

/** "3 files · 1.3 MB": the header's count and total size of the files that state one. */
export function attachmentsSummary(files: readonly AttachmentData[]): string {
  const total = files.reduce((sum, f) => sum + (f.size ?? 0), 0);
  const count =
    files.length === 1
      ? laterMessage("attachments.count.one")
      : laterMessage("attachments.count", { count: files.length });
  return laterMessage("attachments.summary", { count, size: formatBytes(total) });
}

/** "86 KB · Spreadsheet": a row's second line. */
export function attachmentLine(file: AttachmentData): string {
  const kind = attachmentKind(file);
  return file.size === undefined ? kind : laterMessage("attachments.line", { size: formatBytes(file.size), kind });
}

/** MIME types by file name ending, for a file the system gives no type (PDF/A-3 asks for one). */
const TYPES: Readonly<Record<string, string>> = {
  csv: "text/csv",
  json: "application/json",
  jpeg: "image/jpeg",
  jpg: "image/jpeg",
  pdf: "application/pdf",
  png: "image/png",
  txt: "text/plain",
  xlsx: "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet",
  xml: "text/xml",
};

/** `type` when the system gave one, else the MIME type `name`'s ending stands for, else a plain byte stream. */
export function mimeOf(name: string, type: string): string {
  return type !== "" ? type : (TYPES[ending(name)] ?? "application/octet-stream");
}

/** A PDF date (`D:20261027091200Z`) as a `Date`; undefined when it doesn't read as one. */
export function pdfDate(date: string | undefined): Date | undefined {
  const m = /^D:(\d{4})(\d{2})?(\d{2})?(\d{2})?(\d{2})?(\d{2})?(Z|[+-]\d{2}'?\d{2}'?)?/.exec(date ?? "");
  if (m === null) return undefined;
  const [, y, mo = "01", d = "01", h = "00", mi = "00", s = "00", zone] = m;
  const offset =
    zone === undefined || zone === "Z" ? "Z" : `${zone.slice(0, 3)}:${zone.replaceAll("'", "").slice(3, 5) || "00"}`;
  const parsed = new Date(`${y}-${mo}-${d}T${h}:${mi}:${s}${offset}`);
  return Number.isNaN(parsed.getTime()) ? undefined : parsed;
}

/** A `Date` as the PDF date the attachment edits take (`D:20261117093000Z`, in UTC). */
export function toPdfDate(at: Date): string {
  const pad = (n: number) => String(n).padStart(2, "0");
  return `D:${at.getUTCFullYear()}${pad(at.getUTCMonth() + 1)}${pad(at.getUTCDate())}${pad(at.getUTCHours())}${pad(at.getUTCMinutes())}${pad(at.getUTCSeconds())}Z`;
}

/** An accessible row name: "repairs-by-region.xlsx, 86 KB, spreadsheet, source of page 3". */
export function attachmentRowName(file: AttachmentData, pageName: (index: number) => string): string {
  const parts = [attachmentName(file)];
  if (file.size !== undefined) parts.push(formatBytes(file.size));
  parts.push(attachmentKind(file).toLowerCase());
  const badge = relationshipBadge(file, pageName);
  if (badge !== undefined) parts.push(badge.toLowerCase());
  return parts.join(", ");
}
