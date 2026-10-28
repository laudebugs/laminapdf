/**
 * PDF object model (ISO 32000-1 §7.3) as a discriminated union on `kind`.
 *
 * Names and dict keys are "binary strings": one JS char per byte (code unit 0..255),
 * without the leading `/`. This keeps them usable as `Map` keys while preserving
 * arbitrary bytes from `#xx` escapes.
 */

export interface PdfNull {
  readonly kind: "null";
}
export interface PdfBool {
  readonly kind: "bool";
  readonly value: boolean;
}
/** Integer. Stored as a JS number; the parser rejects integers outside the safe range. */
export interface PdfInt {
  readonly kind: "int";
  readonly value: number;
}
export interface PdfReal {
  readonly kind: "real";
  readonly value: number;
}
export interface PdfString {
  readonly kind: "string";
  readonly bytes: Uint8Array;
  /** True if written/read in `<hex>` form; false for `(literal)`. */
  readonly hex: boolean;
}
export interface PdfName {
  readonly kind: "name";
  /** Binary string, no leading slash. */
  readonly name: string;
}
export interface PdfArray {
  readonly kind: "array";
  readonly items: PdfObject[];
}
export interface PdfDict {
  readonly kind: "dict";
  /** Insertion-ordered. Keys are binary-string names without the slash. */
  readonly entries: Map<string, PdfObject>;
}
export interface PdfStream {
  readonly kind: "stream";
  readonly dict: PdfDict;
  /** Raw (still encoded) stream bytes. */
  readonly data: Uint8Array;
}
export interface PdfRef {
  readonly kind: "ref";
  readonly num: number;
  readonly gen: number;
}

export type PdfObject =
  | PdfNull
  | PdfBool
  | PdfInt
  | PdfReal
  | PdfString
  | PdfName
  | PdfArray
  | PdfDict
  | PdfStream
  | PdfRef;

export type PdfKind = PdfObject["kind"];

// ---------------------------------------------------------------------------
// Constructors

export const NULL: PdfNull = Object.freeze({ kind: "null" });

export const bool = (value: boolean): PdfBool => ({ kind: "bool", value });
export const int = (value: number): PdfInt => ({ kind: "int", value });
export const real = (value: number): PdfReal => ({ kind: "real", value });
export const name = (n: string): PdfName => ({ kind: "name", name: n });
export const ref = (num: number, gen = 0): PdfRef => ({ kind: "ref", num, gen });
export const array = (items: PdfObject[]): PdfArray => ({ kind: "array", items });

/** Literal string from a binary string (one char per byte; chars > 255 are truncated to their low byte). */
export const str = (s: string | Uint8Array): PdfString => ({
  kind: "string",
  bytes: typeof s === "string" ? latin1(s) : s,
  hex: false,
});
export const hexStr = (s: string | Uint8Array): PdfString => ({
  kind: "string",
  bytes: typeof s === "string" ? latin1(s) : s,
  hex: true,
});

/**
 * Build a dict. Pass entries as an array of pairs, a Map, or a plain object
 * (beware: plain objects reorder integer-like keys).
 */
export function dict(
  init?: ReadonlyArray<readonly [string, PdfObject]> | Map<string, PdfObject> | Record<string, PdfObject>,
): PdfDict {
  if (init === undefined) return { kind: "dict", entries: new Map() };
  if (init instanceof Map) return { kind: "dict", entries: new Map(init) };
  if (Array.isArray(init)) return { kind: "dict", entries: new Map(init as ReadonlyArray<[string, PdfObject]>) };
  return { kind: "dict", entries: new Map(Object.entries(init)) };
}

export const stream = (d: PdfDict, data: Uint8Array): PdfStream => ({ kind: "stream", dict: d, data });

/** Numbers: integer-valued → `int`, otherwise `real`. */
export const num = (v: number): PdfInt | PdfReal => (Number.isInteger(v) ? int(v) : real(v));

// ---------------------------------------------------------------------------
// Accessors (no resolution — see PdfDocument.resolve for that)

export function dictGet(d: PdfDict | PdfStream, key: string): PdfObject | undefined {
  return (d.kind === "stream" ? d.dict : d).entries.get(key);
}

export function asNumber(o: PdfObject | undefined): number | undefined {
  return o !== undefined && (o.kind === "int" || o.kind === "real") ? o.value : undefined;
}

export function asName(o: PdfObject | undefined): string | undefined {
  return o?.kind === "name" ? o.name : undefined;
}

export function isRef(o: PdfObject | undefined): o is PdfRef {
  return o?.kind === "ref";
}

// ---------------------------------------------------------------------------
// Byte helpers (no TextEncoder needed for the 8-bit world of PDF syntax)

/** Binary string → bytes (low byte of each UTF-16 code unit). */
export function latin1(s: string): Uint8Array {
  const out = new Uint8Array(s.length);
  for (let i = 0; i < s.length; i++) out[i] = s.charCodeAt(i) & 0xff;
  return out;
}

/** Bytes → binary string. */
export function fromLatin1(b: Uint8Array, start = 0, end = b.length): string {
  let s = "";
  const CHUNK = 0x2000;
  for (let i = start; i < end; i += CHUNK) {
    s += String.fromCharCode(...b.subarray(i, Math.min(end, i + CHUNK)));
  }
  return s;
}
