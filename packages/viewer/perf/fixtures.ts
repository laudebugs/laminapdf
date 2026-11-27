// The harness's PDFs, built with @laminapdf/sdk from fixed seeds and cached under perf/.fixtures/
// by a hash of this file, so each is built once until its recipe changes:
// - `ten`: 10 pages of text with one image each, about 1 MB (first page, zoom);
// - `hundred`: 100 pages of text (an edit on screen, with `ten` and `thousand`);
// - `thousand`: 1,000 pages of text (the scroll);
// - `images`: 300 pages, each one full-page image (the fling);
// - `paper`: 2 pages citing [1]–[24], linked to their references on 2 pages of two columns (link previews);
// - `marked`: `hundred` with a filled square annotation on every page (an annotation hidden during a drag);
// - `mb10`, `mb100`: 40 and 100 pages of text with one noise image each, about 10 and 100 MB (the
//   touched-page reload after a content edit, with `ten` as the 1 MB file; E6f);
// - `logo.png`: the small picture that edit places on a page (not a PDF, so not in FIXTURES).
// The cache key also hashes the WebAssembly module that writes them, so a builder change rebuilds
// them after `cargo xtask wasm`.

import { Buffer } from "node:buffer";
import { createHash } from "node:crypto";
import { existsSync, mkdirSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { createRequire } from "node:module";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import { crc32, deflateSync } from "node:zlib";
import { Builder, Document, Font, Image, init, type Op } from "@laminapdf/sdk";
import { RELOAD_FILES } from "./reload.ts";

/** The fixtures by name, in the order the harness builds them. */
export const FIXTURES = ["ten", "hundred", "thousand", "images", "paper", "marked"] as const;
/** The reload's files besides `ten` (reload.json), built only for a run that measures the reload. */
const LARGE = RELOAD_FILES.filter((file) => file.fixture !== "ten").map((file) => file.fixture);
export type FixtureName = (typeof FIXTURES)[number] | (typeof LARGE)[number];

const A4: [number, number] = [595, 842];
const WORDS = "page viewer render tile scroll zoom sharp memory worker canvas bitmap layout margin cache frame"
  .split(" ")
  .concat("the a of to and in for with on at by from".split(" "));

/** A small seeded generator (mulberry32), so the same seed gives the same file. */
function random(seed: number): () => number {
  let s = seed >>> 0;
  return () => {
    s = (s + 0x6d2b79f5) >>> 0;
    let t = s;
    t = Math.imul(t ^ (t >>> 15), t | 1);
    t ^= t + Math.imul(t ^ (t >>> 7), t | 61);
    return ((t ^ (t >>> 14)) >>> 0) / 2 ** 32;
  };
}

function paragraph(next: () => number, words: number): string {
  const out: string[] = [];
  for (let i = 0; i < words; i++) out.push(WORDS[Math.floor(next() * WORDS.length)] ?? "page");
  return `${out.join(" ")}.`;
}

/** An RGBA image of seeded noise: it barely compresses, so each one adds its own weight. */
function noise(next: () => number, size: number): Image {
  const pixels = new Uint8Array(size * size * 4);
  for (let i = 0; i < pixels.length; i += 4) {
    pixels[i] = next() * 256;
    pixels[i + 1] = next() * 256;
    pixels[i + 2] = next() * 256;
    pixels[i + 3] = 255;
  }
  return Image.rgba(size, size, pixels);
}

function builder(name: FixtureName): Builder {
  return new Builder({
    id: createHash("md5").update(`lamina-perf-${name}`).digest("hex"),
    producer: "lamina perf harness",
    creationDate: "D:20261105120000Z",
  });
}

function textPage(b: Builder, next: () => number, page: number, image?: Image): void {
  const font = Font.standard("Helvetica");
  const p = b.page(A4);
  const s = p.surface();
  s.text(`Page ${page + 1}`, { font, size: 18, at: [56, 72] });
  const top = image === undefined ? 96 : 280;
  if (image !== undefined) s.image(image, [56, 96, 160, 160]);
  for (let y = top, i = 0; y < 760; y += 72, i++) {
    s.textBox(paragraph(next, 40 + i * 3), { font, size: 10, box: [56, y, 483, 64] });
  }
  p.finish();
}

/** Pages 1–2 cite [1]–[24]; pages 3–4 hold the references, 6 per column, as named destinations. */
function paper(b: Builder): void {
  const font = Font.standard("Helvetica");
  const place = (n: number) => ({
    page: 2 + Math.floor((n - 1) / 12),
    x: (n - 1) % 12 < 6 ? 56 : 316,
    top: 96 + ((n - 1) % 6) * 110,
  });
  for (let n = 1; n <= 24; n++) {
    const at = place(n);
    b.destination(`cite.${n}`, at.page, { type: "xyz", left: at.x, top: at.top });
  }
  for (let p = 0; p < 4; p++) {
    const page = b.page(A4);
    const s = page.surface();
    for (let n = 1; n <= 24; n++) {
      if (p < 2 && Math.floor((n - 1) / 12) === p) {
        const y = 96 + ((n - 1) % 12) * 56;
        s.text("As shown in", { font, size: 11, at: [56, y] });
        s.text(`[${n}]`, { font, size: 11, at: [124, y], link: { target: { type: "named", name: `cite.${n}` } } });
      }
      const at = place(n);
      if (p >= 2 && at.page === p) {
        s.textBox(`[${n}] ${paragraph(random(n), 30)}`, { font, size: 9, box: [at.x, at.top, 224, 96] });
      }
    }
    page.finish();
  }
}

/** The files of text pages with a noise image each, by name: their pages and the image's side in pixels. */
const HEAVY: Partial<Record<string, readonly [pages: number, side: number]>> = Object.fromEntries(
  RELOAD_FILES.map((file) => [file.fixture, [file.pages, file.side] as const]),
);

/** The fixtures of text pages alone, by their page counts. */
const TEXT_PAGES: Partial<Record<FixtureName, number>> = { hundred: 100, thousand: 1000 };

/** `hundred` with a filled square over the text of every page: one annotation for a drag to hide. */
function marked(): Uint8Array {
  const doc = Document.open(build("hundred"));
  try {
    for (let i = 0; i < 100; i++) {
      const square = doc.shapeAt(i, "square", [56, 300, 256, 400]);
      doc.apply([{ op: "addAnnot", args: { ...square, spec: { ...square.spec, interior: [1, 0.85, 0.2] } } } as Op]);
    }
    return doc.save();
  } finally {
    doc.free();
  }
}

/** Builds fixture `name`'s bytes. */
function build(name: FixtureName): Uint8Array {
  if (name === "marked") return marked();
  const b = builder(name);
  const next = random(name.length * 7919);
  const text = TEXT_PAGES[name];
  const heavy = HEAVY[name];
  if (heavy !== undefined) {
    // Each page has an image of its own, so the file's size is the sum of theirs.
    for (let i = 0; i < heavy[0]; i++) textPage(b, next, i, noise(next, heavy[1]));
  } else if (text !== undefined) {
    for (let i = 0; i < text; i++) textPage(b, next, i);
  } else if (name === "paper") {
    paper(b);
  } else {
    const images = [0, 1, 2, 3].map(() => noise(next, 384));
    for (let i = 0; i < 300; i++) {
      const p = b.page(A4);
      const image = images[i % images.length];
      if (image !== undefined) p.surface().image(image, [0, 0, A4[0], A4[1]]);
      p.finish();
    }
  }
  const doc = b.finish();
  const bytes = doc.save();
  doc.free();
  return bytes;
}

/** The PNG the content edit places (E6f): `side` by `side` pixels of seeded noise, compressed by deflate. */
export function logoPng(side = 64): Uint8Array {
  const next = random(2026);
  const rows = Buffer.alloc((side * 3 + 1) * side);
  for (let y = 0; y < side; y++) {
    for (let i = 0; i < side * 3; i++) rows[y * (side * 3 + 1) + 1 + i] = Math.floor(next() * 256);
  }
  const chunk = (type: string, data: Buffer) => {
    const out = Buffer.alloc(12 + data.length);
    out.writeUInt32BE(data.length, 0);
    out.write(type, 4, "latin1");
    data.copy(out, 8);
    out.writeUInt32BE(crc32(out.subarray(4, 8 + data.length)), 8 + data.length);
    return out;
  };
  const header = Buffer.alloc(13);
  header.writeUInt32BE(side, 0);
  header.writeUInt32BE(side, 4);
  header.set([8, 2, 0, 0, 0], 8);
  return new Uint8Array(
    Buffer.concat([
      Buffer.from([0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a]),
      chunk("IHDR", header),
      chunk("IDAT", deflateSync(rows)),
      chunk("IEND", Buffer.alloc(0)),
    ]),
  );
}

/** The built text module, found from @laminapdf/sdk, which loads it. */
function textModule(): string {
  const sdk = fileURLToPath(import.meta.resolve("@laminapdf/sdk"));
  const wasm = createRequire(sdk).resolve("@laminapdf/wasm/package.json");
  return join(dirname(wasm), "dist/lamina_pdf_text.wasm");
}

/**
 * Writes every fixture missing from `dir` (a file: URL ending in `/`); returns `dir`. The large
 * files of the reload scenario (reload.json) are built only with `reload`: a quick run skips the
 * minutes they take. A changed recipe removes every file built by the old one.
 */
export async function ensureFixtures(dir: URL, { reload }: { readonly reload: boolean }): Promise<URL> {
  const recipe = createHash("sha256")
    .update(readFileSync(new URL(import.meta.url)))
    .update(readFileSync(new URL("reload.json", import.meta.url)))
    .update(readFileSync(textModule()))
    .digest("hex")
    .slice(0, 12);
  mkdirSync(dir, { recursive: true });
  const stamp = new URL("recipe.txt", dir);
  const everything = [...FIXTURES, ...LARGE] as FixtureName[];
  if (!(existsSync(stamp) && readFileSync(stamp, "utf8") === recipe)) {
    for (const name of everything) rmSync(new URL(`${name}.pdf`, dir), { force: true });
  }
  const wanted = reload ? everything : [...FIXTURES];
  const missing = wanted.filter((name) => !existsSync(new URL(`${name}.pdf`, dir)));
  if (missing.length > 0 || !existsSync(new URL("logo.png", dir))) {
    await init({ text: true });
    for (const name of missing) writeFileSync(new URL(`${name}.pdf`, dir), build(name));
    writeFileSync(new URL("logo.png", dir), logoPng());
  }
  writeFileSync(stamp, recipe);
  return dir;
}
