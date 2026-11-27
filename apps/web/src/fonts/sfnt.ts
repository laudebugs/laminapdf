// What the font picker needs to know of a font file before it hands the bytes to the core: that it
// is a TrueType or OpenType font (not a web font), its family, whether its licence lets it be
// embedded (`OS/2` `fsType`, ISO 32000-2 §9.9), and the characters it has (its `cmap`). The core
// reads and embeds the font; this only lets the picker refuse a file or choose a fallback early.

import { type CodePoints, merge } from "./ranges.ts";

/** What a font file declares. */
export interface FontFacts {
  /** The family name, else the empty string. */
  readonly family: string;
  /** Whether the font's `fsType` forbids embedding it: restricted licence, or bitmaps only. */
  readonly restricted: boolean;
  /** The characters it has, from its Unicode `cmap`. */
  readonly coverage: CodePoints;
}

/** Why a file isn't a font the reader can use. */
export type NotAFont = "empty" | "web-font" | "unknown-format" | "damaged";

const TTF = 0x00010000;
const TRUE = 0x74727565;
const OTTO = 0x4f54544f;
const TTCF = 0x74746366;
const WOFF = 0x774f4646;
const WOFF2 = 0x774f4632;

/** The facts of `bytes`, or why it isn't a font. */
export function readFont(bytes: Uint8Array): FontFacts | NotAFont {
  if (bytes.length === 0) return "empty";
  if (bytes.length < 12) return "damaged";
  const view = new DataView(bytes.buffer, bytes.byteOffset, bytes.byteLength);
  const tag = view.getUint32(0);
  if (tag === WOFF || tag === WOFF2) return "web-font";
  // A collection's first font is the one the core reads.
  const start = tag === TTCF && bytes.length >= 16 ? view.getUint32(12) : 0;
  const header = tag === TTCF ? view.getUint32(start) : tag;
  if (header !== TTF && header !== TRUE && header !== OTTO) return "unknown-format";
  try {
    const tables = directory(view, start);
    return {
      family: familyOf(view, tables.get("name")),
      restricted: restrictedBy(view, tables.get("OS/2")),
      coverage: charactersOf(view, tables.get("cmap")),
    };
  } catch {
    return "damaged";
  }
}

type Tables = ReadonlyMap<string, number>;

/** Each table's offset by its tag. */
function directory(view: DataView, start: number): Tables {
  const count = view.getUint16(start + 4);
  const tables = new Map<string, number>();
  for (let i = 0; i < count; i++) {
    const record = start + 12 + 16 * i;
    const tag = String.fromCharCode(...[0, 1, 2, 3].map((k) => view.getUint8(record + k)));
    const offset = view.getUint32(record + 8);
    if (offset >= view.byteLength) throw new RangeError("a table is outside the file");
    tables.set(tag, offset);
  }
  return tables;
}

/** `fsType`: restricted licence embedding (bit 1) unless preview and print or editable embedding
 * (bits 2 and 3) is also allowed, the least restrictive winning; bitmaps only (bit 9). */
function restrictedBy(view: DataView, os2: number | undefined): boolean {
  if (os2 === undefined) return false;
  const fsType = view.getUint16(os2 + 8);
  const restricted = (fsType & 0x0002) !== 0 && (fsType & 0x000c) === 0;
  return restricted || (fsType & 0x0200) !== 0;
}

/** The typographic family (name ID 16), else the family (ID 1), in a Windows Unicode record. */
function familyOf(view: DataView, name: number | undefined): string {
  if (name === undefined) return "";
  const count = view.getUint16(name + 2);
  const strings = name + view.getUint16(name + 4);
  const found = new Map<number, string>();
  for (let i = 0; i < count; i++) {
    const record = name + 6 + 12 * i;
    const [platform, encoding, , id] = [0, 2, 4, 6].map((k) => view.getUint16(record + k));
    if (platform !== 3 || (encoding !== 1 && encoding !== 10) || (id !== 1 && id !== 16)) continue;
    const length = view.getUint16(record + 8);
    const offset = strings + view.getUint16(record + 10);
    let text = "";
    for (let k = 0; k + 1 < length; k += 2) text += String.fromCharCode(view.getUint16(offset + k));
    found.set(id ?? 0, text);
  }
  return found.get(16) ?? found.get(1) ?? "";
}

/** The code points of the best Unicode subtable: format 12, else format 4. */
function charactersOf(view: DataView, cmap: number | undefined): CodePoints {
  if (cmap === undefined) return [];
  const count = view.getUint16(cmap + 2);
  const pairs: [number, number][] = [];
  const seen = new Set<number>();
  for (let i = 0; i < count; i++) {
    const [platform, encoding] = [cmap + 4 + 8 * i, cmap + 6 + 8 * i].map((p) => view.getUint16(p));
    const unicode = platform === 0 || (platform === 3 && (encoding === 1 || encoding === 10));
    const table = cmap + view.getUint32(cmap + 8 + 8 * i);
    if (!unicode || seen.has(table)) continue;
    seen.add(table);
    const format = view.getUint16(table);
    if (format === 12) groups(view, table, pairs);
    else if (format === 4) segments(view, table, pairs);
  }
  return merge(pairs);
}

/** Format 12's groups: `[first, last, glyph]` each. */
function groups(view: DataView, table: number, out: [number, number][]): void {
  const count = view.getUint32(table + 12);
  for (let g = 0; g < count; g++) {
    const at = table + 16 + 12 * g;
    out.push([view.getUint32(at), view.getUint32(at + 4)]);
  }
}

/** Format 4's segments: the end codes, then the start codes; the last segment is the sentinel. */
function segments(view: DataView, table: number, out: [number, number][]): void {
  const count = view.getUint16(table + 6) / 2;
  const ends = table + 14;
  const starts = ends + 2 * count + 2;
  for (let s = 0; s < count; s++) {
    const [first, last] = [view.getUint16(starts + 2 * s), view.getUint16(ends + 2 * s)];
    if (first !== 0xffff && last >= first) out.push([first, last]);
  }
}
