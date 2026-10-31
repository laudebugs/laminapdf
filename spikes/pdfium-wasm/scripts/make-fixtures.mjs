// Writes the spike's reference files into out/public/fixtures/ (gitignored; served by Vite at
// /fixtures/) with @lamina/pdf's builder, plus copies of the repository fixtures it reads:
//
// - ten-page.pdf: 10 text pages with a picture each, about 1 MB (the web plan §4 file).
// - thousand-page.pdf: 1,000 text pages (Noto Sans, embedded and subset).
// - scan-100mb.pdf: image-heavy, about 100 MB: one 1,600 × 1,200 noise image per page (a worst
//   case for size: noise doesn't compress; a real scan's JPEGs are smaller per page).
// - reload-50mb.pdf, reload-500mb.pdf: the same kind of pages, for the reload measurement (the
//   1 MB input is ten-page.pdf).
// - corpus/: fixtures/corpus/*.pdf and the golden examples, the render-time sample.
// - encrypted-r6.pdf: fixtures/encrypted/r6-aes-256-user.pdf (AES-256, user password `Pässwört-6`);
//   encrypted-r4.pdf: r4-aes-128-user.pdf (AES-128, `user-pw`).
//
//   node scripts/make-fixtures.mjs [--skip-500]
import { copyFileSync, mkdirSync, readdirSync, readFileSync, statSync, writeFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import { Builder, Font, Image, init } from "@lamina/pdf";

const here = dirname(dirname(fileURLToPath(import.meta.url)));
const root = join(here, "..", "..");
const out = join(here, "out", "public", "fixtures");
mkdirSync(join(out, "corpus"), { recursive: true });

await init({ text: true });
const noto = readFileSync(join(root, "fixtures/fonts/NotoSans-Regular.ttf"));

const PARAGRAPH =
  "The quick brown fox jumps over the lazy dog. Pack my box with five dozen liquor jugs. " +
  "Sphinx of black quartz, judge my vow. How vexingly quick daft zebras jump. ";

/** A builder with fixed metadata, so the files are the same on every run. */
function builder(name) {
  return new Builder({
    id: new TextEncoder().encode(`c1b-${name}`.padEnd(16, "-").slice(0, 16)),
    producer: "lamina spike pdfium-wasm",
    creationDate: "D:20261030120000Z",
  });
}

function save(name, b) {
  const doc = b.finish();
  try {
    const bytes = doc.save();
    writeFileSync(join(out, name), bytes);
    console.log(`${name}: ${(bytes.length / 1e6).toFixed(1)} MB`);
  } finally {
    doc.free();
  }
}

/** A text page: a heading and a column of paragraphs. */
function textPage(b, font, n, total) {
  const page = b.page("A4");
  const s = page.surface();
  s.text(`Page ${n} of ${total}`, { font, size: 18, at: [56, 72] });
  s.textBox(PARAGRAPH.repeat(18), { font, size: 10, box: [56, 96, 483, 690] });
  return { page, s };
}

// 10 pages, ~1 MB: text plus a 200 × 160 noise picture per page.
{
  const b = builder("ten");
  const font = Font.fromBytes(noto);
  for (let n = 1; n <= 10; n++) {
    const { page, s } = textPage(b, font, n, 10);
    s.image(noise(200, 160, n), [56, 500, 300, 240]);
    page.finish();
  }
  save("ten-page.pdf", b);
}

// 1,000 text pages.
{
  const b = builder("thousand");
  const font = Font.fromBytes(noto);
  for (let n = 1; n <= 1000; n++) textPage(b, font, n, 1000).page.finish();
  save("thousand-page.pdf", b);
}

/** A width × height noise image, different for each seed (xorshift; incompressible). */
function noise(width, height, seed) {
  const pixels = new Uint8Array(width * height * 4);
  let x = (seed * 2654435761) >>> 0 || 1;
  for (let i = 0; i < pixels.length; i += 4) {
    x ^= x << 13;
    x >>>= 0;
    x ^= x >>> 17;
    x ^= x << 5;
    x >>>= 0;
    pixels[i] = x & 0xff;
    pixels[i + 1] = (x >>> 8) & 0xff;
    pixels[i + 2] = (x >>> 16) & 0xff;
    pixels[i + 3] = 255;
  }
  return Image.rgba(width, height, pixels);
}

/** An image-heavy file of about `mb` MB: one full-page noise image per page. */
function scan(name, mb) {
  const b = builder(name);
  const perPage = 1600 * 1200 * 3; // RGB after the opaque alpha is dropped
  const pages = Math.max(1, Math.round((mb * 1e6) / perPage));
  for (let n = 1; n <= pages; n++) {
    const page = b.page("A4");
    page.surface().image(noise(1600, 1200, n), [0, 0, 595, 842]);
    page.finish();
  }
  save(`${name}.pdf`, b);
}

scan("scan-100mb", 100);
scan("reload-50mb", 50);
if (!process.argv.includes("--skip-500")) scan("reload-500mb", 500);

// The render-time sample and the encrypted file.
for (const dir of ["fixtures/corpus", "fixtures/golden/examples"]) {
  for (const f of readdirSync(join(root, dir)).filter((f) => f.endsWith(".pdf"))) {
    copyFileSync(join(root, dir, f), join(out, "corpus", `${dir.split("/").at(-1)}-${f}`));
  }
}
copyFileSync(join(root, "fixtures/encrypted/r6-aes-256-user.pdf"), join(out, "encrypted-r6.pdf"));
copyFileSync(join(root, "fixtures/encrypted/r4-aes-128-user.pdf"), join(out, "encrypted-r4.pdf"));
writeFileSync(
  join(out, "index.json"),
  `${JSON.stringify(
    readdirSync(join(out, "corpus")).sort().map((f) => ({ name: f, size: statSync(join(out, "corpus", f)).size })),
    null,
    1,
  )}\n`,
);
