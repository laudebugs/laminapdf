import { ascii, indexOf, lastIndexOf } from "./bytes.js";
import { PdfError } from "./errors.js";
import { DEFAULT_MAX_DECODED, flateDecode, unpredict } from "./filters.js";
import { isDelimiter, isDigit, isWhitespace, Lexer } from "./lexer.js";
import type { PdfArray, PdfDict, PdfObject, PdfRef, PdfStream } from "./objects.js";
import { ref as mkRef, NULL } from "./objects.js";
import { Parser, parseIndirectObject, parseObjHeader } from "./parser.js";

export interface OpenOptions {
  /** Cap on the decoded size of any single stream (bytes). Default 64 MiB. */
  maxDecodedStreamSize?: number;
}

/** One cross-reference entry. */
export type XrefEntry =
  | { readonly type: "free" }
  | { readonly type: "offset"; readonly offset: number; readonly gen: number }
  | { readonly type: "compressed"; readonly stream: number; readonly index: number };

export type Rect = readonly [number, number, number, number];

/** A leaf of the page tree with inheritable attributes applied. */
export interface Page {
  /** Indirect reference of the page object (undefined for an inline dict, which is invalid but tolerated). */
  readonly ref: PdfRef | undefined;
  readonly dict: PdfDict;
  readonly mediaBox: Rect | undefined;
  readonly cropBox: Rect | undefined;
  readonly rotate: number;
  readonly resources: PdfDict | undefined;
}

interface XrefSection {
  entries: Map<number, XrefEntry>;
  trailer: PdfDict;
}

interface ObjStmCache {
  data: Uint8Array;
  first: number;
  pairs: Array<readonly [number, number]>;
  /** Sorted distinct member offsets (relative to /First), to bound each member's parse. */
  bounds: number[];
}

const STARTXREF = ascii("startxref");
const TRAILER = ascii("trailer");
const PDF_HEADER = ascii("%PDF-");
/** Keys of an xref-stream dict that must not leak into a trailer copy. */
const XREF_STREAM_KEYS = new Set(["Type", "W", "Index", "Filter", "DecodeParms", "Length", "XRefStm", "Prev", "F", "FFilter", "FDecodeParms", "DL"]);

/** Open a PDF from bytes. Throws {@link PdfError} if no usable document can be found even after repair. */
export function open(bytes: Uint8Array, options: OpenOptions = {}): PdfDocument {
  return new PdfDocument(bytes, options);
}

export class PdfDocument {
  readonly bytes: Uint8Array;
  readonly maxDecoded: number;
  private xref = new Map<number, XrefEntry>();
  private trailerDict: PdfDict = { kind: "dict", entries: new Map() };
  private revisionCount = 0;
  private repaired = false;
  /** Offset of the newest xref section (what `startxref` points at), when not repaired. */
  private startxref: number | undefined;
  private cache = new Map<number, PdfObject>();
  private loading = new Set<number>();
  private objStms = new Map<number, ObjStmCache>();
  private pageList: Page[] | undefined;
  private offsetsSorted: number[] | undefined;

  constructor(bytes: Uint8Array, options: OpenOptions) {
    this.bytes = bytes;
    this.maxDecoded = options.maxDecodedStreamSize ?? DEFAULT_MAX_DECODED;
    if (bytes.length === 0) throw new PdfError("Xref", "empty input");
    let ok = false;
    try {
      ok = this.loadXref() && this.catalogLooksValid();
    } catch (e) {
      if (!(e instanceof PdfError)) throw e;
      ok = false;
    }
    if (!ok) this.repair();
  }

  // -------------------------------------------------------------------------
  // Public API

  /** The newest trailer dictionary (for xref-stream files: the stream's dict). */
  trailer(): PdfDict {
    return this.trailerDict;
  }

  /** The document catalog (`/Root`). */
  catalog(): PdfDict {
    const root = this.resolve(this.trailerDict.entries.get("Root") ?? NULL);
    if (root.kind !== "dict") throw new PdfError("Invalid", "/Root is not a dictionary");
    return root;
  }

  /** Follow references until a direct object is reached. Missing/free objects are `null`. */
  resolve(o: PdfObject): PdfObject {
    let cur = o;
    let seen: Set<number> | undefined;
    while (cur.kind === "ref") {
      seen ??= new Set();
      if (seen.has(cur.num)) throw new PdfError("Cycle", `reference cycle through ${cur.num} ${cur.gen} R`);
      seen.add(cur.num);
      cur = this.getObject(cur.num);
    }
    return cur;
  }

  pageCount(): number {
    return this.pages().length;
  }

  pages(): Page[] {
    this.pageList ??= this.walkPages();
    return this.pageList;
  }

  /** Number of xref sections reached from `startxref` via `/Prev` (after repair: trailers found, min 1). */
  revisions(): number {
    return this.revisionCount;
  }

  isEncrypted(): boolean {
    return this.trailerDict.entries.has("Encrypt");
  }

  wasRepaired(): boolean {
    return this.repaired;
  }

  /** `x.y` from the `%PDF-x.y` header, if present in the first 1024 bytes. */
  headerVersion(): string | undefined {
    const i = indexOf(this.bytes.subarray(0, 1024), PDF_HEADER);
    if (i < 0) return undefined;
    let s = "";
    for (let p = i + 5; p < this.bytes.length && p < i + 12; p++) {
      const b = this.bytes[p]!;
      if (!(isDigit(b) || b === 0x2e)) break;
      s += String.fromCharCode(b);
    }
    return s || undefined;
  }

  /** Offset of the newest xref section; undefined for repaired documents. */
  startxrefOffset(): number | undefined {
    return this.startxref;
  }

  /** In-use object numbers, ascending. */
  objectNumbers(): number[] {
    return [...this.xref].filter(([, e]) => e.type !== "free").map(([n]) => n).sort((a, b) => a - b);
  }

  xrefEntry(num: number): XrefEntry | undefined {
    return this.xref.get(num);
  }

  /** Load object `num` (direct object; streams included). Missing or free → `null`. */
  getObject(num: number): PdfObject {
    const cached = this.cache.get(num);
    if (cached !== undefined) return cached;
    const e = this.xref.get(num);
    if (e === undefined || e.type === "free") return NULL;
    if (this.loading.has(num)) throw new PdfError("Cycle", `object ${num} depends on itself while loading`);
    this.loading.add(num);
    try {
      const obj = e.type === "offset" ? this.loadAtOffset(num, e.offset) : this.loadCompressed(num, e.stream, e.index);
      this.cache.set(num, obj);
      return obj;
    } finally {
      this.loading.delete(num);
    }
  }

  /** Decode a stream's data through its /Filter chain (Flate + predictors supported). */
  decodeStream(s: PdfStream): Uint8Array {
    const filters = this.listOf(s.dict.entries.get("Filter"));
    const parms = this.listOf(s.dict.entries.get("DecodeParms") ?? s.dict.entries.get("DP"));
    let data = s.data;
    for (let i = 0; i < filters.length; i++) {
      const f = filters[i]!;
      if (f.kind !== "name") throw new PdfError("Filter", "filter is not a name");
      const p = parms[i];
      const pd = p !== undefined && p.kind === "dict" ? p : undefined;
      switch (f.name) {
        case "FlateDecode":
        case "Fl":
          data = flateDecode(data, this.maxDecoded);
          if (pd !== undefined) data = this.applyPredictor(data, pd);
          break;
        default:
          throw new PdfError("UnsupportedFilter", `/${f.name}`);
      }
      if (data.length > this.maxDecoded) throw new PdfError("LimitExceeded", "decoded stream too large");
    }
    return data;
  }

  /** Decoded content of a page (multiple /Contents streams joined with a newline). */
  pageContents(page: Page): Uint8Array {
    const c = this.resolve(page.dict.entries.get("Contents") ?? NULL);
    const streams: PdfStream[] = [];
    if (c.kind === "stream") streams.push(c);
    else if (c.kind === "array") {
      for (const item of c.items) {
        const s = this.resolve(item);
        if (s.kind === "stream") streams.push(s);
      }
    }
    const parts = streams.map((s) => this.decodeStream(s));
    let len = Math.max(0, parts.length - 1);
    for (const p of parts) len += p.length;
    const out = new Uint8Array(len);
    let o = 0;
    parts.forEach((p, i) => {
      if (i > 0) out[o++] = 0x0a;
      out.set(p, o);
      o += p.length;
    });
    return out;
  }

  // -------------------------------------------------------------------------
  // Xref loading (normal path)

  private loadXref(): boolean {
    const d = this.bytes;
    const sx = lastIndexOf(d, STARTXREF);
    if (sx < 0) return false;
    const lex = new Lexer(d, sx + STARTXREF.length);
    const t = lex.next();
    if (t.t !== "int") return false;
    const start = t.v;
    const visited = new Set<number>();
    let offset: number | undefined = start;
    let revisions = 0;
    let newestTrailer: PdfDict | undefined;
    const merged = new Map<number, XrefEntry>();
    const mergeIn = (entries: Map<number, XrefEntry>): void => {
      for (const [n, e] of entries) if (!merged.has(n)) merged.set(n, e);
    };
    while (offset !== undefined) {
      if (visited.has(offset)) break; // /Prev loop: stop following
      if (offset < 0 || offset >= d.length) {
        if (offset === start) return false;
        break; // /Prev outside the file: keep what we have
      }
      visited.add(offset);
      const sec = this.parseXrefSection(offset);
      revisions++;
      newestTrailer ??= sec.trailer;
      mergeIn(sec.entries);
      const stm = sec.trailer.entries.get("XRefStm");
      if (stm?.kind === "int" && !visited.has(stm.value)) {
        // Hybrid file: the table's entries win, the stream fills the gaps.
        try {
          mergeIn(this.parseXrefSection(stm.value).entries);
        } catch (e) {
          if (!(e instanceof PdfError)) throw e;
        }
      }
      const prev = sec.trailer.entries.get("Prev");
      offset = prev?.kind === "int" ? prev.value : undefined;
    }
    if (newestTrailer === undefined) return false;
    // Validate: every in-use direct entry must point at its own `n g obj`.
    for (const [n, e] of merged) {
      if (e.type !== "offset") continue;
      const h = parseObjHeader(d, e.offset);
      if (h === undefined || h.num !== n) return false;
    }
    this.xref = merged;
    this.trailerDict = newestTrailer;
    this.revisionCount = revisions;
    this.startxref = start;
    return newestTrailer.entries.has("Root");
  }

  private catalogLooksValid(): boolean {
    return this.resolve(this.trailerDict.entries.get("Root") ?? NULL).kind === "dict";
  }

  private parseXrefSection(offset: number): XrefSection {
    const d = this.bytes;
    const lex = new Lexer(d, offset);
    lex.skipWs();
    const save = lex.pos;
    const t = lex.next();
    if (t.t === "kw" && t.v === "xref") return this.parseXrefTable(lex);
    lex.pos = save;
    return this.parseXrefStream(save);
  }

  private parseXrefTable(lex: Lexer): XrefSection {
    const entries = new Map<number, XrefEntry>();
    for (;;) {
      const p = lex.pos;
      const t = lex.next();
      if (t.t === "kw" && t.v === "trailer") break;
      if (t.t !== "int") throw new PdfError("Xref", "bad xref subsection header", p);
      const c = lex.next();
      if (c.t !== "int" || c.v < 0) throw new PdfError("Xref", "bad xref subsection count", p);
      for (let i = 0; i < c.v; i++) {
        const ep = lex.pos;
        const off = lex.next();
        const gen = lex.next();
        const kind = lex.next();
        if (off.t !== "int" || gen.t !== "int" || kind.t !== "kw" || (kind.v !== "n" && kind.v !== "f")) {
          throw new PdfError("Xref", "bad xref entry", ep);
        }
        const n = t.v + i;
        if (entries.has(n)) continue;
        entries.set(n, kind.v === "n" ? { type: "offset", offset: off.v, gen: gen.v } : { type: "free" });
      }
    }
    const trailer = new Parser(lex).parseObject();
    if (trailer.kind !== "dict") throw new PdfError("Xref", "trailer is not a dictionary", lex.pos);
    return { entries, trailer };
  }

  private parseXrefStream(offset: number): XrefSection {
    const io = parseIndirectObject(this.bytes, offset, directInt);
    const s = io.obj;
    if (s.kind !== "stream" || s.dict.entries.get("Type")?.kind !== "name") {
      throw new PdfError("Xref", "startxref does not point at an xref table or stream", offset);
    }
    const ty = s.dict.entries.get("Type");
    if (ty?.kind !== "name" || ty.name !== "XRef") throw new PdfError("Xref", "not an /XRef stream", offset);
    const entries = this.xrefStreamEntries(s);
    // The xref stream's own number must be known even if its row is missing.
    if (!entries.has(io.num)) entries.set(io.num, { type: "offset", offset, gen: io.gen });
    return { entries, trailer: s.dict };
  }

  private xrefStreamEntries(s: PdfStream): Map<number, XrefEntry> {
    const e = s.dict.entries;
    const w = e.get("W");
    const size = e.get("Size");
    if (w?.kind !== "array" || w.items.length < 3 || size?.kind !== "int") throw new PdfError("Xref", "bad /W or /Size");
    const widths = w.items.slice(0, 3).map((x) => (x.kind === "int" && x.value >= 0 && x.value <= 8 ? x.value : -1));
    if (widths.includes(-1)) throw new PdfError("Xref", "bad /W widths");
    const [w0, w1, w2] = widths as [number, number, number];
    const rowLen = w0 + w1 + w2;
    if (rowLen === 0) throw new PdfError("Xref", "zero-width xref rows");
    const index = e.get("Index");
    const ranges: number[] = [];
    if (index?.kind === "array") {
      for (const x of index.items) {
        if (x.kind !== "int" || x.value < 0) throw new PdfError("Xref", "bad /Index");
        ranges.push(x.value);
      }
    } else ranges.push(0, size.value);
    const data = this.decodeStream(s);
    const entries = new Map<number, XrefEntry>();
    let p = 0;
    const field = (width: number): number => {
      let v = 0;
      for (let i = 0; i < width; i++) v = v * 256 + data[p++]!;
      return v;
    };
    for (let r = 0; r + 1 < ranges.length; r += 2) {
      const first = ranges[r]!;
      const count = ranges[r + 1]!;
      for (let i = 0; i < count && p + rowLen <= data.length; i++) {
        const type = w0 === 0 ? 1 : field(w0);
        const f2 = field(w1);
        const f3 = field(w2);
        const n = first + i;
        if (entries.has(n)) continue;
        if (type === 0) entries.set(n, { type: "free" });
        else if (type === 1) entries.set(n, { type: "offset", offset: f2, gen: w2 === 0 ? 0 : f3 });
        else if (type === 2) entries.set(n, { type: "compressed", stream: f2, index: f3 });
      }
    }
    return entries;
  }

  // -------------------------------------------------------------------------
  // Repair: rebuild the xref by scanning for `n g obj`

  private repair(): void {
    const d = this.bytes;
    const starts = scanObjHeaders(d);
    this.xref = new Map();
    this.cache.clear();
    this.objStms.clear();
    this.offsetsSorted = undefined;
    this.repaired = true;
    this.startxref = undefined;
    // Parse candidates in file order. Each parse is bounded by the next candidate, and
    // candidates that fall inside an object just parsed (e.g. `1 0 obj` text in stream
    // data) are ignored. Later definitions of the same number win.
    const found = new Map<number, XrefEntry>();
    const parsed = new Map<number, PdfObject>();
    let skipUntil = 0;
    for (let k = 0; k < starts.length; k++) {
      const start = starts[k]!;
      if (start < skipUntil) continue;
      try {
        const io = parseIndirectObject(d, start, this.lengthResolver, starts[k + 1] ?? d.length);
        found.set(io.num, { type: "offset", offset: start, gen: io.gen });
        parsed.set(io.num, io.obj);
        skipUntil = io.end;
      } catch (e) {
        if (!(e instanceof PdfError)) throw e;
      }
    }
    this.xref = found;
    this.cache = parsed;
    this.offsetsSorted = undefined;

    const candidates: PdfDict[] = [];
    // `trailer` keywords: only the last few matter (the newest with /Root is preferred).
    const trailerAt: number[] = [];
    for (let i = indexOf(d, TRAILER); i >= 0; i = indexOf(d, TRAILER, i + TRAILER.length)) trailerAt.push(i);
    for (const i of trailerAt.slice(-MAX_TRAILER_CANDIDATES)) {
      try {
        const t = new Parser(new Lexer(d, i + TRAILER.length)).parseObject();
        if (t.kind === "dict") candidates.push(t);
      } catch (e) {
        if (!(e instanceof PdfError)) throw e;
      }
    }
    // XRef streams (trailer candidates) and object streams (type-2 members).
    let catalog: number | undefined;
    for (const [n, o] of [...parsed].sort((a, b) => a[0] - b[0])) {
      const dd = o.kind === "stream" ? o.dict : o.kind === "dict" ? o : undefined;
      const type = dd?.entries.get("Type");
      const tn = type?.kind === "name" ? type.name : undefined;
      if (tn === "XRef" && o.kind === "stream") candidates.push(o.dict);
      else if (tn === "Catalog") catalog ??= n;
      else if (tn === "ObjStm" && o.kind === "stream") {
        try {
          const cache = this.objStm(n);
          cache.pairs.forEach(([num], index) => {
            if (!this.xref.has(num)) this.xref.set(num, { type: "compressed", stream: n, index });
          });
        } catch (e) {
          if (!(e instanceof PdfError)) throw e;
        }
      }
    }
    let trailer: PdfDict | undefined;
    for (let i = candidates.length - 1; i >= 0; i--) {
      const c = candidates[i]!;
      if (c.entries.has("Root") && this.safeIsDict(c.entries.get("Root")!)) {
        trailer = c;
        break;
      }
    }
    if (trailer === undefined && catalog === undefined) {
      // Objects stored only in object streams: look there for a catalog.
      for (const [n, e] of this.xref) {
        if (e.type !== "compressed") continue;
        const o = this.safeGet(n);
        const ty = o?.kind === "dict" ? o.entries.get("Type") : undefined;
        if (ty?.kind === "name" && ty.name === "Catalog") {
          catalog = n;
          break;
        }
      }
    }
    if (trailer === undefined) {
      if (catalog === undefined) throw new PdfError("Xref", "no trailer or catalog found (repair failed)");
      const entries = new Map<string, PdfObject>();
      let size = 0;
      for (const n of this.xref.keys()) size = Math.max(size, n + 1);
      entries.set("Size", { kind: "int", value: size });
      entries.set("Root", mkRef(catalog));
      trailer = { kind: "dict", entries };
    } else {
      // Strip xref-stream-only keys so the trailer looks like a classic one.
      const entries = new Map<string, PdfObject>();
      for (const [k, v] of trailer.entries) if (!XREF_STREAM_KEYS.has(k)) entries.set(k, v);
      trailer = { kind: "dict", entries };
    }
    this.trailerDict = trailer;
    this.revisionCount = Math.max(1, candidates.length);
  }

  private safeGet(n: number): PdfObject | undefined {
    try {
      return this.getObject(n);
    } catch (e) {
      if (!(e instanceof PdfError)) throw e;
      return undefined;
    }
  }

  private safeIsDict(o: PdfObject): boolean {
    try {
      return this.resolve(o).kind === "dict";
    } catch (e) {
      if (!(e instanceof PdfError)) throw e;
      return false;
    }
  }

  // -------------------------------------------------------------------------
  // Object loading

  private readonly lengthResolver = (o: PdfObject): number | undefined => {
    if (o.kind === "int") return o.value;
    if (o.kind !== "ref") return undefined;
    try {
      const r = this.resolve(o);
      return r.kind === "int" ? r.value : undefined;
    } catch (e) {
      if (!(e instanceof PdfError)) throw e;
      return undefined;
    }
  };

  /** Offset of the next object after `offset` (bounds the parse), or EOF. */
  private nextOffsetAfter(offset: number): number {
    if (this.offsetsSorted === undefined) {
      const offs: number[] = [];
      for (const e of this.xref.values()) if (e.type === "offset") offs.push(e.offset);
      this.offsetsSorted = offs.sort((a, b) => a - b);
    }
    const i = upperBound(this.offsetsSorted, offset);
    return this.offsetsSorted[i] ?? this.bytes.length;
  }

  private loadAtOffset(num: number, offset: number): PdfObject {
    const io = parseIndirectObject(this.bytes, offset, this.lengthResolver, this.nextOffsetAfter(offset));
    if (io.num !== num) throw new PdfError("Xref", `expected object ${num}, found ${io.num}`, offset);
    return io.obj;
  }

  private objStm(stmNum: number): ObjStmCache {
    const hit = this.objStms.get(stmNum);
    if (hit !== undefined) return hit;
    const e = this.xref.get(stmNum);
    if (e?.type !== "offset") throw new PdfError("Xref", `object stream ${stmNum} is not a direct object`);
    const s = this.getObject(stmNum);
    if (s.kind !== "stream") throw new PdfError("Invalid", `object stream ${stmNum} is not a stream`);
    const n = s.dict.entries.get("N");
    const first = s.dict.entries.get("First");
    if (n?.kind !== "int" || first?.kind !== "int" || n.value < 0 || first.value < 0) {
      throw new PdfError("Invalid", "object stream lacks /N or /First");
    }
    const data = this.decodeStream(s);
    if (first.value > data.length || n.value > data.length) throw new PdfError("Invalid", "object stream header out of range");
    const lex = new Lexer(data, 0, first.value);
    const pairs: Array<readonly [number, number]> = [];
    for (let i = 0; i < n.value; i++) {
      const a = lex.next();
      const b = lex.next();
      if (a.t !== "int" || b.t !== "int") break;
      pairs.push([a.v, b.v]);
    }
    const bounds = [...new Set(pairs.map(([, off]) => off))].sort((a, b) => a - b);
    const c = { data, first: first.value, pairs, bounds };
    this.objStms.set(stmNum, c);
    return c;
  }

  private loadCompressed(num: number, stmNum: number, index: number): PdfObject {
    if (stmNum === num) throw new PdfError("Cycle", `object stream ${num} contains itself`);
    const c = this.objStm(stmNum);
    let pair = c.pairs[index];
    if (pair === undefined || pair[0] !== num) pair = c.pairs.find(([n]) => n === num);
    if (pair === undefined) return NULL;
    const start = c.first + pair[1];
    if (start > c.data.length) throw new PdfError("Invalid", "object stream offset out of range");
    const next = c.bounds[upperBound(c.bounds, pair[1])];
    const end = next === undefined ? c.data.length : Math.min(c.data.length, c.first + next);
    return new Parser(new Lexer(c.data, start, Math.max(start, end))).parseObject();
  }

  private listOf(o: PdfObject | undefined): PdfObject[] {
    if (o === undefined) return [];
    const r = this.resolve(o);
    if (r.kind === "null") return [];
    if (r.kind === "array") return r.items.map((x) => this.resolve(x));
    return [r];
  }

  private applyPredictor(data: Uint8Array, p: PdfDict): Uint8Array {
    const get = (k: string, def: number): number => {
      const v = this.resolve(p.entries.get(k) ?? NULL);
      return v.kind === "int" ? v.value : def;
    };
    const predictor = get("Predictor", 1);
    if (predictor <= 1) return data;
    return unpredict(data, {
      predictor,
      colors: get("Colors", 1),
      bitsPerComponent: get("BitsPerComponent", 8),
      columns: get("Columns", 1),
    });
  }

  // -------------------------------------------------------------------------
  // Page tree

  private walkPages(): Page[] {
    const root = this.catalog();
    const top = root.entries.get("Pages");
    if (top === undefined) throw new PdfError("Invalid", "catalog has no /Pages");
    const out: Page[] = [];
    const visited = new Set<number>();
    interface Inherited {
      mediaBox: Rect | undefined;
      cropBox: Rect | undefined;
      rotate: number | undefined;
      resources: PdfDict | undefined;
    }
    // Explicit stack (reverse-pushed kids) to keep document order without recursion.
    const stack: Array<{ item: PdfObject; inh: Inherited; depth: number }> = [
      { item: top, inh: { mediaBox: undefined, cropBox: undefined, rotate: undefined, resources: undefined }, depth: 0 },
    ];
    while (stack.length > 0) {
      const { item, inh, depth } = stack.pop()!;
      if (depth > 1024) throw new PdfError("DepthLimit", "page tree too deep");
      let ref: PdfRef | undefined;
      if (item.kind === "ref") {
        if (visited.has(item.num)) throw new PdfError("Cycle", `page tree revisits object ${item.num}`);
        visited.add(item.num);
        ref = item;
      }
      // The root must resolve; below it, unreadable kids are skipped like any other junk.
      const d = depth === 0 ? this.resolve(item) : this.lenient(item);
      if (d.kind !== "dict") continue;
      const e = d.entries;
      const here: Inherited = {
        mediaBox: this.rect(e.get("MediaBox")) ?? inh.mediaBox,
        cropBox: this.rect(e.get("CropBox")) ?? inh.cropBox,
        rotate: this.intOf(e.get("Rotate")) ?? inh.rotate,
        resources: this.dictOf(e.get("Resources")) ?? inh.resources,
      };
      const type = e.get("Type");
      const kids = this.lenient(e.get("Kids") ?? NULL);
      const isNode = type?.kind === "name" ? type.name === "Pages" : kids.kind === "array";
      if (isNode) {
        if (kids.kind !== "array") continue;
        for (let i = kids.items.length - 1; i >= 0; i--) stack.push({ item: kids.items[i]!, inh: here, depth: depth + 1 });
      } else {
        out.push({ ref, dict: d, mediaBox: here.mediaBox, cropBox: here.cropBox, rotate: here.rotate ?? 0, resources: here.resources });
      }
    }
    return out;
  }

  /** Resolve, mapping a broken object to `null`. Reference cycles still throw. */
  private lenient(o: PdfObject): PdfObject {
    try {
      return this.resolve(o);
    } catch (e) {
      if (!(e instanceof PdfError) || e.code === "Cycle") throw e;
      return NULL;
    }
  }

  private rect(o: PdfObject | undefined): Rect | undefined {
    if (o === undefined) return undefined;
    const a = this.lenient(o);
    if (a.kind !== "array" || a.items.length !== 4) return undefined;
    const nums = (a as PdfArray).items.map((x) => {
      const v = this.lenient(x);
      return v.kind === "int" || v.kind === "real" ? v.value : Number.NaN;
    });
    if (nums.some(Number.isNaN)) return undefined;
    return nums as unknown as Rect;
  }

  private intOf(o: PdfObject | undefined): number | undefined {
    if (o === undefined) return undefined;
    const v = this.lenient(o);
    return v.kind === "int" ? v.value : undefined;
  }

  private dictOf(o: PdfObject | undefined): PdfDict | undefined {
    if (o === undefined) return undefined;
    const v = this.lenient(o);
    return v.kind === "dict" ? v : undefined;
  }
}

const OBJ = ascii("obj");
const MAX_TRAILER_CANDIDATES = 32;

/** Index of the first element > x in a sorted array. */
function upperBound(a: readonly number[], x: number): number {
  let lo = 0;
  let hi = a.length;
  while (lo < hi) {
    const mid = (lo + hi) >> 1;
    if (a[mid]! <= x) lo = mid + 1;
    else hi = mid;
  }
  return lo;
}

/** Offsets of every `num gen obj` header (the keyword `obj`, walked back over two integers). */
function scanObjHeaders(d: Uint8Array): number[] {
  const out: number[] = [];
  for (let i = indexOf(d, OBJ); i >= 0; i = indexOf(d, OBJ, i + 3)) {
    if (!(i + 3 >= d.length || isWhitespace(d[i + 3]) || isDelimiter(d[i + 3]))) continue;
    let p = i - 1;
    if (p < 0 || !isWhitespace(d[p])) continue;
    while (p >= 0 && isWhitespace(d[p])) p--;
    const genEnd = p + 1;
    while (p >= 0 && isDigit(d[p])) p--;
    if (p + 1 === genEnd || genEnd - (p + 1) > 5 || p < 0 || !isWhitespace(d[p])) continue;
    while (p >= 0 && isWhitespace(d[p])) p--;
    const numEnd = p + 1;
    while (p >= 0 && isDigit(d[p])) p--;
    if (p + 1 === numEnd || numEnd - (p + 1) > 10) continue;
    if (p >= 0 && !(isWhitespace(d[p]) || isDelimiter(d[p]))) continue;
    out.push(p + 1);
  }
  return out;
}

/** /Length resolver used before the xref exists: direct integers only. */
function directInt(o: PdfObject): number | undefined {
  return o.kind === "int" ? o.value : undefined;
}
