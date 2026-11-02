// Links and outlines: four A5 pages joined by links, bookmarks and page labels. The title page
// links to a website, to a page and through a named destination; every other page has a linked
// text run back to it. The outline is nested and shown when the file opens.
import { Builder, type BuilderOptions, Font, type LinkTarget, type Surface } from "@laminapdf/sdk";

const TITLES = ["Title page", "Contents", "Chapter 1", "Chapter 2"] as const;
const MARGIN = 36;
const TITLE_BASELINE = 60;
const TITLE_SIZE = 18;
const LINK_SIZE = 12;
/** The baselines of the title page's three links. */
const LINK_BASELINES = [100, 125, 150] as const;
/** A link's clickable box: this wide, this tall, starting this far above the baseline. */
const LINK_WIDTH = 220;
const LINK_HEIGHT = 20;
const LINK_RISE = 15;

/**
 * The document's bytes. Needs `init({ text: true })`.
 *
 * @param options - can fix the file ID, the producer and the creation date.
 */
export function linksAndOutlines(options: BuilderOptions = {}): Uint8Array<ArrayBuffer> {
  using b = new Builder(options);
  const font = Font.standard("Helvetica");
  b.font(font);
  b.destination("chapter-2", 3);
  TITLES.forEach((title, index) => {
    const page = b.page("A5");
    const s = page.surface();
    s.text(title, { font, size: TITLE_SIZE, at: [MARGIN, TITLE_BASELINE] });
    if (index === 0) titlePageLinks(s, font);
    else {
      const link = { target: { type: "page", index: 0 }, contents: "Title page" } as const;
      s.text("Back to the title page", { font, size: LINK_SIZE, at: [MARGIN, 100], fill: [0, 0.2, 0.8], link });
    }
    page.finish();
  });
  navigation(b);
  using doc = b.finish();
  return doc.save();
}

/** The title page's links: to a website, to a page, and to a named destination. */
function titlePageLinks(s: Surface, font: Font): void {
  const links: readonly (readonly [label: string, target: LinkTarget, contents?: string])[] = [
    ["Visit example.com", { type: "uri", uri: "https://example.com/" }, "Example"],
    ["Go to chapter 1", { type: "page", index: 2 }],
    ["Go to chapter 2 (named)", { type: "named", name: "chapter-2" }],
  ];
  links.forEach(([label, target, contents], i) => {
    const baseline = LINK_BASELINES[i] ?? 0;
    s.text(label, { font, size: LINK_SIZE, at: [MARGIN, baseline] });
    s.link([MARGIN, baseline - LINK_RISE, LINK_WIDTH, LINK_HEIGHT], target, contents);
  });
}

/** The outline (bookmarks), the page labels, and the viewer opening with the outline shown. */
function navigation(b: Builder): void {
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
}
