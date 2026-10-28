import { ascii, indexOf } from "./bytes.js";
import { PdfError } from "./errors.js";
import { isRegular, Lexer, type Token } from "./lexer.js";
import type { PdfDict, PdfObject, PdfRef, PdfStream } from "./objects.js";
import { NULL } from "./objects.js";

/** Maximum array/dict nesting depth accepted by the parser. */
export const MAX_DEPTH = 256;

const ENDSTREAM = ascii("endstream");

export class Parser {
  readonly lex: Lexer;
  private peeked: Token | undefined;

  constructor(lex: Lexer) {
    this.lex = lex;
  }

  next(): Token {
    const t = this.peeked;
    if (t !== undefined) {
      this.peeked = undefined;
      return t;
    }
    return this.lex.next();
  }

  peek(): Token {
    this.peeked ??= this.lex.next();
    return this.peeked;
  }

  /** Byte position of the next unread token's start (approximate when peeked). */
  get pos(): number {
    return this.lex.pos;
  }

  /** Parse one direct object (refs allowed, streams not). */
  parseObject(depth = 0): PdfObject {
    const start = this.lex.pos;
    const t = this.next();
    return this.fromToken(t, depth, start);
  }

  private fromToken(t: Token, depth: number, start: number): PdfObject {
    switch (t.t) {
      case "int": {
        if (t.v >= 0 && this.peeked === undefined) {
          const r = this.tryRef(t.v);
          if (r !== undefined) return r;
        }
        return { kind: "int", value: t.v };
      }
      case "real":
        return { kind: "real", value: t.v };
      case "name":
        return { kind: "name", name: t.v };
      case "str":
        return { kind: "string", bytes: t.v, hex: t.hex };
      case "[": {
        if (depth + 1 > MAX_DEPTH) throw new PdfError("DepthLimit", `nesting deeper than ${MAX_DEPTH}`, start);
        const items: PdfObject[] = [];
        for (;;) {
          const p = this.lex.pos;
          const n = this.next();
          if (n.t === "]") return { kind: "array", items };
          if (n.t === "eof") throw new PdfError("UnexpectedEof", "unterminated array", start);
          items.push(this.fromToken(n, depth + 1, p));
        }
      }
      case "<<":
        return this.parseDictBody(depth, start);
      case "kw":
        switch (t.v) {
          case "null":
            return NULL;
          case "true":
            return { kind: "bool", value: true };
          case "false":
            return { kind: "bool", value: false };
          default:
            throw new PdfError("Syntax", `unexpected keyword '${t.v.slice(0, 32)}'`, start);
        }
      case "eof":
        throw new PdfError("UnexpectedEof", "expected an object", start);
      default:
        throw new PdfError("Syntax", `unexpected '${t.t}'`, start);
    }
  }

  private parseDictBody(depth: number, start: number): PdfDict {
    if (depth + 1 > MAX_DEPTH) throw new PdfError("DepthLimit", `nesting deeper than ${MAX_DEPTH}`, start);
    const entries = new Map<string, PdfObject>();
    for (;;) {
      const kp = this.lex.pos;
      const k = this.next();
      if (k.t === ">>") return { kind: "dict", entries };
      if (k.t === "eof") throw new PdfError("UnexpectedEof", "unterminated dictionary", start);
      if (k.t !== "name") throw new PdfError("Syntax", "dictionary key is not a name", kp);
      const vp = this.lex.pos;
      const v = this.next();
      if (v.t === ">>") {
        // Missing value before '>>': treat as null (tolerant), and close.
        entries.set(k.v, NULL);
        return { kind: "dict", entries };
      }
      if (v.t === "eof") throw new PdfError("UnexpectedEof", "unterminated dictionary", start);
      entries.set(k.v, this.fromToken(v, depth + 1, vp));
    }
  }

  /** After reading a non-negative integer, check for `gen R`; rewind if absent. */
  private tryRef(num: number): PdfRef | undefined {
    const save = this.lex.pos;
    try {
      const g = this.lex.next();
      if (g.t === "int" && g.v >= 0 && g.v <= 65535) {
        const r = this.lex.next();
        if (r.t === "kw" && r.v === "R") return { kind: "ref", num, gen: g.v };
      }
    } catch (e) {
      if (!(e instanceof PdfError)) throw e;
    }
    this.lex.pos = save;
    return undefined;
  }
}

export interface IndirectObject {
  num: number;
  gen: number;
  obj: PdfObject;
  /** Offset just past the object (after `endobj` when present). */
  end: number;
}

/** Parse the `num gen obj` header at `offset` (leading whitespace allowed). */
export function parseObjHeader(data: Uint8Array, offset: number): { num: number; gen: number; pos: number } | undefined {
  try {
    const lex = new Lexer(data, offset);
    const a = lex.next();
    const b = lex.next();
    const c = lex.next();
    if (a.t === "int" && b.t === "int" && c.t === "kw" && c.v === "obj" && a.v >= 0 && b.v >= 0) {
      return { num: a.v, gen: b.v, pos: lex.pos };
    }
  } catch (e) {
    if (!(e instanceof PdfError)) throw e;
  }
  return undefined;
}

/**
 * Parse an indirect object at `offset`. `resolveLength` turns a /Length value
 * (possibly a reference) into a number, or undefined if it cannot.
 */
export function parseIndirectObject(
  data: Uint8Array,
  offset: number,
  resolveLength: (o: PdfObject) => number | undefined,
  limit = data.length,
): IndirectObject {
  const hdr = parseObjHeader(data, offset);
  if (hdr === undefined) throw new PdfError("Syntax", "expected 'n g obj'", offset);
  // The dictionary/value part is bounded by `limit` (usually the next object's offset) so that
  // garbage cannot make every object parse run to EOF; stream data itself is not bounded.
  const lex = new Lexer(data, hdr.pos, Math.max(hdr.pos, Math.min(limit, data.length)));
  const p = new Parser(lex);
  let obj = p.parseObject();
  // Look for `stream` / `endobj` without consuming anything else.
  const after = lex.pos;
  let tok: Token;
  try {
    tok = lex.next();
  } catch (e) {
    if (!(e instanceof PdfError)) throw e;
    return { num: hdr.num, gen: hdr.gen, obj, end: after };
  }
  if (tok.t === "kw" && tok.v === "stream") {
    if (obj.kind !== "dict") throw new PdfError("Syntax", "stream without dictionary", after);
    const { stream, end } = readStreamBody(data, lex.pos, obj, resolveLength);
    obj = stream;
    lex.pos = end;
    try {
      const e = lex.next();
      if (!(e.t === "kw" && e.v === "endobj")) lex.pos = end;
    } catch (err) {
      if (!(err instanceof PdfError)) throw err;
      lex.pos = end;
    }
    return { num: hdr.num, gen: hdr.gen, obj, end: lex.pos };
  }
  if (tok.t === "kw" && tok.v === "endobj") return { num: hdr.num, gen: hdr.gen, obj, end: lex.pos };
  // Missing endobj: tolerate.
  return { num: hdr.num, gen: hdr.gen, obj, end: after };
}

/** Positions of every `endstream` in a buffer, computed once per buffer (keeps repair linear). */
const endstreamIndex = new WeakMap<Uint8Array, number[]>();

function nextEndstream(data: Uint8Array, from: number): number {
  let list = endstreamIndex.get(data);
  if (list === undefined) {
    list = [];
    for (let i = indexOf(data, ENDSTREAM); i >= 0; i = indexOf(data, ENDSTREAM, i + ENDSTREAM.length)) list.push(i);
    endstreamIndex.set(data, list);
  }
  let lo = 0;
  let hi = list.length;
  while (lo < hi) {
    const mid = (lo + hi) >> 1;
    if (list[mid]! < from) lo = mid + 1;
    else hi = mid;
  }
  return lo < list.length ? list[lo]! : -1;
}

function hasEndstreamAt(data: Uint8Array, pos: number): boolean {
  let p = pos;
  while (p < data.length && (data[p] === 0x0d || data[p] === 0x0a || data[p] === 0x20 || data[p] === 0x09)) p++;
  if (p + ENDSTREAM.length > data.length) return false;
  for (let i = 0; i < ENDSTREAM.length; i++) if (data[p + i] !== ENDSTREAM[i]) return false;
  return !isRegular(data[p + ENDSTREAM.length]);
}

/** `pos` is just after the `stream` keyword. Returns the stream and the offset after `endstream`. */
function readStreamBody(
  data: Uint8Array,
  pos: number,
  dict: PdfDict,
  resolveLength: (o: PdfObject) => number | undefined,
): { stream: PdfStream; end: number } {
  let p = pos;
  // Spaces/tabs before the EOL are tolerated; the EOL is CRLF or LF (a lone CR too).
  while (data[p] === 0x20 || data[p] === 0x09) p++;
  if (data[p] === 0x0d && data[p + 1] === 0x0a) p += 2;
  else if (data[p] === 0x0a || data[p] === 0x0d) p += 1;
  const lenObj = dict.entries.get("Length");
  const len = lenObj === undefined ? undefined : resolveLength(lenObj);
  if (len !== undefined && Number.isSafeInteger(len) && len >= 0 && p + len <= data.length && hasEndstreamAt(data, p + len)) {
    const end = nextEndstream(data, p + len) + ENDSTREAM.length;
    return { stream: { kind: "stream", dict, data: data.slice(p, p + len) }, end };
  }
  // /Length missing or wrong: scan for `endstream` and drop the EOL before it.
  const idx = nextEndstream(data, p);
  if (idx < 0) throw new PdfError("UnexpectedEof", "stream without endstream", pos);
  let e = idx;
  if (e > p && data[e - 1] === 0x0a) e--;
  if (e > p && data[e - 1] === 0x0d) e--;
  return { stream: { kind: "stream", dict, data: data.slice(p, e) }, end: idx + ENDSTREAM.length };
}

/** Parse a single object from bytes (e.g. `<< /A 1 >>`). Trailing content is ignored. */
export function parseObject(bytes: Uint8Array): PdfObject {
  return new Parser(new Lexer(bytes)).parseObject();
}
