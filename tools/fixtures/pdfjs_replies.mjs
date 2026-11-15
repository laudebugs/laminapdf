// pdf.js's view of replies and review states (todo E5b): per file and page, each annotation that
// replies to another, with what it replies to and its state as `page.getAnnotations()` reports
// them, and how many drawing operators pdf.js runs for the page with its annotations shown. Writes the
// facts that crates/laminapdf/tests/comments.rs compares: Lamina PDF's replies and states read as
// such, and draw nothing (the answered report draws as many operators as the marked-up one).
// Tools only, never run by tests.
//
//     cd "$(mktemp -d)" && npm init -y && npm i pdfjs-dist@6.4.299   # Apache-2.0; tools only
//     node tools/fixtures/pdfjs_replies.mjs "$PWD/node_modules/pdfjs-dist" \
//       fixtures/golden/examples/mark_up.pdf fixtures/golden/examples/review_comments.pdf \
//       fixtures/annotations/review.pdf > fixtures/annotations/pdfjs-replies.json
import { readFileSync } from "node:fs";
import { basename } from "node:path";
import { argv, stdout } from "node:process";
import { loadPdfjs } from "./pdfjs_common.mjs";

const PINNED = "6.4.299";
const [pdfjsDir, ...files] = argv.slice(2);
const { pdfjs, version } = await loadPdfjs(
  pdfjsDir,
  PINNED,
  "usage: pdfjs_replies.mjs <path to node_modules/pdfjs-dist> <file.pdf>…",
);

const out = { pdfjs_version: version, files: {} };
for (const file of files.sort()) {
  const doc = await pdfjs.getDocument({ data: new Uint8Array(readFileSync(file)), verbosity: 0 }).promise;
  const pages = [];
  for (let i = 1; i <= doc.numPages; i++) {
    const page = await doc.getPage(i);
    const replies = (await page.getAnnotations())
      .filter((a) => a.inReplyTo)
      .map((a) => ({
        id: a.id,
        inReplyTo: a.inReplyTo,
        replyType: a.replyType ?? null,
        stateModel: a.stateModel ?? null,
        state: a.state ?? null,
      }));
    // pdf.js wraps each annotation's appearance in beginAnnotation and endAnnotation, which draw
    // nothing themselves: count the operators between them.
    const list = await page.getOperatorList({ annotationMode: pdfjs.AnnotationMode.ENABLE });
    const wrappers = [pdfjs.OPS.beginAnnotation, pdfjs.OPS.endAnnotation];
    pages.push({ replies, operators: list.fnArray.filter((op) => !wrappers.includes(op)).length });
  }
  out.files[basename(file)] = pages;
}
stdout.write(`${JSON.stringify(out, null, 2)}\n`);
