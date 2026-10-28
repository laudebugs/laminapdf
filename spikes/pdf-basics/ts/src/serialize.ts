import { ByteSink } from "./bytes.js";
import { PdfError } from "./errors.js";
import { formatInt, formatReal } from "./format.js";
import type { PdfDict, PdfObject, PdfStream } from "./objects.js";
import { int } from "./objects.js";

const HEX = "0123456789ABCDEF";

/** Bytes that must be `#xx`-escaped in a name: outside `!`..`~`, `#`, and delimiters. */
function nameNeedsEscape(b: number): boolean {
  if (b < 0x21 || b > 0x7e) return true;
  switch (b) {
    case 0x23: // #
    case 0x28: // (
    case 0x29: // )
    case 0x3c: // <
    case 0x3e: // >
    case 0x5b: // [
    case 0x5d: // ]
    case 0x7b: // {
    case 0x7d: // }
    case 0x2f: // /
    case 0x25: // %
      return true;
    default:
      return false;
  }
}

export function writeName(sink: ByteSink, n: string): void {
  sink.byte(0x2f);
  for (let i = 0; i < n.length; i++) {
    const b = n.charCodeAt(i) & 0xff;
    if (nameNeedsEscape(b)) sink.byte(0x23).byte(HEX.charCodeAt(b >> 4)).byte(HEX.charCodeAt(b & 15));
    else sink.byte(b);
  }
}

export function writeLiteralString(sink: ByteSink, bytes: Uint8Array): void {
  sink.byte(0x28);
  for (const b of bytes) {
    if (b === 0x5c || b === 0x28 || b === 0x29) sink.byte(0x5c).byte(b);
    else if (b === 0x0d) sink.byte(0x5c).byte(0x72); // \r
    else sink.byte(b);
  }
  sink.byte(0x29);
}

function writeHexString(sink: ByteSink, bytes: Uint8Array): void {
  sink.byte(0x3c);
  for (const b of bytes) sink.byte(HEX.charCodeAt(b >> 4)).byte(HEX.charCodeAt(b & 15));
  sink.byte(0x3e);
}

/** Return a copy of `d` with /Length set: kept in place if present, else appended. */
export function withLength(d: PdfDict, length: number): PdfDict {
  const entries = new Map(d.entries);
  entries.set("Length", int(length));
  return { kind: "dict", entries };
}

/**
 * Serialize one object. Streams are written as
 * `<< …dict… /Length N >>\nstream\n<bytes>\nendstream` with /Length forced to the data length.
 * Recursion is bounded by the caller's object depth; objects built by this library's parser
 * are at most 256 deep.
 */
export function writeObject(sink: ByteSink, o: PdfObject): void {
  switch (o.kind) {
    case "null":
      sink.str("null");
      return;
    case "bool":
      sink.str(o.value ? "true" : "false");
      return;
    case "int":
      sink.str(formatInt(o.value));
      return;
    case "real":
      sink.str(formatReal(o.value));
      return;
    case "string":
      if (o.hex) writeHexString(sink, o.bytes);
      else writeLiteralString(sink, o.bytes);
      return;
    case "name":
      writeName(sink, o.name);
      return;
    case "array": {
      sink.byte(0x5b);
      let first = true;
      for (const item of o.items) {
        if (!first) sink.byte(0x20);
        first = false;
        writeObject(sink, item);
      }
      sink.byte(0x5d);
      return;
    }
    case "dict":
      sink.str("<<");
      for (const [k, v] of o.entries) {
        sink.byte(0x20);
        writeName(sink, k);
        sink.byte(0x20);
        writeObject(sink, v);
      }
      sink.str(" >>");
      return;
    case "stream":
      writeStream(sink, o);
      return;
    case "ref":
      if (!Number.isSafeInteger(o.num) || o.num < 0 || !Number.isSafeInteger(o.gen) || o.gen < 0) {
        throw new PdfError("Invalid", `bad reference ${o.num} ${o.gen}`);
      }
      sink.str(`${o.num} ${o.gen} R`);
      return;
  }
}

function writeStream(sink: ByteSink, s: PdfStream): void {
  writeObject(sink, withLength(s.dict, s.data.length));
  sink.str("\nstream\n").bytes(s.data).str("\nendstream");
}

/** Serialize to bytes. */
export function serialize(o: PdfObject): Uint8Array {
  const sink = new ByteSink(64);
  writeObject(sink, o);
  return sink.toBytes();
}
