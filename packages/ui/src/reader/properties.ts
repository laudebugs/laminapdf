// What the document properties dialog (R08) shows, worked out from the document's properties
// apart from React: the XMP packet's few fields, merged with the Info entries (XMP wins when they
// differ), dates in the reader's locale, the page sizes by paper name, the file size, and the
// lines that Copy all writes. The properties are a shape here, as the viewer is: @laminapdf/ui
// depends on neither @laminapdf/sdk nor @laminapdf/viewer.

import { message } from "../messages.js";

/** What the author permits (`/P`): a request to readers, not a lock. */
export interface PropertiesPermissions {
  readonly print: boolean;
  readonly printHighQuality: boolean;
  readonly modify: boolean;
  readonly copy: boolean;
  readonly annotate: boolean;
  readonly fillForms: boolean;
  readonly accessibility: boolean;
  readonly assemble: boolean;
}

/** One font the pages use. */
export interface PropertiesFont {
  readonly name?: string | undefined;
  readonly kind?: string | undefined;
  readonly embedded: boolean;
  readonly subset: boolean;
}

/** A document's properties, as `@laminapdf/sdk`'s `doc.properties()` gives them. */
export interface DocumentPropertiesData {
  readonly title?: string | undefined;
  readonly author?: string | undefined;
  readonly subject?: string | undefined;
  readonly keywords?: string | undefined;
  readonly creator?: string | undefined;
  readonly producer?: string | undefined;
  /** ISO 8601. */
  readonly created?: string | undefined;
  readonly modified?: string | undefined;
  /** The XMP packet's text. */
  readonly xmp?: string | undefined;
  readonly version?: string | undefined;
  readonly pageCount: number;
  readonly fileSize: number;
  readonly linearized: boolean;
  readonly tagged: boolean;
  readonly encryption?:
    | {
        readonly revision: number;
        readonly keyBits: number;
        readonly cipher: string;
        readonly permissions?: PropertiesPermissions | undefined;
      }
    | undefined;
  readonly fonts: readonly PropertiesFont[];
  readonly signatures: number;
  readonly contains: { readonly javascript: boolean; readonly attachments: number; readonly forms: boolean };
}

/** The XMP fields the dialog shows (§14.3.2; Dublin Core, XMP basic, Adobe PDF schemas). */
export interface XmpFields {
  title?: string;
  author?: string;
  subject?: string;
  keywords?: string;
  creator?: string;
  producer?: string;
  created?: string;
  modified?: string;
}

const NS = {
  dc: "http://purl.org/dc/elements/1.1/",
  xmp: "http://ns.adobe.com/xap/1.0/",
  pdf: "http://ns.adobe.com/pdf/1.3/",
  rdf: "http://www.w3.org/1999/02/22-rdf-syntax-ns#",
} as const;

/** How XMP text becomes a document: `DOMParser`, which never loads external entities or DTDs. */
export type XmlParser = (text: string) => Document;

/** The browser's parser, as XML. */
export const domXmlParser: XmlParser = (text) => new DOMParser().parseFromString(text, "application/xml");

/**
 * The fields of an XMP packet the dialog shows: the first `rdf:li` of a language alternative or a
 * list (`dc:title`, `dc:creator`), else the element's text, else the attribute form
 * (`<rdf:Description pdf:Producer="…">`). An unreadable packet gives nothing.
 */
export function readXmp(text: string, parse: XmlParser = domXmlParser): XmpFields {
  let doc: Document;
  try {
    doc = parse(text);
  } catch {
    return {};
  }
  if (doc.getElementsByTagName("parsererror").length > 0) return {};
  const read = (ns: string, name: string): string | undefined => {
    const element = doc.getElementsByTagNameNS(ns, name)[0];
    if (element !== undefined) {
      const item = element.getElementsByTagNameNS(NS.rdf, "li")[0];
      const value = (item ?? element).textContent?.trim();
      if (value !== undefined && value !== "") return value;
    }
    for (const description of Array.from(doc.getElementsByTagNameNS(NS.rdf, "Description"))) {
      const value = description.getAttributeNS(ns, name)?.trim();
      if (value) return value;
    }
    return undefined;
  };
  const fields: XmpFields = {};
  const set = (key: keyof XmpFields, value: string | undefined) => {
    if (value !== undefined) fields[key] = value;
  };
  set("title", read(NS.dc, "title"));
  set("author", read(NS.dc, "creator"));
  set("subject", read(NS.dc, "description"));
  set("keywords", read(NS.pdf, "Keywords"));
  set("creator", read(NS.xmp, "CreatorTool"));
  set("producer", read(NS.pdf, "Producer"));
  set("created", read(NS.xmp, "CreateDate"));
  set("modified", read(NS.xmp, "ModifyDate"));
  return fields;
}

/** A date for the reader's locale (`14 Oct 2026, 09:12`); the text as written when it isn't a date. */
export function formatDate(iso: string, locale?: string): string {
  // `Date` reads a date-time without an offset as local time, which shows it as written; a date
  // alone it reads as UTC, which would move it a day west of Greenwich, so it gets a local midnight.
  const dateOnly = /^\d{4}(-\d\d){0,2}$/.test(iso);
  const time = new Date(dateOnly ? `${iso.padEnd(10, "-01").slice(0, 10)}T00:00:00` : iso);
  if (Number.isNaN(time.getTime())) return iso;
  const style: Intl.DateTimeFormatOptions = dateOnly
    ? { dateStyle: "medium" }
    : { dateStyle: "medium", timeStyle: "short" };
  return new Intl.DateTimeFormat(locale, style).format(time);
}

/** A file size: `1.2 MB (1,262,480 bytes)`. */
export function formatFileSize(bytes: number, locale?: string): string {
  const exact = new Intl.NumberFormat(locale).format(bytes);
  const units = ["KB", "MB", "GB"] as const;
  let value = bytes;
  let unit: string | undefined;
  for (const next of units) {
    if (value < 1000) break;
    value /= 1000;
    unit = next;
  }
  if (unit === undefined) return message("properties.size.bytes", { bytes: exact });
  const short = new Intl.NumberFormat(locale, { maximumFractionDigits: 1 }).format(value);
  return message("properties.size", { size: `${short} ${unit}`, bytes: exact });
}

/** Paper sizes in points, portrait, by the name the dialog gives them. */
const PAPERS: readonly (readonly [string, number, number])[] = [
  ["US Letter", 612, 792],
  ["US Legal", 612, 1008],
  ["Tabloid", 792, 1224],
  ["A3", 841.89, 1190.55],
  ["A4", 595.28, 841.89],
  ["A5", 419.53, 595.28],
];

/** A page size's name: a paper (`A4`, `US Letter`, either way up), else its size in inches or millimetres. */
export function paperName(width: number, height: number): string {
  const [short, long] = width <= height ? [width, height] : [height, width];
  const paper = PAPERS.find(([, w, h]) => Math.abs(w - short) < 2 && Math.abs(h - long) < 2);
  if (paper !== undefined) return paper[0];
  const inches = (pt: number) => Math.round((pt / 72) * 100) / 100;
  return `${inches(width)} × ${inches(height)} in`;
}

/** The page sizes: `A4 · all pages`, or `Mixed: 22 US Letter, 2 Tabloid`, most common first. */
export function pageSizeSummary(sizes: readonly { readonly width: number; readonly height: number }[]): string {
  const counts = new Map<string, number>();
  for (const { width, height } of sizes) {
    const name = paperName(width, height);
    counts.set(name, (counts.get(name) ?? 0) + 1);
  }
  const entries = [...counts].sort((a, b) => b[1] - a[1]);
  const first = entries[0];
  if (first === undefined) return "";
  if (entries.length === 1) return message("properties.pageSize.all", { size: first[0] });
  return message("properties.pageSize.mixed", { sizes: entries.map(([name, n]) => `${n} ${name}`).join(", ") });
}

/** One row of the dialog: its label and value. */
export interface PropertyRow {
  readonly label: string;
  readonly value: string;
}

/** One section: its title, and its rows. */
export interface PropertySection {
  readonly title: string;
  readonly rows: readonly PropertyRow[];
}

/** What the dialog is given besides the properties: the pages' sizes and labels. */
export interface PropertiesContext {
  readonly pages: readonly { readonly width: number; readonly height: number }[];
  /** Each page's label, when the document labels its pages. */
  readonly labels?: readonly string[] | undefined;
  readonly locale?: string | undefined;
}

const yes = (flag: boolean) => message(flag ? "properties.yes" : "properties.no");

function description(p: DocumentPropertiesData, locale: string | undefined): PropertyRow[] {
  const xmp = p.xmp === undefined ? {} : readXmp(p.xmp);
  const pick = (key: keyof XmpFields & keyof DocumentPropertiesData) => xmp[key] ?? p[key];
  const rows: PropertyRow[] = [];
  const add = (id: Parameters<typeof message>[0], value: string | undefined) => {
    if (value !== undefined && value !== "") rows.push({ label: message(id), value });
  };
  add("properties.title", pick("title"));
  add("properties.author", pick("author"));
  add("properties.subject", pick("subject"));
  add("properties.keywords", pick("keywords"));
  const created = pick("created");
  const modified = pick("modified");
  add("properties.created", created === undefined ? undefined : formatDate(created, locale));
  add("properties.modified", modified === undefined ? undefined : formatDate(modified, locale));
  const creator = pick("creator");
  const producer = pick("producer");
  add(
    "properties.madeWith",
    creator !== undefined && producer !== undefined && creator !== producer
      ? message("properties.madeWith.both", { creator, producer })
      : (creator ?? producer),
  );
  return rows;
}

/** `24 · labelled 1–21, A-1–A-3`: the count, and the labels' runs when the document labels its pages. */
function pagesValue(count: number, labels: readonly string[] | undefined): string {
  if (labels === undefined || labels.length === 0) return String(count);
  const runs: string[] = [];
  let start = 0;
  const prefixAndNumber = (label: string) => /^(.*?)(\d+)$/.exec(label);
  for (let i = 1; i <= labels.length; i++) {
    const before = prefixAndNumber(labels[i - 1] ?? "");
    const here = i < labels.length ? prefixAndNumber(labels[i] ?? "") : null;
    const continues =
      before !== null && here !== null && before[1] === here[1] && Number(here[2]) === Number(before[2]) + 1;
    if (!continues) {
      const first = labels[start] ?? "";
      const last = labels[i - 1] ?? "";
      runs.push(first === last ? first : `${first}–${last}`);
      start = i;
    }
  }
  return message("properties.pages.labelled", { count, labels: runs.join(", ") });
}

function documentRows(p: DocumentPropertiesData, ctx: PropertiesContext): PropertyRow[] {
  const contains = [
    p.contains.attachments === 0
      ? message("properties.contains.noAttachments")
      : message(p.contains.attachments === 1 ? "properties.contains.attachment" : "properties.contains.attachments", {
          count: p.contains.attachments,
        }),
    message(p.contains.forms ? "properties.contains.form" : "properties.contains.noForms"),
    message(p.contains.javascript ? "properties.contains.javascript" : "properties.contains.noJavascript"),
  ].join(" · ");
  const rows: PropertyRow[] = [];
  if (p.version !== undefined) rows.push({ label: message("properties.version"), value: p.version });
  rows.push({ label: message("properties.pages"), value: pagesValue(p.pageCount, ctx.labels) });
  if (ctx.pages.length > 0) rows.push({ label: message("properties.pageSize"), value: pageSizeSummary(ctx.pages) });
  rows.push(
    { label: message("properties.fileSize"), value: formatFileSize(p.fileSize, ctx.locale) },
    { label: message("properties.tagged"), value: message(p.tagged ? "properties.tagged.yes" : "properties.no") },
    { label: message("properties.fastWebView"), value: yes(p.linearized) },
    { label: message("properties.contains"), value: contains },
  );
  return rows;
}

/** `AES 256-bit`, `RC4 128-bit`: the cipher's name and the key length. */
function cipherName(cipher: string, bits: number): string {
  const name = cipher.startsWith("aes") ? "AES" : cipher === "rc4" ? "RC4" : cipher;
  return `${name} ${bits}-bit`;
}

/** What the author restricted, and what they allow: `Printing: not allowed · Copying text: allowed`. */
export function permissionsValue(permissions: PropertiesPermissions): string {
  const allowed = (flag: boolean) => message(flag ? "properties.allowed" : "properties.notAllowed");
  const printing = permissions.print
    ? permissions.printHighQuality
      ? message("properties.allowed")
      : message("properties.permission.standardOnly")
    : message("properties.notAllowed");
  return [
    `${message("properties.permission.print")}: ${printing}`,
    `${message("properties.permission.copy")}: ${allowed(permissions.copy)}`,
    `${message("properties.permission.modify")}: ${allowed(permissions.modify)}`,
    `${message("properties.permission.annotate")}: ${allowed(permissions.annotate)}`,
    `${message("properties.permission.fillForms")}: ${allowed(permissions.fillForms)}`,
  ].join(" · ");
}

function securityRows(p: DocumentPropertiesData): PropertyRow[] {
  const { encryption } = p;
  const rows: PropertyRow[] = [
    {
      label: message("properties.encryption"),
      value:
        encryption === undefined
          ? message("properties.encryption.none")
          : message("properties.encryption.value", {
              cipher: cipherName(encryption.cipher, encryption.keyBits),
              revision: encryption.revision,
            }),
    },
  ];
  if (encryption?.permissions !== undefined) {
    rows.push({ label: message("properties.permissions"), value: permissionsValue(encryption.permissions) });
  }
  rows.push({
    label: message("properties.signatures"),
    value:
      p.signatures === 0
        ? message("properties.signatures.none")
        : message(p.signatures === 1 ? "properties.signatures.one" : "properties.signatures.many", {
            count: p.signatures,
          }),
  });
  return rows;
}

/** A font's "In the file" column. */
export function fontStorage(font: PropertiesFont): string {
  if (!font.embedded) return message("properties.font.notEmbedded");
  return message(font.subset ? "properties.font.subset" : "properties.font.embedded");
}

/** The Description, Document and Security sections, in the dialog's order (Fonts is a table between them). */
export function propertySections(
  p: DocumentPropertiesData,
  ctx: PropertiesContext,
): {
  description: PropertySection;
  document: PropertySection;
  security: PropertySection;
} {
  return {
    description: { title: message("properties.section.description"), rows: description(p, ctx.locale) },
    document: { title: message("properties.section.document"), rows: documentRows(p, ctx) },
    security: { title: message("properties.section.security"), rows: securityRows(p) },
  };
}

/** Copy all: every section as plain text, a `Label: value` line per row, the fonts one per line. */
export function propertiesText(p: DocumentPropertiesData, ctx: PropertiesContext): string {
  const { description: d, document, security } = propertySections(p, ctx);
  const block = (section: PropertySection) =>
    [section.title, ...section.rows.map((row) => `${row.label}: ${row.value}`)].join("\n");
  const fonts = [
    message("properties.section.fonts"),
    ...p.fonts.map((f) =>
      [f.name ?? message("properties.font.unnamed"), f.kind ?? "", fontStorage(f)].filter(Boolean).join(" · "),
    ),
  ].join("\n");
  return [block(d), block(document), fonts, block(security)].join("\n\n");
}
