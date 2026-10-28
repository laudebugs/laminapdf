import { readdirSync, readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import { expect } from "vitest";
import { type PdfDocument, PdfError } from "../src/index.js";

export const HERE = dirname(fileURLToPath(import.meta.url));
export const PKG = join(HERE, "..");
export const FIXTURES = join(PKG, "..", "fixtures");
export const CORPUS = join(FIXTURES, "corpus");

export const read = (p: string): Uint8Array => new Uint8Array(readFileSync(p));
export const golden = (f: string): Uint8Array => read(join(FIXTURES, "golden", f));
export const corpusFile = (f: string): Uint8Array => read(join(CORPUS, f));
export const corpusNames = (): string[] => readdirSync(CORPUS).filter((f) => f.endsWith(".pdf")).sort();

export interface Expected {
  size: number;
  header_version: string;
  page_count: number;
  encrypted: boolean;
  revisions: number;
  has_xref_stream: boolean;
  first_page_mediabox: number[];
  root_keys: string[];
}
export const expected = (): Record<string, Expected> =>
  JSON.parse(readFileSync(join(CORPUS, "expected.json"), "utf8")) as Record<string, Expected>;

export const latin1 = (b: Uint8Array): string => Buffer.from(b).toString("latin1");
export const bytesOf = (s: string): Uint8Array => new Uint8Array(Buffer.from(s, "latin1"));

/** mulberry32: small seeded PRNG so robustness runs are reproducible. */
export function prng(seed: number): () => number {
  let a = seed >>> 0;
  return () => {
    a = (a + 0x6d2b79f5) >>> 0;
    let t = a;
    t = Math.imul(t ^ (t >>> 15), t | 1);
    t ^= t + Math.imul(t ^ (t >>> 7), t | 61);
    return ((t ^ (t >>> 14)) >>> 0) / 4294967296;
  };
}

/** Run `fn`; it may succeed or throw, but only a PdfError is acceptable. */
export function onlyPdfErrors(fn: () => void, label: string): "ok" | "error" {
  try {
    fn();
    return "ok";
  } catch (e) {
    if (!(e instanceof PdfError)) {
      throw new Error(`${label}: non-PdfError escaped: ${e instanceof Error ? (e.stack ?? e.message) : String(e)}`);
    }
    return "error";
  }
}

/** Touch everything a consumer might: pages, every object, page contents. */
export function exercise(doc: PdfDocument): void {
  doc.trailer();
  doc.isEncrypted();
  doc.revisions();
  for (const n of doc.objectNumbers()) {
    try {
      doc.getObject(n);
    } catch (e) {
      if (!(e instanceof PdfError)) throw e;
    }
  }
  doc.catalog();
  for (const p of doc.pages()) {
    try {
      doc.pageContents(p);
    } catch (e) {
      if (!(e instanceof PdfError)) throw e;
    }
  }
}

export function expectPdfError(fn: () => unknown, code?: string): PdfError {
  try {
    fn();
  } catch (e) {
    expect(e).toBeInstanceOf(PdfError);
    if (code !== undefined) expect((e as PdfError).code).toBe(code);
    return e as PdfError;
  }
  throw new Error("expected a PdfError, but nothing was thrown");
}
