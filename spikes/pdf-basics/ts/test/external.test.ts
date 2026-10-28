// Writes compact-mode and incremental outputs for external validation (pypdf strict).
// Enabled when PDF_OUT_DIR is set; also runs pypdf when PYPDF_PYTHON points at a Python with pypdf.
import { execFileSync } from "node:child_process";
import { existsSync, mkdirSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { describe, expect, test } from "vitest";
import { array, dict, int, name, PdfWriter, ref, str, stream, writeIncremental } from "../src/index.js";
import { golden } from "./helpers.js";
import { helloContent, helloPdf, imagingPdf } from "./recipes.js";

const OUT = process.env.PDF_OUT_DIR;
const PY = process.env.PYPDF_PYTHON;

function twoPageDoc(): PdfWriter {
  const w = new PdfWriter();
  w.add(dict([["Type", name("Catalog")], ["Pages", ref(2)]]));
  w.add(dict([["Type", name("Pages")], ["Kids", array([ref(3), ref(5)])], ["Count", int(2)], ["MediaBox", array([int(0), int(0), int(612), int(792)])]]));
  w.add(dict([["Type", name("Page")], ["Parent", ref(2)], ["Contents", ref(4)], ["Resources", dict([["Font", dict([["F1", ref(7)]])]])]]));
  w.add(stream(dict(), helloContent("Page one")));
  w.add(dict([["Type", name("Page")], ["Parent", ref(2)], ["Contents", ref(6)], ["Resources", dict([["Font", dict([["F1", ref(7)]])]])]]));
  w.add(stream(dict(), helloContent("Page two")));
  w.add(dict([["Type", name("Font")], ["Subtype", name("Type1")], ["BaseFont", name("Helvetica")]]));
  w.add(dict([["Title", str("lamina compact")]]));
  return w;
}

describe.skipIf(OUT === undefined)("external validation outputs", () => {
  const files: Array<[string, Uint8Array, number]> = [];
  const trailer = dict([["Root", ref(1)], ["Info", ref(8)]]);
  const compact = twoPageDoc().toBytes(trailer, { mode: "compact" });
  const compactZ = twoPageDoc().toBytes(trailer, { mode: "compact", compressStreams: true });
  const inc1 = writeIncremental(golden("hello.pdf"), [{ num: 5, body: stream(dict(), helloContent("Hello, Update")) }]);
  const inc2 = writeIncremental(inc1, [{ num: 6, body: dict([["Title", str("rev 3")]]) }], { trailer: dict([["Info", ref(6)]]) });
  const incOnCompact = writeIncremental(compact, [{ num: 6, body: stream(dict(), helloContent("Page two, revised")) }]);
  files.push(
    ["compact.pdf", compact, 2],
    ["compact-flate.pdf", compactZ, 2],
    ["incremental-2rev.pdf", inc1, 1],
    ["incremental-3rev.pdf", inc2, 1],
    ["incremental-on-compact.pdf", incOnCompact, 2],
    ["hello.pdf", helloPdf(), 1],
    ["imaging-demo.pdf", imagingPdf(), 1],
  );

  test("write files", () => {
    mkdirSync(OUT!, { recursive: true });
    for (const [f, b] of files) writeFileSync(join(OUT!, f), b);
  });

  test.skipIf(PY === undefined || !existsSync(PY ?? ""))("pypdf strict=True reads each file", () => {
    for (const [f, , pages] of files) {
      const out = execFileSync(PY!, ["-I", "-c", "import pypdf,sys; r=pypdf.PdfReader(sys.argv[1],strict=True); print(len(r.pages)); print(r.pages[-1].extract_text())", join(OUT!, f)], {
        encoding: "utf8",
        stdio: ["ignore", "pipe", "pipe"],
      });
      expect(out.split("\n")[0], f).toBe(String(pages));
      if (f === "incremental-on-compact.pdf") expect(out).toContain("Page two, revised");
      if (f.startsWith("incremental-2")) expect(out).toContain("Hello, Update");
    }
  });
});
