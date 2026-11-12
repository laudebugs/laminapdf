// The guide as one PDF (plugins/guide.mjs), run by the docs build after it writes the Markdown
// twins: `node scripts/guide-pdf.mjs <dist> <site>`. Reads the guide's twins from dist/, builds the
// PDF with @laminapdf/sdk's Markdown builder, writes it to dist/pdf/, then reopens it and fails on
// a PDF with no pages or no outline. qpdf, where it is installed, checks it too.
import { execFileSync } from "node:child_process";
import { mkdir, readFile, writeFile } from "node:fs/promises";
import process from "node:process";
import { pathToFileURL } from "node:url";
import { packageVersion, twinPath } from "../plugins/agent-docs.mjs";
import { GUIDE_PDF, guideAreas, guideMarkdown, guidePdf } from "../plugins/guide.mjs";

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

const { Document } = await import("@laminapdf/sdk");
const doc = Document.open(bytes);
try {
  const outline = doc.outline().length;
  if (doc.pageCount === 0 || outline === 0)
    throw new Error(`guide PDF: ${doc.pageCount} pages, ${outline} outline entries`);
  console.log(`docs: guide PDF, ${doc.pageCount} pages, ${Math.round(bytes.length / 1024)} KB`);
} finally {
  doc.free();
}
try {
  execFileSync("qpdf", ["--check", out.pathname], { stdio: "ignore" });
  console.log("docs: guide PDF passes qpdf --check");
} catch (error) {
  if (/** @type {NodeJS.ErrnoException} */ (error).code !== "ENOENT") throw new Error("guide PDF: qpdf --check failed");
}
