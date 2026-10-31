// A 100-page report with two embedded fonts: a heading, a paragraph broken into a box, a table
// with hairlines, a line of Chinese (glyphs the fixture subset has) and a footer on every page.
// It has no Rust golden: it is the §4 performance case (pdf-library plan: a 100-page report in
// ≤ 500 ms, < 64 MB in a Worker) that the runtime matrix (B6c) times in every runtime, checking
// that each produces the same bytes (test/suite/examples.ts). Its Rust twin is the
// `report_100_pages` bench in crates/lamina/benches/report.rs, which checks it builds the same
// bytes (REPORT_SHA256): change both together.
import { Builder, Font } from "@laminapdf/pdf";

/** The number of pages. */
export const REPORT_PAGES = 100;

const PARAGRAPH =
  "Quarterly figures for every region follow. Each table lists the units shipped, the revenue " +
  "and the change against the previous quarter; totals are at the end of the report.";

/**
 * The document's bytes. Needs `init({ text: true })`.
 *
 * @param notoSans - `fixtures/fonts/NotoSans-Regular.ttf`.
 * @param notoSansSC - `fixtures/fonts/NotoSansSC-Regular.otf`.
 */
export function report(notoSans: Uint8Array, notoSansSC: Uint8Array): Uint8Array {
  const b = new Builder({
    id: new TextEncoder().encode("lamina-ex-report"),
    producer: "lamina examples",
    creationDate: "D:20261030120000Z",
  });
  try {
    const font = Font.fromBytes(notoSans);
    const chinese = Font.fromBytes(notoSansSC);
    b.font(font);
    b.font(chinese);
    for (let n = 1; n <= REPORT_PAGES; n++) {
      const page = b.page("A4");
      const s = page.surface();
      s.text(`Region ${n}: quarterly report`, { font, size: 20, at: [56, 72] });
      s.textBox(PARAGRAPH, { font, size: 11, box: [56, 88, 483, 60] });
      s.text("你好世界，这是中文字体子集测试。", { font: chinese, size: 11, at: [56, 166] });
      for (let row = 0; row < 20; row++) {
        const y = 200 + 24 * row;
        const units = (n * 37 + row * 11) % 500;
        s.text(`Product ${row + 1}`, { font, size: 10, at: [56, y] });
        s.textBox(String(units), { font, size: 10, box: [300, y - 11, 60, 14], align: "end" });
        s.textBox(`€ ${(units * 12.5).toFixed(2)}`, { font, size: 10, box: [400, y - 11, 139, 14], align: "end" });
        s.strokeColor([0.8])
          .path((p) => p.moveTo(56, y + 7).lineTo(539, y + 7))
          .stroke({ width: 0.5 });
      }
      s.text(`Page ${n} of ${REPORT_PAGES}`, { font, size: 9, at: [56, 800] });
      page.finish();
    }
    b.metadata({ title: "Quarterly report", author: "lamina examples" }).lang("en-GB");
    // Free the document at once (the saved bytes are a copy): a Worker or a timing loop calling
    // this many times shouldn't wait for the garbage collector to release WebAssembly memory.
    const doc = b.finish();
    try {
      return doc.save();
    } finally {
      doc.free();
    }
  } finally {
    b.free(); // a no-op after finish(); frees the builder now when drawing threw
  }
}
