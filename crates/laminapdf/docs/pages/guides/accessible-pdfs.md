---
title: "Accessible PDFs"
description: "Build PDF/UA-1 documents that screen readers can follow, with every machine-checkable rule checked when the builder finishes."
version: "0.3.0"
url: "https://docs.laminapdf.com/guides/accessible-pdfs/"
---

# Accessible PDFs

Build PDF/UA-1 documents that screen readers can follow, with every machine-checkable rule checked when the builder finishes.

This guide builds a report that screen-reader users can read, which you need whenever a document has to meet accessibility law or a client's accessibility policy.

## What PDF/UA asks for

PDF/UA-1 ("Universal Accessibility") is the standard for accessible PDF. A PDF/UA file is tagged: its structure tree says which content is a heading, a paragraph, a list, a table or a figure, and in what order to read it. Everything else, such as running headers and decorative rules, is marked as an artifact, which assistive technology skips. The file also needs a title, a language, alternate text for every figure, a description for every link and embedded fonts that map each glyph to characters.

Turn the profile on with one option: `profiles: ["PDF/UA-1"]` in TypeScript, `BuildOptions::default().with_profile(Profile::PdfUa1)` in Rust. Then `finish()` checks the rules a program can check and fails with a coded error for the first one the document breaks:

| Rule | Error |
| --- | --- |
| The metadata has a title | [`missing-title`](../../errors/missing-title.md) |
| The document has a language (`lang`) | [`missing-lang`](../../errors/missing-lang.md) |
| Everything drawn is tagged or an artifact | [`untagged-content`](../../errors/untagged-content.md) |
| Every figure and formula has alternate text | [`missing-alt-text`](../../errors/missing-alt-text.md) |
| Headings start at `H1` and skip no level going down | [`heading-level-skipped`](../../errors/heading-level-skipped.md) |
| One unnumbered `H` per section, never beside `H1`–`H6` | [`unnumbered-heading-misused`](../../errors/unnumbered-heading-misused.md) |
| Table cells sit in rows, rows in tables, list items in lists | [`invalid-structure-nesting`](../../errors/invalid-structure-nesting.md) |
| Every link has a `Link` element and a description | [`link-without-structure`](../../errors/link-without-structure.md), [`link-without-contents`](../../errors/link-without-contents.md) |
| Every font is embedded (no standard 14 font) | [`font-not-embedded`](../../errors/font-not-embedded.md) |
| Every glyph maps to Unicode | [`glyph-without-unicode`](../../errors/glyph-without-unicode.md) |
| The PDF version is 1.7 or lower | [`profile-version-conflict`](../../errors/profile-version-conflict.md) |

The builder writes the rest for you. It puts the PDF/UA identifier in the document's XMP metadata, the XML packet in the file that describes it. It also asks viewers to show the title instead of the file name, and orders links by the structure.

## The example

The report below draws its running header as an artifact. Then come a heading, a paragraph, a table with header cells, a bar chart as a figure with alternate text, and a link with a description. A small `line` helper gives each line of text its structure element.

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

[See the output](../../pages/recipes/pdf-ua-report.md).

## What a program can't check

About half of PDF/UA's requirements need a person. Lamina PDF can check that a figure has alternate text, but not that the text describes the figure. It can check that headings go down one level at a time. It can't check that the right text is a heading, that the reading order makes sense or that colour isn't the only way the document conveys meaning. Read the document with a screen reader before you publish it.

## How Lamina PDF checks its own output

Every example on this site that claims PDF/UA-1 is validated each night with veraPDF, the open-source validator the PDF Association backs.

> **Details:** PDF/UA-1 is ISO 14289-1. veraPDF checks it against the Matterhorn Protocol, the PDF Association's list of PDF/UA failure conditions; most of the builder's checks are Matterhorn conditions a program can test; the version check comes from PDF/UA-1's base, PDF 1.7.

Next: to make a document both accessible and archival, add a PDF/A profile next to `PDF/UA-1`, as [Archival PDFs](../../pages/guides/archival-pdfs.md) shows.
