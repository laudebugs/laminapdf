import { PdfError } from "./errors.js";

/** Character classes (ISO 32000-1 §7.2.2). */
const WS = 1;
const DELIM = 2;
const CLASS = new Uint8Array(256);
for (const b of [0x00, 0x09, 0x0a, 0x0c, 0x0d, 0x20]) CLASS[b] = WS;
for (const c of "()<>[]{}/%") CLASS[c.charCodeAt(0)] = DELIM;

export const isWhitespace = (b: number | undefined): boolean => b !== undefined && CLASS[b] === WS;
export const isDelimiter = (b: number | undefined): boolean => b !== undefined && CLASS[b] === DELIM;
export const isRegular = (b: number | undefined): boolean => b !== undefined && CLASS[b] === 0;
export const isDigit = (b: number | undefined): boolean => b !== undefined && b >= 0x30 && b <= 0x39;

export type Token =
  | { readonly t: "int"; readonly v: number }
  | { readonly t: "real"; readonly v: number }
  | { readonly t: "name"; readonly v: string }
  | { readonly t: "str"; readonly v: Uint8Array; readonly hex: boolean }
  | { readonly t: "kw"; readonly v: string }
  | { readonly t: "[" | "]" | "<<" | ">>" | "{" | "}" | "eof" };

const INT_RE = /^[+-]?\d+$/;
const REAL_RE = /^[+-]?(?:\d+\.\d*|\.\d+)$/;

function hexVal(b: number): number {
  if (b >= 0x30 && b <= 0x39) return b - 0x30;
  if (b >= 0x41 && b <= 0x46) return b - 0x37;
  if (b >= 0x61 && b <= 0x66) return b - 0x57;
  return -1;
}

/** Tokenizer over a byte array. Always makes progress; never reads out of bounds. */
export class Lexer {
  readonly data: Uint8Array;
  pos: number;
  readonly end: number;

  constructor(data: Uint8Array, pos = 0, end = data.length) {
    this.data = data;
    this.pos = pos;
    this.end = end;
  }

  /** Skip whitespace and comments. */
  skipWs(): void {
    const d = this.data;
    while (this.pos < this.end) {
      const b = d[this.pos]!;
      if (CLASS[b] === WS) this.pos++;
      else if (b === 0x25) {
        while (this.pos < this.end && d[this.pos] !== 0x0a && d[this.pos] !== 0x0d) this.pos++;
      } else break;
    }
  }

  next(): Token {
    this.skipWs();
    const d = this.data;
    if (this.pos >= this.end) return { t: "eof" };
    const start = this.pos;
    const b = d[this.pos]!;
    switch (b) {
      case 0x5b:
        this.pos++;
        return { t: "[" };
      case 0x5d:
        this.pos++;
        return { t: "]" };
      case 0x7b:
        this.pos++;
        return { t: "{" };
      case 0x7d:
        this.pos++;
        return { t: "}" };
      case 0x2f:
        return this.readName();
      case 0x28:
        return this.readLiteral();
      case 0x3c:
        if (d[this.pos + 1] === 0x3c && this.pos + 1 < this.end) {
          this.pos += 2;
          return { t: "<<" };
        }
        return this.readHex();
      case 0x3e:
        if (d[this.pos + 1] === 0x3e && this.pos + 1 < this.end) {
          this.pos += 2;
          return { t: ">>" };
        }
        this.pos++;
        throw new PdfError("Syntax", "unexpected '>'", start);
      case 0x29:
        this.pos++;
        throw new PdfError("Syntax", "unbalanced ')'", start);
      default:
        break;
    }
    while (this.pos < this.end && CLASS[d[this.pos]!] === 0) this.pos++;
    let s = "";
    for (let i = start; i < this.pos; i++) s += String.fromCharCode(d[i]!);
    if (INT_RE.test(s)) {
      const v = Number(s);
      if (!Number.isSafeInteger(v)) throw new PdfError("Syntax", `integer out of range: ${s}`, start);
      return { t: "int", v };
    }
    if (REAL_RE.test(s)) return { t: "real", v: Number(s) };
    return { t: "kw", v: s };
  }

  private readName(): Token {
    const d = this.data;
    this.pos++; // '/'
    let s = "";
    while (this.pos < this.end) {
      const b = d[this.pos]!;
      if (CLASS[b] !== 0) break;
      if (b === 0x23 && this.pos + 2 < this.end) {
        const h = hexVal(d[this.pos + 1]!);
        const l = hexVal(d[this.pos + 2]!);
        if (h >= 0 && l >= 0) {
          s += String.fromCharCode((h << 4) | l);
          this.pos += 3;
          continue;
        }
      }
      s += String.fromCharCode(b);
      this.pos++;
    }
    return { t: "name", v: s };
  }

  private readLiteral(): Token {
    const d = this.data;
    const start = this.pos;
    this.pos++; // '('
    const out: number[] = [];
    let depth = 1;
    while (this.pos < this.end) {
      const b = d[this.pos++]!;
      if (b === 0x5c) {
        if (this.pos >= this.end) break;
        const e = d[this.pos++]!;
        switch (e) {
          case 0x6e: out.push(0x0a); break; // \n
          case 0x72: out.push(0x0d); break; // \r
          case 0x74: out.push(0x09); break; // \t
          case 0x62: out.push(0x08); break; // \b
          case 0x66: out.push(0x0c); break; // \f
          case 0x0d: // line continuation (CR or CRLF)
            if (this.pos < this.end && d[this.pos] === 0x0a) this.pos++;
            break;
          case 0x0a: break; // line continuation (LF)
          default:
            if (e >= 0x30 && e <= 0x37) {
              let v = e - 0x30;
              for (let k = 0; k < 2 && this.pos < this.end; k++) {
                const o = d[this.pos]!;
                if (o < 0x30 || o > 0x37) break;
                v = v * 8 + (o - 0x30);
                this.pos++;
              }
              out.push(v & 0xff);
            } else {
              out.push(e); // \( \) \\ and unknown escapes: the char itself
            }
        }
        continue;
      }
      if (b === 0x28) depth++;
      else if (b === 0x29) {
        depth--;
        if (depth === 0) return { t: "str", v: Uint8Array.from(out), hex: false };
      } else if (b === 0x0d) {
        // Unescaped EOL (CR or CRLF) inside a string reads as a single LF (§7.3.4.2).
        if (this.pos < this.end && d[this.pos] === 0x0a) this.pos++;
        out.push(0x0a);
        continue;
      }
      out.push(b);
    }
    throw new PdfError("UnexpectedEof", "unterminated literal string", start);
  }

  private readHex(): Token {
    const d = this.data;
    const start = this.pos;
    this.pos++; // '<'
    const out: number[] = [];
    let hi = -1;
    while (this.pos < this.end) {
      const b = d[this.pos++]!;
      if (b === 0x3e) {
        if (hi >= 0) out.push(hi << 4); // odd length: pad with 0
        return { t: "str", v: Uint8Array.from(out), hex: true };
      }
      if (CLASS[b] === WS) continue;
      const v = hexVal(b);
      if (v < 0) throw new PdfError("Syntax", "invalid character in hex string", this.pos - 1);
      if (hi < 0) hi = v;
      else {
        out.push((hi << 4) | v);
        hi = -1;
      }
    }
    throw new PdfError("UnexpectedEof", "unterminated hex string", start);
  }
}
