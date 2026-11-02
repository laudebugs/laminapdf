// Five scripts, each with its own embedded font: a shaped line and a paragraph broken into a
// narrow box (right-to-left for Arabic and Hebrew). The TS twin of
// crates/laminapdf/examples/multilingual.rs.
import { Builder, type Direction, Font } from "@laminapdf/sdk";

/** Each script's line, its paragraph and the paragraph's direction (fonts in the same order). */
export const SCRIPTS: readonly (readonly [line: string, paragraph: string, direction: Direction])[] = [
  [
    "Latin: the office staff fill the affine waffle flask",
    "A paragraph breaks at spaces and fits the width of its box, line by line.",
    "ltr",
  ],
  ["مرحبا بالعالم السلام عليكم", "مرحبا بالعالم السلام عليكم مرحبا بالعالم السلام عليكم", "rtl"],
  ["नमस्ते दुनिया। हिन्दी पाठ का परीक्षण", "नमस्ते दुनिया। हिन्दी पाठ का परीक्षण नमस्ते दुनिया।", "ltr"],
  ["שלום עולם וברוכים הבאים", "שלום עולם וברוכים הבאים שלום עולם וברוכים הבאים", "rtl"],
  ["你好世界，这是中文字体子集测试。", "你好世界，这是中文字体子集测试。你好世界，这是中文。", "ltr"],
];

/**
 * The document's bytes. Needs `init({ text: true })`.
 *
 * @param fonts - Noto Sans, Noto Sans Arabic, Devanagari, Hebrew and SC (`fixtures/fonts/`), in
 *   the order of {@link SCRIPTS}.
 */
export function multilingual(fonts: readonly Uint8Array[]): Uint8Array<ArrayBuffer> {
  const b = new Builder({
    id: new TextEncoder().encode("lamina-ex-multil"),
    producer: "lamina examples",
    creationDate: "D:20261030120000Z",
  });
  const loaded = fonts.map((bytes) => Font.fromBytes(bytes));
  for (const font of loaded) b.font(font);
  const page = b.page("A4");
  const s = page.surface();
  SCRIPTS.forEach(([line, paragraph, direction], row) => {
    const font = loaded[row];
    if (font === undefined) throw new Error(`no font for script ${row}`);
    const y = 80 + 140 * row;
    s.text(line, { font, size: 16, at: [56, y] });
    // A narrow box breaks the paragraph into lines; right-to-left paragraphs start at the right.
    s.textBox(paragraph, { font, size: 12, box: [56, y + 14, 220, 100], direction, align: "start" });
  });
  page.finish();
  return b.finish().save();
}
