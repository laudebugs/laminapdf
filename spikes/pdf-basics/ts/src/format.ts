import { PdfError } from "./errors.js";

const f64 = new Float64Array(1);
const u32 = new Uint32Array(f64.buffer);
// Detect endianness once (u32[1] holds the high word on little-endian hosts).
const HI = new Uint8Array(new Uint16Array([1]).buffer)[0] === 1 ? 1 : 0;
const LO = 1 - HI;

/**
 * Format `x` with exactly `digits` fractional digits, matching Python's
 * `f"{x:.{digits}f}"`: the *exact* binary value is rounded half-to-even.
 * (`Number#toFixed` differs on exact ties, e.g. 0.125 → "0.13" vs Python "0.12".)
 * Never uses exponent notation. Negative zero keeps its sign, like Python.
 */
export function formatFixed(x: number, digits: number): string {
  if (!Number.isFinite(x)) throw new PdfError("Invalid", `cannot format non-finite number ${x}`);
  if (!Number.isInteger(digits) || digits < 0 || digits > 100) {
    throw new PdfError("Invalid", `bad precision ${digits}`);
  }
  f64[0] = x;
  const hi = u32[HI]!;
  const lo = u32[LO]!;
  const negative = hi >>> 31 === 1;
  const biasedExp = (hi >>> 20) & 0x7ff;
  let mant = (BigInt(hi & 0xfffff) << 32n) | BigInt(lo);
  let exp: number;
  if (biasedExp === 0) {
    exp = -1074; // subnormal
  } else {
    mant |= 1n << 52n;
    exp = biasedExp - 1075;
  }
  // value = mant * 2^exp ; want round(value * 10^digits)
  let q: bigint;
  const scale = 10n ** BigInt(digits);
  if (exp >= 0) {
    q = (mant << BigInt(exp)) * scale;
  } else {
    const sh = BigInt(-exp);
    const scaled = mant * scale;
    q = scaled >> sh;
    const rem = scaled - (q << sh);
    const half = 1n << (sh - 1n);
    if (rem > half || (rem === half && (q & 1n) === 1n)) q += 1n;
  }
  let s = q.toString();
  if (digits > 0) {
    if (s.length <= digits) s = "0".repeat(digits - s.length + 1) + s;
    s = `${s.slice(0, s.length - digits)}.${s.slice(s.length - digits)}`;
  }
  return negative ? `-${s}` : s;
}

/**
 * The spec's real-number rule: no exponent, at most 6 fractional digits,
 * trailing zeros and a trailing `.` trimmed, `-0` → `0`.
 */
export function formatReal(x: number): string {
  let s = formatFixed(x, 6);
  if (s.includes(".")) {
    let end = s.length;
    while (s.charCodeAt(end - 1) === 0x30) end--;
    if (s.charCodeAt(end - 1) === 0x2e) end--;
    s = s.slice(0, end);
  }
  return s === "-0" ? "0" : s;
}

/** Integer formatting (decimal). Rejects non-integers. */
export function formatInt(x: number): string {
  if (!Number.isSafeInteger(x)) throw new PdfError("Invalid", `not a safe integer: ${x}`);
  return Object.is(x, -0) ? "0" : String(x);
}
