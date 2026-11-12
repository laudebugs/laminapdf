// The XMP tab's data (O04): the packet read into schemas and their properties, what the reader
// may edit there, the edits made so far, and each edit's value as the edit ops take it. It reads
// the packet with the browser's DOMParser and depends on neither @laminapdf/sdk nor
// @laminapdf/viewer: the app turns the edits into ops.

import type { MetadataField } from "./properties-form.js";

const RDF = "http://www.w3.org/1999/02/22-rdf-syntax-ns#";
const XML_NS = "http://www.w3.org/XML/1998/namespace";
const PDFA_ID = "http://www.aiim.org/pdfa/ns/id/";
const PDFUA_ID = "http://www.aiim.org/pdfua/ns/id/";
const PDFA_EXTENSION = [
  "http://www.aiim.org/pdfa/ns/extension/",
  "http://www.aiim.org/pdfa/ns/schema#",
  "http://www.aiim.org/pdfa/ns/property#",
  "http://www.aiim.org/pdfa/ns/type#",
  "http://www.aiim.org/pdfa/ns/field#",
];

/** The schemas the tab names, in the order it lists them; others follow, by prefix. */
const KNOWN_SCHEMAS: readonly (readonly [namespace: string, name: string])[] = [
  ["http://purl.org/dc/elements/1.1/", "Dublin Core"],
  ["http://ns.adobe.com/pdf/1.3/", "PDF"],
  ["http://ns.adobe.com/xap/1.0/", "XMP basic"],
  ["http://ns.adobe.com/xap/1.0/mm/", "Media management"],
  ["http://ns.adobe.com/xap/1.0/rights/", "Rights management"],
  [PDFA_ID, "PDF/A identification"],
  [PDFUA_ID, "PDF/UA identification"],
  ["http://www.aiim.org/pdfa/ns/extension/", "PDF/A extension schemas"],
];

/**
 * The properties the Description tab edits, by namespace and name, as Rust's `xmp::MAPPINGS`
 * lists the Info-mapped ones (test/xmp-model.test.ts reads `mapping.rs` to hold them equal).
 */
export const MAPPED: readonly (readonly [namespace: string, name: string, field: MetadataField])[] = [
  ["http://purl.org/dc/elements/1.1/", "title", "title"],
  ["http://purl.org/dc/elements/1.1/", "creator", "authors"],
  ["http://purl.org/dc/elements/1.1/", "description", "subject"],
  ["http://ns.adobe.com/pdf/1.3/", "Keywords", "keywords"],
  ["http://ns.adobe.com/pdf/1.3/", "Producer", "producer"],
  ["http://ns.adobe.com/xap/1.0/", "CreatorTool", "creator"],
  ["http://ns.adobe.com/xap/1.0/", "CreateDate", "created"],
  ["http://ns.adobe.com/xap/1.0/", "ModifyDate", "modified"],
];

/** A value as the edit ops take it (`@laminapdf/sdk`'s `XmpValue`). */
export type XmpValueData =
  | { readonly type: "text" | "uri"; readonly value: string }
  | { readonly type: "integer" | "real"; readonly value: number }
  | { readonly type: "boolean"; readonly value: boolean }
  | { readonly type: "date"; readonly value: string }
  | { readonly type: "langAlt"; readonly value: readonly { readonly lang: string; readonly text: string }[] }
  | { readonly type: "seq" | "bag"; readonly value: readonly XmpValueData[] };

/** The types the tab offers, as the edit ops name them. */
export type XmpType = "text" | "uri" | "integer" | "real" | "boolean" | "date" | "seq" | "bag" | "langAlt";

/** One XMP edit the tab collects; the app makes each an op. */
export type XmpChange =
  | {
      readonly kind: "set";
      readonly namespace: string;
      readonly prefix: string;
      readonly name: string;
      readonly value: XmpValueData;
    }
  | { readonly kind: "remove"; readonly namespace: string; readonly name: string }
  | { readonly kind: "replace"; readonly xml: string };

/** A property as the packet holds it. */
export interface XmpProperty {
  readonly namespace: string;
  readonly prefix: string;
  readonly name: string;
  /** Its value as text the tab can edit: one line, one item per line, or `lang: text` per line. */
  readonly text: string;
  /** The type the tab shows; `undefined` for a structure, which only Raw XML edits. */
  readonly type: XmpType | undefined;
  /** The Description tab's field, when the Info dictionary mirrors the property. */
  readonly field?: MetadataField | undefined;
  /** Whether the tab leaves it alone: the PDF/A or PDF/UA identification, or the extension schemas. */
  readonly kept: boolean;
}

/** A schema and its properties. */
export interface XmpSchema {
  readonly namespace: string;
  readonly prefix: string;
  /** Its name, for a schema the tab knows; else undefined, and the tab shows the prefix. */
  readonly name: string | undefined;
  readonly properties: readonly XmpProperty[];
}

/** Why XML didn't parse: the browser's message, and where, when it says. */
export interface XmlError {
  readonly message: string;
  readonly line?: number | undefined;
  readonly column?: number | undefined;
}

/** The packet read, or why it couldn't be. */
export type XmpRead =
  | { readonly ok: true; readonly schemas: readonly XmpSchema[] }
  | { readonly ok: false; readonly error: XmlError };

/** The line and column in a parser error's text: Chromium's and WebKit's "line 3 at column 7", Firefox's "Line Number 3, Column 7". */
function position(text: string): Pick<XmlError, "line" | "column"> {
  const m = /line(?: number)?\s+(\d+)(?:,|\s+at)\s+column\s+(\d+)/i.exec(text);
  return m === null ? {} : { line: Number(m[1]), column: Number(m[2]) };
}

/** `xml` parsed, or the parser's error: the first line of its message and where it is. */
export function parseXml(xml: string): Document | XmlError {
  const doc = new DOMParser().parseFromString(xml, "application/xml");
  const error = doc.getElementsByTagName("parsererror")[0];
  if (error === undefined) return doc;
  const text = error.textContent ?? "";
  const message = text
    .split("\n")
    .map((l) => l.trim())
    .find((l) => l.length > 0 && !/^(This page contains|Below is a rendering|XML Parsing Error:?$)/.test(l));
  return { message: message ?? text.trim(), ...position(text) };
}

/** The type the tab shows for property element `e`; undefined for a structure or a qualified value. */
function typeOf(e: Element): { type: XmpType | undefined; text: string } {
  if (e.getAttributeNS(RDF, "resource") !== null) return { type: "uri", text: e.getAttributeNS(RDF, "resource") ?? "" };
  if (e.attributes.length > 0 && [...e.attributes].some((a) => !a.name.startsWith("xmlns")))
    return { type: undefined, text: "" };
  const children = [...e.children];
  if (children.length === 0) return { type: "text", text: e.textContent ?? "" };
  const array = children[0];
  if (children.length > 1 || array === undefined || array.namespaceURI !== RDF) return { type: undefined, text: "" };
  const items = [...array.children];
  if (items.some((li) => li.localName !== "li" || li.children.length > 0)) return { type: undefined, text: "" };
  if (array.localName === "Alt") {
    const lines = items.map((li) => `${li.getAttributeNS(XML_NS, "lang") ?? "x-default"}: ${li.textContent ?? ""}`);
    return { type: "langAlt", text: lines.join("\n") };
  }
  const type = array.localName === "Seq" ? "seq" : array.localName === "Bag" ? "bag" : undefined;
  return { type, text: items.map((li) => li.textContent ?? "").join("\n") };
}

/** A property as the tab shows it, with whether the Description tab edits it or the tab keeps it. */
function shownProperty(namespace: string, prefix: string, name: string, shown: ReturnType<typeof typeOf>): XmpProperty {
  const field = MAPPED.find(([ns, n]) => ns === namespace && n === name)?.[2];
  const kept = namespace === PDFA_ID || namespace === PDFUA_ID || PDFA_EXTENSION.includes(namespace);
  return { namespace, prefix, name, field, kept, ...shown };
}

/** The properties of one `rdf:Description`: its attributes, then its child elements. */
function descriptionProperties(d: Element): XmpProperty[] {
  const attributes = [...d.attributes].filter(
    (a) => a.namespaceURI !== null && a.namespaceURI !== RDF && !a.name.startsWith("xmlns"),
  );
  const elements = [...d.children].filter((e) => e.namespaceURI !== null);
  return [
    ...attributes.map((a) =>
      shownProperty(a.namespaceURI ?? "", a.prefix ?? "", a.localName, { type: "text", text: a.value }),
    ),
    ...elements.map((e) => shownProperty(e.namespaceURI ?? "", e.prefix ?? "", e.localName, typeOf(e))),
  ];
}

/** Every property of the packet's descriptions, attribute and element forms. */
function properties(doc: Document): XmpProperty[] {
  return [...doc.getElementsByTagNameNS(RDF, "Description")]
    .filter((d) => d.parentElement?.namespaceURI === RDF && d.parentElement.localName === "RDF")
    .flatMap(descriptionProperties);
}

/** The packet's properties grouped by schema: the known schemas first, in order, then the others by prefix. */
export function readXmpPacket(xml: string): XmpRead {
  const doc = parseXml(xml);
  if (!(doc instanceof Document)) return { ok: false, error: doc };
  if (doc.getElementsByTagNameNS(RDF, "RDF").length === 0) return { ok: false, error: { message: "no rdf:RDF" } };
  const groups = new Map<string, XmpProperty[]>();
  for (const p of properties(doc)) groups.set(p.namespace, [...(groups.get(p.namespace) ?? []), p]);
  const rank = (ns: string) => {
    const i = KNOWN_SCHEMAS.findIndex(([known]) => known === ns);
    return i === -1 ? KNOWN_SCHEMAS.length : i;
  };
  const schemas = [...groups].map(([namespace, props]) => ({
    namespace,
    prefix: props[0]?.prefix ?? "",
    name: KNOWN_SCHEMAS.find(([ns]) => ns === namespace)?.[1],
    properties: props,
  }));
  schemas.sort((a, b) => rank(a.namespace) - rank(b.namespace) || a.prefix.localeCompare(b.prefix));
  return { ok: true, schemas };
}

/** An XMP date (ISO 8601: `2026-10-14`, `2026-10-14T10:20:00+02:00`) as a PDF date; undefined when it isn't one. */
export function isoToPdfDate(iso: string): string | undefined {
  const m = /^(\d{4})(?:-(\d{2})(?:-(\d{2})(?:T(\d{2}):(\d{2})(?::(\d{2})(?:\.\d+)?)?(Z|[+-]\d{2}:\d{2})?)?)?)?$/.exec(
    iso.trim(),
  );
  if (m === null) return undefined;
  const [y, mo = "", d = "", h = "", mi = "", s = "", zone] = m.slice(1) as [string, ...(string | undefined)[]];
  const tz = zone === undefined ? "" : zone === "Z" ? "Z" : `${zone.slice(0, 3)}'${zone.slice(4)}'`;
  return `D:${y}${mo}${d}${h}${mi}${s}${h === "" ? "" : tz}`;
}

/** Text the tab shows for a simple value, as the edit ops take it; undefined when the text doesn't fit the type. */
function simple(type: Exclude<XmpType, "seq" | "bag" | "langAlt">, text: string): XmpValueData | undefined {
  const t = text.trim();
  switch (type) {
    case "text":
      return { type, value: text };
    case "uri":
      return t === "" ? undefined : { type, value: t };
    case "integer":
      return /^-?\d+$/.test(t) && Number.isSafeInteger(Number(t)) ? { type, value: Number(t) } : undefined;
    case "real":
      return /^-?\d+(\.\d+)?$/.test(t) ? { type, value: Number(t) } : undefined;
    case "boolean":
      return /^(true|false)$/i.test(t) ? { type, value: /^true$/i.test(t) } : undefined;
    case "date": {
      const date = isoToPdfDate(t);
      return date === undefined ? undefined : { type, value: date };
    }
  }
}

/**
 * The tab's text for a value of `type` as the edit ops take it: one line for a simple type, one
 * item per line for a list (of text), `lang: text` per line for a language alternative.
 * Undefined when it doesn't fit: a number that isn't one, an empty list, a line without a language.
 */
export function xmpValueOf(type: XmpType, text: string): XmpValueData | undefined {
  if (type === "seq" || type === "bag") {
    const items = text.split("\n").filter((l) => l.trim() !== "");
    return items.length === 0 ? undefined : { type, value: items.map((value) => ({ type: "text", value })) };
  }
  if (type === "langAlt") {
    const entries: { lang: string; text: string }[] = [];
    for (const line of text.split("\n").filter((l) => l.trim() !== "")) {
      const m = /^\s*([A-Za-z][A-Za-z0-9-]*)\s*:\s?(.*)$/.exec(line);
      if (m === null || entries.some((e) => e.lang.toLowerCase() === (m[1] ?? "").toLowerCase())) return undefined;
      entries.push({ lang: m[1] ?? "", text: m[2] ?? "" });
    }
    return entries.length === 0 ? undefined : { type, value: entries };
  }
  return simple(type, text);
}

/** Whether `name` can name a property: an XML name without a colon (ASCII letters, digits, `-`, `_`, `.`). */
export function isXmlName(name: string): boolean {
  return /^[A-Za-z_][A-Za-z0-9_.-]*$/.test(name);
}

/** Whether `prefix` can name a namespace: as {@link isXmlName}, and not starting with `xml`, which XML reserves. */
export function isXmlPrefix(prefix: string): boolean {
  return isXmlName(prefix) && !/^xml/i.test(prefix);
}

/** The key of a property's change: its namespace and name. */
export function changeKey(change: { readonly namespace: string; readonly name: string }): string {
  return `${change.namespace} ${change.name}`;
}

/** `changes` without any change to the property `key` names ({@link changeKey}). */
export function withoutChange(changes: readonly XmpChange[], key: string): XmpChange[] {
  return changes.filter((c) => c.kind === "replace" || changeKey(c) !== key);
}

/** `changes` with `change` in place of an earlier one for the same property; a replaced packet drops the others. */
export function withChange(changes: readonly XmpChange[], change: XmpChange): XmpChange[] {
  if (change.kind === "replace") return [change];
  return [...withoutChange(changes, changeKey(change)).filter((c) => c.kind !== "replace"), change];
}

/** Whether `changes` remove values the file's earlier versions keep: a removal or a replaced packet, saved incrementally. */
export function xmpRemovalStays(changes: readonly XmpChange[], rewritesWhole: boolean): boolean {
  return !rewritesWhole && changes.some((c) => c.kind === "remove" || c.kind === "replace");
}
