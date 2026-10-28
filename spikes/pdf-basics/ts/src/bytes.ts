/** Growable byte buffer used by the serializer and writers. */
export class ByteSink {
  private buf: Uint8Array;
  length = 0;

  constructor(capacity = 1024) {
    this.buf = new Uint8Array(capacity);
  }

  private reserve(n: number): void {
    const need = this.length + n;
    if (need <= this.buf.length) return;
    let cap = this.buf.length * 2;
    while (cap < need) cap *= 2;
    const next = new Uint8Array(cap);
    next.set(this.buf.subarray(0, this.length));
    this.buf = next;
  }

  byte(b: number): this {
    this.reserve(1);
    this.buf[this.length++] = b;
    return this;
  }

  /** Append a binary string (one byte per char). */
  str(s: string): this {
    this.reserve(s.length);
    for (let i = 0; i < s.length; i++) this.buf[this.length++] = s.charCodeAt(i) & 0xff;
    return this;
  }

  bytes(b: Uint8Array): this {
    this.reserve(b.length);
    this.buf.set(b, this.length);
    this.length += b.length;
    return this;
  }

  /** Copy of the contents. */
  toBytes(): Uint8Array {
    return this.buf.slice(0, this.length);
  }
}

/** First index of `needle` in `hay` at or after `from`, or -1. */
export function indexOf(hay: Uint8Array, needle: Uint8Array, from = 0): number {
  const n = needle.length;
  if (n === 0) return from;
  const first = needle[0];
  outer: for (let i = Math.max(0, from); i + n <= hay.length; i++) {
    if (hay[i] !== first) continue;
    for (let j = 1; j < n; j++) if (hay[i + j] !== needle[j]) continue outer;
    return i;
  }
  return -1;
}

/** Last index of `needle` in `hay` starting at or before `from`, or -1. */
export function lastIndexOf(hay: Uint8Array, needle: Uint8Array, from = hay.length - needle.length): number {
  const n = needle.length;
  outer: for (let i = Math.min(from, hay.length - n); i >= 0; i--) {
    for (let j = 0; j < n; j++) if (hay[i + j] !== needle[j]) continue outer;
    return i;
  }
  return -1;
}

export function concat(parts: readonly Uint8Array[]): Uint8Array {
  let len = 0;
  for (const p of parts) len += p.length;
  const out = new Uint8Array(len);
  let o = 0;
  for (const p of parts) {
    out.set(p, o);
    o += p.length;
  }
  return out;
}

export function ascii(s: string): Uint8Array {
  const out = new Uint8Array(s.length);
  for (let i = 0; i < s.length; i++) out[i] = s.charCodeAt(i);
  return out;
}
