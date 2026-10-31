// Navigation: URI, page and named-destination links, an outline with a heading, colour and
// styles, page labels and the outline shown on open. The TS twin of
// crates/lamina/examples/links_and_outlines.rs.
import { Builder, Font, type Rect } from "@laminapdf/pdf";

const TITLES = ["Title page", "Contents", "Chapter 1", "Chapter 2"] as const;

/** The document's bytes. Needs `init({ text: true })`. */
export function linksAndOutlines(): Uint8Array {
  const row = (y: number): Rect => [36, y - 15, 220, 20];
  const b = new Builder({
    id: new TextEncoder().encode("lamina-ex-links!"),
    producer: "lamina examples",
    creationDate: "D:20261030120000Z",
  });
  const font = Font.standard("Helvetica");
  b.font(font);
  b.destination("chapter-2", 3);
  TITLES.forEach((title, index) => {
    const page = b.page("A5");
    const s = page.surface();
    s.text(title, { font, size: 18, at: [36, 60] });
    if (index === 0) {
      s.text("Visit example.com", { font, size: 12, at: [36, 100] });
      s.link(row(100), { type: "uri", uri: "https://example.com/" }, "Example");
      s.text("Go to chapter 1", { font, size: 12, at: [36, 125] });
      s.link(row(125), { type: "page", index: 2 });
      s.text("Go to chapter 2 (named)", { font, size: 12, at: [36, 150] });
      s.link(row(150), { type: "named", name: "chapter-2" });
    } else {
      s.text("Back to the title page", {
        font,
        size: 12,
        at: [36, 100],
        fill: [0, 0.2, 0.8],
        link: { target: { type: "page", index: 0 }, contents: "Title page" },
      });
    }
    page.finish();
  });
  b.outline([
    { title: "Title page", target: { type: "page", index: 0 } },
    { title: "Contents", target: { type: "page", index: 1 }, color: [0, 0, 0.6] },
    {
      title: "Chapters",
      open: true,
      bold: true,
      children: [
        { title: "Chapter 1", target: { type: "page", index: 2 } },
        { title: "Chapter 2", target: { type: "named", name: "chapter-2" }, italic: true },
      ],
    },
  ]);
  b.pageLabels([
    { start: 0, style: "lowerRoman" },
    { start: 2, style: "decimal" },
  ]).viewerPreferences({ pageMode: "outlines" });
  return b.finish().save();
}
