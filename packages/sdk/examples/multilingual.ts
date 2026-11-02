// Five scripts on one page, each as a single line and as a wrapped paragraph. The scripts are
// Latin, Arabic and Hebrew (right to left), Devanagari and simplified Chinese; each is shaped and
// bidi-resolved, and each paragraph wraps in a text box. Each script needs a font that covers it,
// such as the Noto fonts (OFL).
import { Builder, type BuilderOptions, type Direction, Font } from "@laminapdf/sdk";

/** Each script's line, its paragraph and the paragraph's direction, in the order of the fonts. */
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
const MARGIN = 56;
const FIRST_BASELINE = 80;
/** The distance from one script's line to the next one's. */
const SCRIPT_HEIGHT = 140;
/** The paragraph's box: this far below the line, this wide and this tall. */
const PARAGRAPH_DROP = 14;
const PARAGRAPH_WIDTH = 220;
const PARAGRAPH_HEIGHT = 100;

/**
 * The document's bytes. Needs `init({ text: true })`.
 *
 * @param fonts - one font file per script, in the order of {@link SCRIPTS}.
 * @param options - can fix the file ID, the producer and the creation date.
 */
export function multilingual(fonts: readonly Uint8Array[], options: BuilderOptions = {}): Uint8Array<ArrayBuffer> {
  if (fonts.length !== SCRIPTS.length) throw new Error(`${SCRIPTS.length} fonts, one per script, are needed`);
  using b = new Builder(options);
  const loaded = fonts.map((bytes) => Font.fromBytes(bytes));
  for (const font of loaded) b.font(font);
  const page = b.page("A4");
  const s = page.surface();
  SCRIPTS.forEach(([line, paragraph, direction], row) => {
    const font = loaded[row];
    if (font === undefined) return;
    const y = FIRST_BASELINE + SCRIPT_HEIGHT * row;
    s.text(line, { font, size: 16, at: [MARGIN, y] });
    // A narrow box breaks the paragraph into lines; right-to-left paragraphs start at the right.
    const box = [MARGIN, y + PARAGRAPH_DROP, PARAGRAPH_WIDTH, PARAGRAPH_HEIGHT] as const;
    s.textBox(paragraph, { font, size: 12, box, direction, align: "start" });
  });
  page.finish();
  using doc = b.finish();
  return doc.save();
}
