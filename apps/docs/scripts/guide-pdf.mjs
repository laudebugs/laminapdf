// The guide as one PDF (plugins/guide.mjs), run by the docs build after it writes the Markdown
// twins: `node scripts/guide-pdf.mjs <dist> <site>`. Reads the guide's twins from dist/, builds the
// PDF with @laminapdf/sdk's Markdown builder, writes it to dist/pdf/, then reopens it and fails on
// a PDF with no pages or no outline. qpdf, where it is installed, checks it too.
import { spawnSync } from "node:child_process";
import { mkdir, readFile, writeFile } from "node:fs/promises";
import process from "node:process";
import { fileURLToPath, pathToFileURL } from "node:url";
import { packageVersion } from "../plugins/agent-docs.mjs";
import { GUIDE_PDF, guideAreas, guideMarkdown } from "../plugins/guide.mjs";
import { twinPath } from "../plugins/markdown.mjs";

const ROOT = new URL("../../../", import.meta.url);

/** The fonts the PDF is set in, from the docs' dev dependencies and the repository's Noto fixtures. */
const FONTS = {
  regular: "apps/docs/node_modules/source-sans/OTF/SourceSans3-Regular.otf",
  bold: "apps/docs/node_modules/source-sans/OTF/SourceSans3-Bold.otf",
  italic: "apps/docs/node_modules/source-sans/OTF/SourceSans3-It.otf",
  boldItalic: "apps/docs/node_modules/source-sans/OTF/SourceSans3-BoldIt.otf",
  mono: "apps/docs/node_modules/source-code-pro/OTF/SourceCodePro-Regular.otf",
  fallbacks: [
    "fixtures/fonts/NotoSans-Regular.ttf",
    "fixtures/fonts/NotoSansArabic-Regular.ttf",
    "fixtures/fonts/NotoSansDevanagari-Regular.ttf",
    "fixtures/fonts/NotoSansHebrew-Regular.ttf",
    "fixtures/fonts/NotoSansMath-Regular.ttf",
    "fixtures/fonts/NotoSansSC-Regular.otf",
  ],
};

/**
 * @laminapdf/sdk, loaded by name: the docs typecheck runs before the SDK is built, so a typed import
 * would fail there; a name typed as `string` gives an untyped module.
 *
 * @returns {Promise<any>}
 */
function loadSdk() {
  /** @type {string} */
  const name = "@laminapdf/sdk";
  return import(name);
}

/**
 * The guide PDF's bytes, built by @laminapdf/sdk's Markdown builder under PDF/UA-1 with the
 * guide's fonts. Fails on a warning but a missing image: the twins name the docs' diagrams by
 * file, which the PDF leaves out and describes in text.
 *
 * @param {string} markdown
 * @returns {Promise<Uint8Array>}
 */
async function guidePdf(markdown) {
  const { Builder, Font, init } = await loadSdk();
  await init({ text: true });
  const read = async (/** @type {string} */ path) => new Uint8Array(await readFile(new URL(path, ROOT)));
  const fonts = {
    regular: Font.fromBytes(await read(FONTS.regular)),
    bold: Font.fromBytes(await read(FONTS.bold)),
    italic: Font.fromBytes(await read(FONTS.italic)),
    boldItalic: Font.fromBytes(await read(FONTS.boldItalic)),
    mono: Font.fromBytes(await read(FONTS.mono)),
    fallbacks: await Promise.all(FONTS.fallbacks.map(async (path) => Font.fromBytes(await read(path)))),
  };
  // `free()` rather than `using`: Node 22, which the docs build runs on, has no `using`.
  const builder = new Builder({ profiles: ["PDF/UA-1"] });
  try {
    builder.markdown(markdown, {
      fonts,
      page: {
        size: "A4",
        header: { end: "{title}" },
        footer: { start: "{section}", end: "Page {page} of {pages}" },
        furnitureSize: 8,
      },
      contents: 2,
    });
    const doc = builder.finish();
    try {
      const warnings = doc.warnings.filter((/** @type {{ code: string }} */ w) => w.code !== "markdown-image-missing");
      if (warnings.length > 0)
        throw new Error(`guide PDF: ${warnings.map((/** @type {{ code: string }} */ w) => w.code).join(", ")}`);
      return doc.save();
    } finally {
      doc.free();
    }
  } finally {
    builder.free();
  }
}

const [distPath, site] = process.argv.slice(2);
if (distPath === undefined || site === undefined) throw new Error("usage: node scripts/guide-pdf.mjs <dist> <site>");
const dist = pathToFileURL(`${distPath.replace(/\/$/, "")}/`);

const areas = guideAreas();
const twins = new Map();
for (const route of areas.flatMap((area) => area.routes)) {
  twins.set(route, await readFile(new URL(twinPath(route), dist), "utf8"));
}
const bytes = await guidePdf(guideMarkdown({ version: await packageVersion(), site, areas, twins }));
const out = new URL(GUIDE_PDF.slice(1), dist);
await mkdir(new URL(".", out), { recursive: true });
await writeFile(out, bytes);

const { Document } = await loadSdk();
const doc = Document.open(bytes);
try {
  const outline = doc.outline().length;
  if (doc.pageCount === 0 || outline === 0)
    throw new Error(`guide PDF: ${doc.pageCount} pages, ${outline} outline entries`);
  console.log(`docs: guide PDF, ${doc.pageCount} pages, ${Math.round(bytes.length / 1024)} KB`);
} finally {
  doc.free();
}
// qpdf exits 0 when the file is clean and 3 when it has only warnings; 2 is an error.
const qpdf = spawnSync("qpdf", ["--check", fileURLToPath(out)], { encoding: "utf8" });
if (qpdf.error === undefined) {
  if (qpdf.status !== 0 && qpdf.status !== 3)
    throw new Error(`guide PDF: qpdf --check failed:\n${qpdf.stdout}${qpdf.stderr}`);
  console.log(`docs: guide PDF passes qpdf --check${qpdf.status === 3 ? " with warnings" : ""}`);
} else if (/** @type {NodeJS.ErrnoException} */ (qpdf.error).code !== "ENOENT") {
  throw qpdf.error;
}
