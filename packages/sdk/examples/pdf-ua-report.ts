// An accessible report that screen readers can follow, checked against PDF/UA-1 when it is built.
import { Builder, Font, type Surface, type Tag, type TagName } from "@laminapdf/sdk";

const MARGIN = 56;
const ROW = 18;
const COLUMN = 144;
const HEADER_BASELINE = 40;
const TITLE_BASELINE = 90;
const TABLE_TOP = 150;
const LINK_BASELINE = 400;
const CHART_LEFT = 320;
const CHART_BASE = 360;
const BAR_WIDTH = 36;
const BAR_PITCH = 50;
const BODY = 11;
/** The font sizes of the title and the headings; other text is `BODY`. */
const SIZES: Partial<Record<TagName, number>> = { H1: 22, H2: 15 };
const BLUE = [0.12, 0.47, 0.78] as const;
const QUARTERS = [
  ["Q1", 120],
  ["Q2", 135],
  ["Q3", 160],
  ["Q4", 150],
] as const;
const CHART_ALT = "Bar chart of sales by quarter: 120, 135, 160 and 150 thousand euros.";

/**
 * The report's bytes. Needs `init({ text: true })`.
 *
 * @param notoSans - the bytes of the Noto Sans font file.
 */
export function pdfUaReport(notoSans: Uint8Array): Uint8Array<ArrayBuffer> {
  using b = new Builder({ profiles: ["PDF/UA-1"] });
  b.metadata({ title: "Annual sales report 2026" });
  b.lang("en-GB");
  const font = Font.fromBytes(notoSans);
  b.font(font);
  const page = b.page("A4");
  draw(page.surface(), font);
  page.finish();
  using doc = b.finish();
  return doc.save();
}

/** One line of text with its baseline at `y`, as the structure element `tag`, sized by it. */
function line(s: Surface, font: Font, tag: Tag, text: string, x: number, y: number): void {
  const size = (typeof tag === "string" ? SIZES[tag] : undefined) ?? BODY;
  s.tag(tag, (s) => s.text(text, { font, size, at: [x, y] }));
}

function draw(s: Surface, font: Font): void {
  // The running header is decoration: an artifact, which screen readers skip.
  s.artifact("pagination", (s) => s.text("Acme Widgets", { font, size: BODY, at: [MARGIN, HEADER_BASELINE] }));
  line(s, font, "H1", "Annual sales report 2026", MARGIN, TITLE_BASELINE);
  line(s, font, "P", "Sales grew in three quarters of four.", MARGIN, TITLE_BASELINE + 2 * ROW);
  table(s, font);
  chart(s);
  const link = {
    target: { type: "uri", uri: "https://example.com/accounts" },
    contents: "The full 2026 accounts",
  } as const;
  s.tag("P", (s) =>
    s.text("The full accounts", { font, size: BODY, at: [MARGIN, LINK_BASELINE], fill: [...BLUE], link }),
  );
}

/** A heading and a table of the quarters, whose first row holds the columns' header cells. */
function table(s: Surface, font: Font): void {
  line(s, font, "H2", "Sales by quarter", MARGIN, TABLE_TOP);
  const header: Tag = { type: "TH", scope: "column" };
  s.tag("Table", (s) => {
    s.tag("TR", (s) => {
      line(s, font, header, "Quarter", MARGIN, TABLE_TOP + ROW);
      line(s, font, header, "Sales (k€)", MARGIN + COLUMN, TABLE_TOP + ROW);
    });
    QUARTERS.forEach(([quarter, sales], i) => {
      const y = TABLE_TOP + ROW * (i + 2);
      s.tag("TR", (s) => {
        line(s, font, "TD", quarter, MARGIN, y);
        line(s, font, "TD", sales.toFixed(0), MARGIN + COLUMN, y);
      });
    });
  });
}

/** The quarters as bars standing on the chart's base: one figure with alternate text. */
function chart(s: Surface): void {
  s.tag({ type: "Figure", alt: CHART_ALT }, (s) => {
    s.fillColor([...BLUE]);
    QUARTERS.forEach(([, sales], i) => {
      s.path((p) => p.rect([CHART_LEFT + BAR_PITCH * i, CHART_BASE - sales, BAR_WIDTH, sales])).fill();
    });
  });
}
