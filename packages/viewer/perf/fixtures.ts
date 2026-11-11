// The harness's PDFs, built with @laminapdf/sdk from fixed seeds and cached under perf/.fixtures/
// by a hash of this file, so each is built once until its recipe changes:
// - `ten`: 10 pages of text with one image each, about 1 MB (first page, zoom);
// - `hundred`: 100 pages of text (an edit on screen, with `ten` and `thousand`);
// - `thousand`: 1,000 pages of text (the scroll);
// - `images`: 300 pages, each one full-page image (the fling);
// - `paper`: 2 pages citing [1]–[24], linked to their references on 2 pages of two columns (link previews).
// The cache key also hashes the WebAssembly module that writes them, so a builder change rebuilds
// them after `cargo xtask wasm`.

import { createHash } from "node:crypto";
import { existsSync, mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { createRequire } from "node:module";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import { Builder, Font, Image, init } from "@laminapdf/sdk";

/** The fixtures by name, in the order the harness builds them. */
export const FIXTURES = ["ten", "hundred", "thousand", "images", "paper"] as const;
export type FixtureName = (typeof FIXTURES)[number];

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

/** The fixtures of text pages alone, by their page counts. */
const TEXT_PAGES: Partial<Record<FixtureName, number>> = { hundred: 100, thousand: 1000 };

/** Builds fixture `name`'s bytes. */
function build(name: FixtureName): Uint8Array {
  const b = builder(name);
  const next = random(name.length * 7919);
  const text = TEXT_PAGES[name];
  if (name === "ten") {
    for (let i = 0; i < 10; i++) textPage(b, next, i, noise(next, 184));
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

/** The built text module, found from @laminapdf/sdk, which loads it. */
function textModule(): string {
  const sdk = fileURLToPath(import.meta.resolve("@laminapdf/sdk"));
  const wasm = createRequire(sdk).resolve("@laminapdf/wasm/package.json");
  return join(dirname(wasm), "dist/lamina_pdf_text.wasm");
}

/** Writes every fixture missing from `dir` (a file: URL ending in `/`); returns `dir`. */
export async function ensureFixtures(dir: URL): Promise<URL> {
  const recipe = createHash("sha256")
    .update(readFileSync(new URL(import.meta.url)))
    .update(readFileSync(textModule()))
    .digest("hex")
    .slice(0, 12);
  mkdirSync(dir, { recursive: true });
  const stamp = new URL("recipe.txt", dir);
  const fresh = existsSync(stamp) && readFileSync(stamp, "utf8") === recipe;
  if (fresh && FIXTURES.every((name) => existsSync(new URL(`${name}.pdf`, dir)))) return dir;
  await init({ text: true });
  for (const name of FIXTURES) writeFileSync(new URL(`${name}.pdf`, dir), build(name));
  writeFileSync(stamp, recipe);
  return dir;
}
