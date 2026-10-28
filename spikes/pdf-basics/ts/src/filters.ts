import { Inflate, Unzlib, zlibSync } from "fflate";
import { concat } from "./bytes.js";
import { PdfError } from "./errors.js";

/** Default cap on the decoded size of a single stream: 64 MiB. */
export const DEFAULT_MAX_DECODED = 64 * 1024 * 1024;

/** zlib (RFC 1950) compress, as `/FlateDecode` expects. */
export function flateEncode(data: Uint8Array, level: 0 | 1 | 2 | 3 | 4 | 5 | 6 | 7 | 8 | 9 = 6): Uint8Array {
  return zlibSync(data, { level });
}

/**
 * Inflate with a hard output cap. Input is fed in small chunks so a "bomb"
 * is detected after at most ~1 MiB of overshoot rather than after allocating it all.
 * Falls back to raw deflate when the zlib header is missing/bad. A truncated stream
 * yields whatever was decoded before the data ran out (as real-world readers do);
 * it is an error only when nothing could be decoded.
 */
export function flateDecode(data: Uint8Array, maxOut = DEFAULT_MAX_DECODED): Uint8Array {
  const zlibHeader = data.length >= 2 && (data[0]! & 0x0f) === 8 && ((data[0]! << 8) | data[1]!) % 31 === 0;
  try {
    return inflateCapped(data, maxOut, zlibHeader, false);
  } catch (e) {
    if (e instanceof PdfError) throw e;
    if ((e as { code?: unknown }).code === 0) {
      // Unexpected EOF: decode again without ever signalling the end, keeping the partial output.
      let partial: Uint8Array | undefined;
      try {
        partial = inflateCapped(data, maxOut, zlibHeader, true);
      } catch (e2) {
        if (e2 instanceof PdfError) throw e2;
      }
      if (partial !== undefined && partial.length > 0) return partial;
      throw new PdfError("Filter", "FlateDecode: truncated deflate stream");
    }
    if (zlibHeader) {
      // Some writers emit a valid header but no/bad Adler-32 trailer; retry raw past the header.
      try {
        return inflateCapped(data.subarray(2), maxOut, false, false);
      } catch (e2) {
        if (e2 instanceof PdfError) throw e2;
      }
    }
    throw new PdfError("Filter", `FlateDecode: ${e instanceof Error ? e.message : String(e)}`);
  }
}

/** `partial`: feed everything as non-final chunks and return what was decoded (truncated input). */
function inflateCapped(data: Uint8Array, maxOut: number, zlib: boolean, partial: boolean): Uint8Array {
  const parts: Uint8Array[] = [];
  let total = 0;
  let done = false;
  const ondata = (chunk: Uint8Array, final: boolean): void => {
    total += chunk.length;
    if (total > maxOut) throw new PdfError("LimitExceeded", `decoded stream exceeds ${maxOut} bytes`);
    parts.push(chunk);
    if (final) done = true;
  };
  const inf = zlib ? new Unzlib(ondata) : new Inflate(ondata);
  const CHUNK = 1024;
  for (let i = 0; i < data.length && !done; i += CHUNK) {
    const end = Math.min(data.length, i + CHUNK);
    inf.push(data.subarray(i, end), !partial && end === data.length);
  }
  if (partial) return concat(parts);
  if (!done) {
    if (data.length === 0) inf.push(new Uint8Array(0), true);
    if (!done) throw new PdfError("Filter", "FlateDecode: truncated deflate stream");
  }
  return concat(parts);
}

export interface PredictorParams {
  predictor: number;
  colors: number;
  bitsPerComponent: number;
  columns: number;
}

/** Undo a TIFF (2) or PNG (10–15) predictor. Predictor 1 is a no-op. */
export function unpredict(data: Uint8Array, p: PredictorParams): Uint8Array {
  const { predictor, colors, bitsPerComponent: bpc, columns } = p;
  if (predictor === 1) return data;
  if (!(colors >= 1 && colors <= 32 && [1, 2, 4, 8, 16].includes(bpc) && columns >= 1 && columns <= 1 << 24)) {
    throw new PdfError("Filter", "invalid predictor parameters");
  }
  // Exact in doubles: at most 32 × 16 × 2^24 = 2^33 bits per row.
  const bpp = Math.max(1, Math.ceil((colors * bpc) / 8));
  const rowLen = Math.ceil((colors * bpc * columns) / 8);
  if (!Number.isSafeInteger(rowLen)) throw new PdfError("Filter", "invalid predictor parameters");
  if (predictor === 2) {
    if (bpc !== 8) throw new PdfError("UnsupportedFilter", "TIFF predictor only supported for 8 bits per component");
    const out = data.slice();
    for (let r = 0; r + rowLen <= out.length; r += rowLen) {
      for (let i = bpp; i < rowLen; i++) out[r + i] = (out[r + i]! + out[r + i - bpp]!) & 0xff;
    }
    return out;
  }
  if (predictor < 10 || predictor > 15) throw new PdfError("UnsupportedFilter", `predictor ${predictor}`);
  // A PNG row is a filter byte plus `rowLen` bytes, so a row wider than the whole input can
  // never be complete; rejecting it caps the row buffer at the input size.
  if (rowLen > data.length + 1) throw new PdfError("Filter", "predictor row length exceeds stream data");
  const stride = rowLen + 1;
  // A short final row is decoded as far as it goes.
  const rows = Math.ceil(data.length / stride);
  const out = new Uint8Array(data.length - rows);
  let prev = new Uint8Array(rowLen);
  for (let r = 0; r < rows; r++) {
    const type = data[r * stride]!;
    const src = data.subarray(r * stride + 1, r * stride + stride);
    const cur = out.subarray(r * rowLen, r * rowLen + src.length);
    for (let i = 0; i < src.length; i++) {
      const x = src[i]!;
      const a = i >= bpp ? cur[i - bpp]! : 0;
      const b = prev[i]!;
      const c = i >= bpp ? prev[i - bpp]! : 0;
      let v: number;
      switch (type) {
        case 0: v = x; break;
        case 1: v = x + a; break;
        case 2: v = x + b; break;
        case 3: v = x + ((a + b) >> 1); break;
        case 4: {
          const pa = Math.abs(b - c);
          const pb = Math.abs(a - c);
          const pc = Math.abs(a + b - 2 * c);
          v = x + (pa <= pb && pa <= pc ? a : pb <= pc ? b : c);
          break;
        }
        default:
          throw new PdfError("Filter", `bad PNG filter type ${type}`);
      }
      cur[i] = v & 0xff;
    }
    prev = cur;
  }
  return out;
}
