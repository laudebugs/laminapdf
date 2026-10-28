import { ByteSink } from "./bytes.js";
import { open } from "./document.js";
import { PdfError } from "./errors.js";
import { flateEncode } from "./filters.js";
import type { PdfDict, PdfObject, PdfRef, PdfStream } from "./objects.js";
import { int, ref as mkRef, name } from "./objects.js";
import { writeObject } from "./serialize.js";

/** End-of-line of 20-byte xref entries: `\r\n` or ` \n` (both legal, §7.5.4). */
export type XrefEol = "CrLf" | "SpaceLf";

/** A pre-serialized object body, written verbatim between `n g obj\n` and `\nendobj`. */
export interface RawBody {
  readonly kind: "raw";
  readonly bytes: Uint8Array;
}

export type WriterBody = PdfObject | RawBody;

export interface WriterObject {
  readonly num: number;
  readonly gen?: number;
  readonly body: WriterBody;
}

export interface WriteOptions {
  /** Header version, default "1.7". Compact mode needs ≥ 1.5 and bumps lower values to 1.5. */
  version?: string;
  /** Default `CrLf`. */
  xrefEol?: XrefEol;
  /** Flate-encode every stream that has no /Filter yet (sets /Filter /FlateDecode). */
  compressStreams?: boolean;
  /** `classic` (default): xref table. `compact`: object stream + xref stream. */
  mode?: "classic" | "compact";
}

export interface IncrementalOptions {
  xrefEol?: XrefEol;
  compressStreams?: boolean;
  /** Entries to add/replace in the new trailer (e.g. a new /Info). */
  trailer?: PdfDict;
}

export const raw = (bytes: Uint8Array | string): RawBody => ({
  kind: "raw",
  bytes: typeof bytes === "string" ? binary(bytes) : bytes,
});

function binary(s: string): Uint8Array {
  const out = new Uint8Array(s.length);
  for (let i = 0; i < s.length; i++) out[i] = s.charCodeAt(i) & 0xff;
  return out;
}

const HEADER_BINARY = "%\xE2\xE3\xCF\xD3\n";

/** Collects numbered objects and serializes them as a complete file. */
export class PdfWriter {
  private readonly objects = new Map<number, WriterObject>();
  private nextNum = 1;

  /** Reserve the next object number (for forward references). */
  alloc(): PdfRef {
    const r = mkRef(this.nextNum++);
    return r;
  }

  /** Add a body under the next free number. */
  add(body: WriterBody): PdfRef {
    const r = this.alloc();
    this.set(r, body);
    return r;
  }

  /** Set (or replace) object `num`. */
  set(target: PdfRef | number, body: WriterBody, gen?: number): this {
    const num = typeof target === "number" ? target : target.num;
    const g = gen ?? (typeof target === "number" ? 0 : target.gen);
    checkNum(num, g);
    this.objects.set(num, { num, gen: g, body });
    if (num >= this.nextNum) this.nextNum = num + 1;
    return this;
  }

  /** Serialize. `/Size` in the trailer is computed (kept in place if supplied, else written first). */
  toBytes(trailer: PdfDict, options: WriteOptions = {}): Uint8Array {
    return writePdf([...this.objects.values()], trailer, options);
  }
}

function checkNum(num: number, gen: number): void {
  if (!Number.isSafeInteger(num) || num < 1) throw new PdfError("Invalid", `bad object number ${num}`);
  if (!Number.isSafeInteger(gen) || gen < 0 || gen > 65535) throw new PdfError("Invalid", `bad generation ${gen}`);
}

/** Write a complete PDF from numbered objects (written in ascending number order). */
export function writePdf(objects: readonly WriterObject[], trailer: PdfDict, options: WriteOptions = {}): Uint8Array {
  const sorted = sortObjects(objects);
  return options.mode === "compact" ? writeCompact(sorted, trailer, options) : writeClassic(sorted, trailer, options);
}

function sortObjects(objects: readonly WriterObject[]): WriterObject[] {
  const byNum = new Map<number, WriterObject>();
  for (const o of objects) {
    checkNum(o.num, o.gen ?? 0);
    byNum.set(o.num, o);
  }
  return [...byNum.values()].sort((a, b) => a.num - b.num);
}

function eolOf(e: XrefEol | undefined): string {
  return e === "SpaceLf" ? " \n" : "\r\n";
}

function pad(n: number, width: number): string {
  const s = String(n);
  if (s.length > width) throw new PdfError("Invalid", `value ${n} does not fit ${width} digits`);
  return "0".repeat(width - s.length) + s;
}

/** Prepare a stream for writing: optional Flate encoding. /Length is set by the serializer. */
function prepare(body: WriterBody, compress: boolean): WriterBody {
  if (body.kind !== "stream" || !compress || body.dict.entries.has("Filter")) return body;
  const entries = new Map(body.dict.entries);
  entries.set("Filter", name("FlateDecode"));
  const s: PdfStream = { kind: "stream", dict: { kind: "dict", entries }, data: flateEncode(body.data) };
  return s;
}

function writeBody(sink: ByteSink, o: WriterObject, compress: boolean): void {
  sink.str(`${o.num} ${o.gen ?? 0} obj\n`);
  const body = prepare(o.body, compress);
  if (body.kind === "raw") sink.bytes(body.bytes);
  else writeObject(sink, body);
  sink.str("\nendobj\n");
}

/** Copy `trailer` with /Size set (in place if present, else first). */
function trailerWithSize(trailer: PdfDict, size: number): PdfDict {
  const entries = new Map<string, PdfObject>();
  if (!trailer.entries.has("Size")) entries.set("Size", int(size));
  for (const [k, v] of trailer.entries) entries.set(k, k === "Size" ? int(size) : v);
  return { kind: "dict", entries };
}

function writeClassic(objs: WriterObject[], trailer: PdfDict, options: WriteOptions): Uint8Array {
  const sink = new ByteSink(4096);
  const eol = eolOf(options.xrefEol);
  const compress = options.compressStreams === true;
  sink.str(`%PDF-${options.version ?? "1.7"}\n${HEADER_BINARY}`);
  const offsets = new Map<number, { off: number; gen: number }>();
  for (const o of objs) {
    offsets.set(o.num, { off: sink.length, gen: o.gen ?? 0 });
    writeBody(sink, o, compress);
  }
  const size = (objs.at(-1)?.num ?? 0) + 1;
  const xref = sink.length;
  sink.str(`xref\n0 ${size}\n`);
  // Free list: 0 → first gap → … → 0.
  const gaps: number[] = [];
  for (let n = 1; n < size; n++) if (!offsets.has(n)) gaps.push(n);
  sink.str(`${pad(gaps[0] ?? 0, 10)} 65535 f${eol}`);
  let gi = 0;
  for (let n = 1; n < size; n++) {
    const e = offsets.get(n);
    if (e === undefined) {
      gi++;
      sink.str(`${pad(gaps[gi] ?? 0, 10)} 00000 f${eol}`);
    } else sink.str(`${pad(e.off, 10)} ${pad(e.gen, 5)} n${eol}`);
  }
  sink.str("trailer\n");
  writeObject(sink, trailerWithSize(trailer, size));
  sink.str(`\nstartxref\n${xref}\n%%EOF\n`);
  return sink.toBytes();
}

function versionAtLeast15(v: string): string {
  const m = /^(\d+)\.(\d+)$/.exec(v);
  if (m === null) return v;
  const major = Number(m[1]);
  const minor = Number(m[2]);
  return major > 1 || (major === 1 && minor >= 5) ? v : "1.5";
}

function writeCompact(objs: WriterObject[], trailer: PdfDict, options: WriteOptions): Uint8Array {
  const compress = options.compressStreams === true;
  const sink = new ByteSink(4096);
  sink.str(`%PDF-${versionAtLeast15(options.version ?? "1.7")}\n${HEADER_BINARY}`);
  const maxNum = objs.at(-1)?.num ?? 0;
  const packed = objs.filter((o) => (o.gen ?? 0) === 0 && o.body.kind !== "stream" && o.body.kind !== "raw");
  const direct = objs.filter((o) => !packed.includes(o));
  const stmNum = packed.length > 0 ? maxNum + 1 : 0;
  const xrefNum = packed.length > 0 ? maxNum + 2 : maxNum + 1;
  const size = xrefNum + 1;

  type Row = readonly [number, number, number];
  const rows = new Map<number, Row>();
  for (const o of direct) {
    rows.set(o.num, [1, sink.length, o.gen ?? 0]);
    writeBody(sink, o, compress);
  }
  if (packed.length > 0) {
    const header = new ByteSink(256);
    const body = new ByteSink(4096);
    packed.forEach((o, i) => {
      if (i > 0) header.byte(0x20);
      header.str(`${o.num} ${body.length}`);
      writeObject(body, o.body as PdfObject);
      body.byte(0x0a);
      rows.set(o.num, [2, stmNum, i]);
    });
    header.byte(0x0a);
    const first = header.length;
    header.bytes(body.toBytes());
    const data = flateEncode(header.toBytes());
    const d = new Map<string, PdfObject>([
      ["Type", name("ObjStm")],
      ["N", int(packed.length)],
      ["First", int(first)],
      ["Filter", name("FlateDecode")],
    ]);
    rows.set(stmNum, [1, sink.length, 0]);
    writeBody(sink, { num: stmNum, body: { kind: "stream", dict: { kind: "dict", entries: d }, data } }, false);
  }
  const xrefOff = sink.length;
  rows.set(xrefNum, [1, xrefOff, 0]);
  const W = [1, 4, 2] as const;
  const table = new Uint8Array(size * 7);
  for (let n = 0; n < size; n++) {
    const r: Row = rows.get(n) ?? (n === 0 ? [0, 0, 65535] : [0, 0, 0]);
    let p = n * 7;
    table[p++] = r[0];
    for (let k = W[1] - 1; k >= 0; k--) table[p++] = Math.floor(r[1] / 256 ** k) & 0xff;
    table[p++] = (r[2] >> 8) & 0xff;
    table[p++] = r[2] & 0xff;
  }
  const d = new Map<string, PdfObject>([
    ["Type", name("XRef")],
    ["Size", int(size)],
    ["W", { kind: "array", items: W.map((w) => int(w)) }],
    ["Index", { kind: "array", items: [int(0), int(size)] }],
  ]);
  for (const [k, v] of trailer.entries) {
    if (k === "Size" || k === "Prev" || k === "Type" || k === "W" || k === "Index" || k === "Filter" || k === "Length") continue;
    d.set(k, v);
  }
  d.set("Filter", name("FlateDecode"));
  writeBody(sink, { num: xrefNum, body: { kind: "stream", dict: { kind: "dict", entries: d }, data: flateEncode(table) } }, false);
  sink.str(`startxref\n${xrefOff}\n%%EOF\n`);
  return sink.toBytes();
}

/**
 * Append an incremental update (§7.5.6): new/changed objects, an xref section
 * (entry 0 plus one subsection per run of consecutive numbers), and a trailer
 * repeating the previous trailer's keys with /Size updated and /Prev added.
 * The original bytes are kept untouched as a prefix.
 */
export function writeIncremental(original: Uint8Array, objects: readonly WriterObject[], options: IncrementalOptions = {}): Uint8Array {
  const doc = open(original);
  const prev = doc.startxrefOffset();
  if (doc.wasRepaired() || prev === undefined) {
    throw new PdfError("Xref", "cannot append an incremental update to a document that needed repair");
  }
  const objs = sortObjects(objects);
  const eol = eolOf(options.xrefEol);
  const compress = options.compressStreams === true;
  const sink = new ByteSink(original.length + 4096);
  sink.bytes(original);
  const last = original[original.length - 1];
  if (last !== 0x0a && last !== 0x0d) sink.byte(0x0a);
  const offsets: Array<{ num: number; gen: number; off: number }> = [];
  for (const o of objs) {
    offsets.push({ num: o.num, gen: o.gen ?? 0, off: sink.length });
    writeBody(sink, o, compress);
  }
  const xref = sink.length;
  sink.str(`xref\n0 1\n0000000000 65535 f${eol}`);
  for (let i = 0; i < offsets.length; ) {
    let j = i;
    while (j + 1 < offsets.length && offsets[j + 1]!.num === offsets[j]!.num + 1) j++;
    sink.str(`${offsets[i]!.num} ${j - i + 1}\n`);
    for (let k = i; k <= j; k++) sink.str(`${pad(offsets[k]!.off, 10)} ${pad(offsets[k]!.gen, 5)} n${eol}`);
    i = j + 1;
  }
  const old = doc.trailer().entries;
  const oldSize = old.get("Size");
  const size = Math.max(oldSize?.kind === "int" ? oldSize.value : 0, (objs.at(-1)?.num ?? 0) + 1);
  const entries = new Map<string, PdfObject>();
  for (const [k, v] of old) {
    if (k === "Prev" || k === "XRefStm" || k === "Type" || k === "W" || k === "Index" || k === "Filter" || k === "DecodeParms" || k === "Length") continue;
    entries.set(k, k === "Size" ? int(size) : v);
  }
  if (!entries.has("Size")) entries.set("Size", int(size));
  for (const [k, v] of options.trailer?.entries ?? []) if (k !== "Size" && k !== "Prev") entries.set(k, v);
  entries.set("Prev", int(prev));
  sink.str("trailer\n");
  writeObject(sink, { kind: "dict", entries });
  sink.str(`\nstartxref\n${xref}\n%%EOF\n`);
  return sink.toBytes();
}
