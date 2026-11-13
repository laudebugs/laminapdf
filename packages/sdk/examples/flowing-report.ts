// A report that flows across pages: a cover with a contents page, then sections whose
// paragraphs break between pages, footnotes at the foot of their page, the section's title in
// a running header and "Page X of Y" in the footer.
//
// Lines split with at least two at the foot and the head of a page, headings stay with what
// follows them, and the photo moves whole. The document is plain objects naming its fonts and photo,
// whose bytes come beside it; the output is tagged, has bookmarks, and is checked against PDF/UA-1.
import { type BlockSpec, Builder, type BuilderOptions, type FlowDocument, Font, Image } from "@laminapdf/sdk";

const TITLE = "Reading files offline: a year of the reader";
const ARABIC = "يعمل القارئ دون اتصال بالشبكة، فتُفتح الملفات وتُقرأ وتُبحث على الجهاز نفسه، ولا يغادر أي منها الحاسوب.";
const HINDI = "नमस्ते दुनिया। हिन्दी पाठ का परीक्षण।";
const TOPICS = ["Opening files", "Search", "Printing", "Signing", "What comes next"];
const PHOTO_WIDTH = 200;
const FOOTER = { center: "Page {page} of {pages}" };

/** The fonts the report is set in: Latin, a bold face, Arabic and Devanagari. */
export interface ReportFonts {
  readonly regular: Uint8Array;
  /** A collection whose second face (index 1) is the bold one. */
  readonly bold: Uint8Array;
  readonly arabic: Uint8Array;
  readonly devanagari: Uint8Array;
}

/**
 * The report's bytes. Needs `init({ text: true })`.
 *
 * @param fonts - the fonts to set it in.
 * @param photo - the JPEG each section shows.
 * @param options - can fix the file ID, the producer and the creation date.
 */
export function flowingReport(fonts: ReportFonts, photo: Uint8Array, options: BuilderOptions = {}) {
  using b = new Builder({ ...options, profiles: ["PDF/UA-1"] });
  b.flow(report(), {
    fonts: {
      regular: Font.fromBytes(fonts.regular),
      arabic: Font.fromBytes(fonts.arabic),
      devanagari: Font.fromBytes(fonts.devanagari),
      bold: Font.fromCollection(fonts.bold, 1),
    },
    images: { photo: Image.jpeg(photo) },
  });
  using doc = b.finish();
  return doc.save();
}

/** The whole report: a cover with the contents, then a section per topic. */
function report(): FlowDocument {
  const body = { font: "regular", size: 11, fallbacks: ["arabic", "devanagari"] };
  return {
    schema: "laminapdf.document/1",
    title: TITLE,
    lang: "en",
    // The cover is a plain A5 page; the others name their section and their number.
    page: { size: "A5", first: { size: "A5" }, style: "small", header: { end: "{section}" }, footer: FOOTER },
    styles: {
      body,
      note: { ...body, size: 9 },
      arabic: { ...body, language: "ar" },
      hindi: { ...body, language: "hi" },
      heading: { font: "bold", size: 16 },
      cover: { font: "bold", size: 24 },
      small: { font: "regular", size: 9 },
    },
    blocks: [
      { type: "paragraph", text: TITLE, style: "cover" },
      { type: "paragraph", text: "A report from the reader's team." },
      { type: "contents" },
      { type: "pageBreak" },
      ...TOPICS.flatMap(section),
    ],
  };
}

/** A section: its heading, paragraphs (one in Arabic, one in Hindi), a list and a photo. */
function section(topic: string, i: number): BlockSpec[] {
  const sentence = `The ${topic.toLowerCase()} work of the year changed how readers use the app every day. `;
  const note = { text: `Measured on our test files for ${topic}.`, style: "note" };
  const paragraphs = [0, 1, 2, 3].map(
    (n): BlockSpec => ({
      type: "paragraph",
      text: sentence.repeat(n + 4),
      spaceAfter: 8,
      spans: n === 3 ? [{ note }] : [],
    }),
  );
  const foreign: BlockSpec[] = []; // Section 2 has a paragraph in Arabic, section 4 one in Hindi.
  if (i === 1) foreign.push({ type: "paragraph", text: `${ARABIC} `.repeat(8).trimEnd(), style: "arabic" });
  if (i === 3) foreign.push({ type: "paragraph", text: `${HINDI} `.repeat(12).trimEnd(), style: "hindi" });
  const items = ["faster", "smaller", "easier to share"].map((word) => ({ text: `Files are ${word}.` }));
  const alt = `A photo of the team working on ${topic.toLowerCase()}`;
  return [
    { type: "heading", level: 1, text: topic, style: "heading" },
    ...paragraphs,
    ...foreign,
    { type: "list", kind: "bullet", items },
    { type: "figure", image: "photo", alt, width: PHOTO_WIDTH },
  ];
}
