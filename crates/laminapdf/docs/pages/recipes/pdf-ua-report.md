---
title: "A PDF/UA report"
description: "A one-page sales report a screen reader can follow, with headings, a table, a described chart and a link, checked against PDF/UA-1 as it's built."
version: "0.2.0-alpha.4"
url: "https://docs.laminapdf.com/recipes/pdf-ua-report/"
---

# A PDF/UA report

A one-page sales report a screen reader can follow, with headings, a table, a described chart and a link, checked against PDF/UA-1 as it's built.

You'll build a one-page sales report that a screen reader can follow: headings, a table, a chart described in words and a link. You need this when a document has to meet accessibility law or a client's accessibility policy.

The function turns on the PDF/UA-1 profile, then draws each part of the report inside a tag that says what it is: a heading, a table cell, a figure. In TypeScript it needs the text module, `init({ text: true })`. Both languages write the same bytes:

**TypeScript**

`packages/sdk/examples/pdf-ua-report.ts`:

```ts
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
```

**Rust**

`crates/laminapdf/examples/pdf_ua_report.rs`:

```rust
//! An accessible report that screen readers can follow, checked against PDF/UA-1 when it is built.

use laminapdf::{
    ArtifactKind, BuildError, BuildOptions, Builder, Color, FillRule, FontHandle, LinkTarget, Metadata, PageSize,
    PointPt, Profile, Pt, RectPt, SaveOptions, Surface, TableScope, Tag, TagKind,
};

const MARGIN: f64 = 56.0;
const ROW: f64 = 18.0;
const COLUMN: f64 = 144.0;
const HEADER_AT: PointPt = PointPt::from_points(MARGIN, 40.0);
const TITLE_BASELINE: f64 = 90.0;
const TABLE_TOP: f64 = 150.0;
const LINK_AT: PointPt = PointPt::from_points(MARGIN, 400.0);
const CHART_LEFT: f64 = 320.0;
const CHART_BASE: f64 = 360.0;
const BAR_WIDTH: Pt = Pt::new(36.0);
const BAR_PITCH: f64 = 50.0;
const BODY: Pt = Pt::new(11.0);
/// The font sizes of the title and the headings; other text is `BODY`.
const SIZES: [(TagKind, Pt); 2] = [(TagKind::H1, Pt::new(22.0)), (TagKind::H2, Pt::new(15.0))];
const BLUE: Color = Color::Rgb(0.12, 0.47, 0.78);
const QUARTERS: [(&str, f64); 4] = [("Q1", 120.0), ("Q2", 135.0), ("Q3", 160.0), ("Q4", 150.0)];
const CHART_ALT: &str = "Bar chart of sales by quarter: 120, 135, 160 and 150 thousand euros.";

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = std::env::args().skip(1);
    let font = std::fs::read(args.next().ok_or("arguments: <font.ttf> [out-dir]")?)?;
    let out = std::path::PathBuf::from(args.next().unwrap_or_else(|| "target/examples-out".into()));
    std::fs::create_dir_all(&out)?;
    Ok(std::fs::write(out.join("pdf_ua_report.pdf"), build(font)?)?)
}

pub(crate) fn build(font: Vec<u8>) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let mut b = Builder::new(BuildOptions::new().with_profile(Profile::PdfUa1));
    b.metadata(Metadata::default().with_title("Annual sales report 2026"));
    b.lang("en-GB");
    let font = b.font_bytes(font)?;
    let mut page = b.page(PageSize::A4);
    draw(page.surface(), &font)?;
    page.finish()?;
    Ok(b.finish()?.save(&SaveOptions::new())?.bytes)
}

/// One line of text with its baseline at `y`, as the structure element `tag`, sized by it.
fn line(s: &mut Surface, font: &FontHandle, tag: impl Into<Tag>, text: &str, x: f64, y: f64) -> Result<(), BuildError> {
    let tag = tag.into();
    let size = SIZES.iter().find(|e| e.0 == tag.kind()).map_or(BODY, |e| e.1);
    s.tag(tag, |s| s.text(text, font, size, PointPt::from_points(x, y)).map(drop))
}

fn draw(s: &mut Surface, font: &FontHandle) -> Result<(), BuildError> {
    // The running header is decoration: an artifact, which screen readers skip.
    let header = |s: &mut Surface| s.text("Acme Widgets", font, BODY, HEADER_AT).map(drop);
    s.artifact(ArtifactKind::Pagination, header)?;
    line(s, font, TagKind::H1, "Annual sales report 2026", MARGIN, TITLE_BASELINE)?;
    let summary = "Sales grew in three quarters of four.";
    line(s, font, TagKind::P, summary, MARGIN, TITLE_BASELINE + 2.0 * ROW)?;
    table(s, font)?;
    chart(s)?;
    let link = LinkTarget::Uri("https://example.com/accounts".into());
    s.tag(TagKind::P, |s| {
        let text = s.text("The full accounts", font, BODY, LINK_AT)?;
        text.fill(BLUE).link(link, Some("The full 2026 accounts".into()));
        Ok(())
    })
}

/// A heading and a table of the quarters, whose first row holds the columns' header cells.
fn table(s: &mut Surface, font: &FontHandle) -> Result<(), BuildError> {
    line(s, font, TagKind::H2, "Sales by quarter", MARGIN, TABLE_TOP)?;
    let header = Tag::new(TagKind::TH).with_scope(TableScope::Column);
    s.tag(TagKind::Table, |s| {
        s.tag(TagKind::TR, |s| {
            line(s, font, header.clone(), "Quarter", MARGIN, TABLE_TOP + ROW)?;
            line(s, font, header.clone(), "Sales (k€)", MARGIN + COLUMN, TABLE_TOP + ROW)
        })?;
        for (row, (quarter, sales)) in (2_u8..).zip(QUARTERS) {
            let y = TABLE_TOP + ROW * f64::from(row);
            s.tag(TagKind::TR, |s| {
                line(s, font, TagKind::TD, quarter, MARGIN, y)?;
                line(s, font, TagKind::TD, &format!("{sales:.0}"), MARGIN + COLUMN, y)
            })?;
        }
        Ok(())
    })
}

/// The quarters as bars standing on the chart's base: one figure with alternate text.
fn chart(s: &mut Surface) -> Result<(), BuildError> {
    s.tag(Tag::new(TagKind::Figure).with_alt(CHART_ALT), |s| {
        s.fill_color(BLUE);
        for (i, (_, sales)) in (0_u8..).zip(QUARTERS) {
            let x = CHART_LEFT + BAR_PITCH * f64::from(i);
            let bar = RectPt::from_xywh(Pt::new(x), Pt::new(CHART_BASE - sales), BAR_WIDTH, Pt::new(sales));
            s.path(|p| _ = p.rect(bar)).fill(FillRule::NonZero);
        }
        Ok(())
    })
}
```

**Result:** [pdf_ua_report.pdf](https://docs.laminapdf.com/examples/pdf/pdf_ua_report.pdf)

The result is one A4 page that reads, in order, as a title, a summary, a table of four quarters, a bar chart and a link. The Result tab shows it, with a link to the PDF.

## How it works

- `profiles: ["PDF/UA-1"]` (`Profile::PdfUa1` in Rust) tags the document and makes `finish()` check the rules a program can check. It fails with a coded error for the first rule the report breaks; [Accessible PDFs](../../pages/guides/accessible-pdfs.md#what-pdfua-asks-for) lists them.
- `metadata` gives the report its title and `lang` its language, which PDF/UA-1 asks for. The font is an embedded Noto Sans, because PDF/UA-1 refuses the [standard 14 fonts](../../pages/reference/glossary.md#standard-14-fonts).
- `tag` puts what its callback draws inside a structure element, such as `H1`, `P` or `TD`, in reading order. The `line` helper tags one line of text and sizes it by its tag: 22 points for `H1`, 15 for `H2`, 11 for the rest.
- The table's first row holds `TH` header cells with a column scope, so a screen reader can name each cell's column.
- The four bars are one `Figure` with alternate text, the sentence a screen reader reads in place of the drawing. The running header is an artifact, which screen readers skip.
- The link's `contents`, "The full 2026 accounts", is its description. The builder makes the link a `Link` element inside its paragraph.

## What to change

- **The numbers:** change `QUARTERS`, then change `CHART_ALT` to match. The builder checks that the chart has alternate text, not that the text is right; see [What a program can't check](../../pages/guides/accessible-pdfs.md#what-a-program-cant-check).
- **A list:** nest an `"L"`, an `"LI"` for each item, and inside each a `"Lbl"` for the bullet and an `"LBody"` for the text. An `L` holds `LI` items or a nested `L`, and an `LI` holds a `Lbl` and an `LBody`: anything else inside them fails `finish()` with `invalid-structure-nesting`.
- **A passage in another language:** tag it with its own, as in `{ type: "P", lang: "fr-FR" }` (`Tag::new(TagKind::P).with_lang("fr-FR")` in Rust).
- **Accessible and archival at once:** add a PDF/A profile beside it, as in `profiles: ["PDF/A-2u", "PDF/UA-1"]`. The [PDF/A archive](../../pages/recipes/pdf-a-archive.md) does this.

Next: [a PDF/A archive](../../pages/recipes/pdf-a-archive.md), a letter written for long-term keeping.
