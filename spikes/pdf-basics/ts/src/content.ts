import { ByteSink } from "./bytes.js";
import { PdfError } from "./errors.js";
import { formatFixed, formatReal } from "./format.js";
import { latin1 } from "./objects.js";
import { writeLiteralString, writeName } from "./serialize.js";

/** A number with an explicit count of fractional digits (Python `f"{v:.{digits}f}"`). */
export interface Fixed {
  readonly value: number;
  readonly digits: number;
}
export const fixed = (value: number, digits: number): Fixed => ({ value, digits });

/** Numeric operand: plain numbers use the real-number rule; {@link Fixed} uses its precision. */
export type Num = number | Fixed;

export function formatNum(n: Num): string {
  return typeof n === "number" ? formatReal(n) : formatFixed(n.value, n.digits);
}

/** κ = 4(√2 − 1)/3, the cubic Bézier circle constant (same expression as the Python fixture). */
export const KAPPA = (4 * (Math.sqrt(2) - 1)) / 3;

function nameStr(n: string): string {
  const s = new ByteSink(n.length + 1);
  writeName(s, n);
  return binaryOf(s.toBytes());
}

function stringStr(text: string | Uint8Array): string {
  const s = new ByteSink(16);
  writeLiteralString(s, typeof text === "string" ? latin1(text) : text);
  return binaryOf(s.toBytes());
}

function binaryOf(b: Uint8Array): string {
  let r = "";
  for (const x of b) r += String.fromCharCode(x);
  return r;
}

/**
 * Builds content-stream text. Each operator goes on its own line unless grouped
 * with {@link ContentBuilder.inline}. Text is kept as a binary string (one char per byte).
 */
export class ContentBuilder {
  private readonly lines: string[] = [];
  private group: string[] | undefined;

  /** Emit `operands… op`. */
  op(operator: string, ...operands: Num[]): this {
    const parts = operands.map(formatNum);
    parts.push(operator);
    return this.emit(parts.join(" "));
  }

  /** Emit an already-formatted line (or a fragment, inside {@link inline}). */
  raw(text: string): this {
    return this.emit(text);
  }

  private emit(s: string): this {
    if (this.group !== undefined) this.group.push(s);
    else this.lines.push(s);
    return this;
  }

  /** Put every operator emitted by `fn` on one line, separated by spaces. */
  inline(fn: (b: this) => void): this {
    if (this.group !== undefined) throw new PdfError("Invalid", "inline groups do not nest");
    const g: string[] = [];
    this.group = g;
    try {
      fn(this);
    } finally {
      this.group = undefined;
    }
    return this.emit(g.join(" "));
  }

  comment(text: string): this {
    if (/[\r\n]/.test(text)) throw new PdfError("Invalid", "comment contains a line break");
    return this.emit(`% ${text}`);
  }

  // Graphics state
  q(): this { return this.op("q"); }
  Q(): this { return this.op("Q"); }
  cm(a: Num, b: Num, c: Num, d: Num, e: Num, f: Num): this { return this.op("cm", a, b, c, d, e, f); }
  w(width: Num): this { return this.op("w", width); }
  J(cap: number): this { return this.op("J", cap); }
  j(join: number): this { return this.op("j", join); }
  d(dash: readonly Num[], phase: Num): this { return this.emit(`[${dash.map(formatNum).join(" ")}] ${formatNum(phase)} d`); }
  gs(name: string): this { return this.emit(`${nameStr(name)} gs`); }
  // Colour
  RG(r: Num, g: Num, b: Num): this { return this.op("RG", r, g, b); }
  rg(r: Num, g: Num, b: Num): this { return this.op("rg", r, g, b); }
  // Path construction & painting
  m(x: Num, y: Num): this { return this.op("m", x, y); }
  l(x: Num, y: Num): this { return this.op("l", x, y); }
  c(x1: Num, y1: Num, x2: Num, y2: Num, x3: Num, y3: Num): this { return this.op("c", x1, y1, x2, y2, x3, y3); }
  h(): this { return this.op("h"); }
  re(x: Num, y: Num, width: Num, height: Num): this { return this.op("re", x, y, width, height); }
  S(): this { return this.op("S"); }
  f(): this { return this.op("f"); }
  B(): this { return this.op("B"); }
  W(): this { return this.op("W"); }
  n(): this { return this.op("n"); }
  sh(name: string): this { return this.emit(`${nameStr(name)} sh`); }
  // Text
  BT(): this { return this.op("BT"); }
  ET(): this { return this.op("ET"); }
  Tf(font: string, size: Num): this { return this.emit(`${nameStr(font)} ${formatNum(size)} Tf`); }
  Td(tx: Num, ty: Num): this { return this.op("Td", tx, ty); }
  TL(leading: Num): this { return this.op("TL", leading); }
  Tj(text: string | Uint8Array): this { return this.emit(`${stringStr(text)} Tj`); }
  /** The `'` operator: next line, then show text. */
  quote(text: string | Uint8Array): this { return this.emit(`${stringStr(text)} '`); }

  /**
   * Closed circle as four Béziers (`m`, 4×`c`, `h`), every coordinate with 3 decimals —
   * byte-compatible with `examples/02-imaging-model/make_imaging_demo.py`.
   */
  circle(cx: number, cy: number, r: number): this {
    const k = KAPPA * r;
    const F = (v: number): Fixed => fixed(v, 3);
    return this.m(F(cx + r), F(cy))
      .c(F(cx + r), F(cy + k), F(cx + k), F(cy + r), F(cx), F(cy + r))
      .c(F(cx - k), F(cy + r), F(cx - r), F(cy + k), F(cx - r), F(cy))
      .c(F(cx - r), F(cy - k), F(cx - k), F(cy - r), F(cx), F(cy - r))
      .c(F(cx + k), F(cy - r), F(cx + r), F(cy - k), F(cx + r), F(cy))
      .h();
  }

  /** Lines joined by `\n`; `trailingNewline` adds a final `\n`. */
  toString(trailingNewline = false): string {
    if (this.group !== undefined) throw new PdfError("Invalid", "unterminated inline group");
    const s = this.lines.join("\n");
    return trailingNewline ? `${s}\n` : s;
  }

  toBytes(trailingNewline = false): Uint8Array {
    return latin1(this.toString(trailingNewline));
  }
}
