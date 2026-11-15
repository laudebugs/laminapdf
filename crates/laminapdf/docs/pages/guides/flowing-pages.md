---
title: "Pages that flow"
description: "Pour headings, paragraphs, tables and images, or a Markdown file, into as many pages as they need, with footnotes, a contents page and \"Page X of Y\" footers."
version: "0.4.0"
url: "https://docs.laminapdf.com/guides/flowing-pages/"
---

# Pages that flow

Pour headings, paragraphs, tables and images, or a Markdown file, into as many pages as they need, with footnotes, a contents page and "Page X of Y" footers.

This guide lays out a document whose length you don't know in advance, such as a report or a contract. You give Lamina PDF blocks and a page template, and it makes as many pages as they need, with a header and a footer on each.

It keeps headings with their text and never leaves a single line of a paragraph alone at a page's foot or head, and it tags the result so screen readers can follow it. In Rust you build the blocks with types such as `Flow`, `Paragraph` and `Table`. In TypeScript you give it the same document as plain objects, or as Markdown; the [JSON section](#write-it-as-json) explains the format.

## The example

The example builds a 13-page report: a cover with a contents page, then five sections of headings, paragraphs, a footnote, a bulleted list and a photo. One paragraph is in Arabic and one in Hindi, and each is split by a page break. Every page after the cover names its section in the header and says "Page 3 of 13" in the footer, and every section has a bookmark. The report is built under PDF/UA-1, the accessibility standard, so the build fails if the output breaks one of its rules.

The TypeScript version describes the same report in plain objects, in the `laminapdf.document/1` format, with a named style for each kind of text. It passes the fonts and the photo beside the document, by the names the document uses. Both languages write the same bytes:

**TypeScript**

`packages/sdk/examples/flowing-report.ts`:

```ts
// A report that flows across pages: a cover with a contents page, then sections whose
// paragraphs break between pages, footnotes at the foot of their page, the section's title in
// a running header and "Page X of Y" in the footer.
//
// Lines split with at least two at a page's foot and head, headings stay with what follows, and the
// photo moves whole. The document is plain objects; its fonts and photo come beside it by name.
import { type BlockSpec, Builder, type BuilderOptions, type FlowDocument, Font, Image } from "@laminapdf/sdk";

const TITLE = "Reading files offline: a year of the reader";
const ARABIC = "يعمل القارئ دون اتصال بالشبكة، فتُفتح الملفات وتُقرأ وتُبحث على الجهاز نفسه، ولا يغادر أي منها الحاسوب.";
const HINDI = "नमस्ते दुनिया। हिन्दी पाठ का परीक्षण।";
const TOPICS = ["Opening files", "Search", "Printing", "Signing", "What comes next"];
const PHOTO_WIDTH = 200;
const PARAGRAPH_SPACE = 8;
const FOOTER = { center: "Page {page} of {pages}" };

/** The report's fonts: Latin, a collection whose second face is the bold one, Arabic and Devanagari. */
export type FlowingReportFonts = Readonly<Record<"regular" | "bold" | "arabic" | "devanagari", Uint8Array>>;

/**
 * The report's bytes. Needs `init({ text: true })`.
 *
 * @param fonts - the fonts to set it in.
 * @param photo - the JPEG each section shows.
 * @param options - can fix the file ID, the producer and the creation date.
 */
export function flowingReport(
  fonts: FlowingReportFonts,
  photo: Uint8Array,
  options: BuilderOptions = {},
): Uint8Array<ArrayBuffer> {
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
  const paragraph = (repeats: number, withNote = false): BlockSpec => ({
    type: "paragraph",
    text: sentence.repeat(repeats),
    spaceAfter: PARAGRAPH_SPACE,
    spans: withNote ? [{ note }] : null,
  });
  const paragraphs = [paragraph(4), paragraph(5), paragraph(6), paragraph(7, true)];
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
```

**Rust**

`crates/laminapdf/examples/flowing_report.rs`:

```rust
//! A report that flows across pages: a cover with a contents page, then sections whose
//! paragraphs break between pages, footnotes at the foot of their page, the section's title in
//! a running header and "Page X of Y" in the footer.
//!
//! Lines split with at least two at the foot and the head of a page, headings stay with what
//! follows them, and the photo moves whole. The output is tagged, has bookmarks, and is checked
//! against PDF/UA-1. Run it with `cargo run -p laminapdf --example flowing_report -- regular.ttf
//! bold.ttc arabic.ttf devanagari.ttf photo.jpg`.

use std::error::Error;

use laminapdf::flow::{
    Contents, Figure, Flow, FlowBlock, List, ListItem, ListKind, PageRegion, PageTemplate, Paragraph, ParagraphStyle,
    TextStyle,
};
use laminapdf::{BuildOptions, Builder, Font, Image, Metadata, PageSize, Profile, SaveOptions};

const TITLE: &str = "Reading files offline: a year of the reader";
pub(crate) const ARABIC: &str =
    "يعمل القارئ دون اتصال بالشبكة، فتُفتح الملفات وتُقرأ وتُبحث على الجهاز نفسه، ولا يغادر أي منها الحاسوب.";
pub(crate) const HINDI: &str = "नमस्ते दुनिया। हिन्दी पाठ का परीक्षण।";
const TOPICS: [&str; 5] = ["Opening files", "Search", "Printing", "Signing", "What comes next"];

fn main() -> Result<(), Box<dyn Error>> {
    let paths: Vec<String> = std::env::args().skip(1).collect();
    let [regular, bold, arabic, hindi, photo] = paths.as_slice() else {
        return Err("arguments: <regular.ttf> <bold.ttc> <arabic.ttf> <devanagari.ttf> <photo.jpg>".into());
    };
    let read = |path: &String| std::fs::read(path);
    let fonts = [read(regular)?, read(bold)?, read(arabic)?, read(hindi)?];
    let pdf = build(BuildOptions::new(), fonts, std::fs::read(photo)?)?;
    std::fs::create_dir_all("target/examples-out")?;
    Ok(std::fs::write("target/examples-out/flowing_report.pdf", pdf)?)
}

/// The report's bytes. `options` can fix the file ID, the producer and the creation date.
pub(crate) fn build(options: BuildOptions, fonts: [Vec<u8>; 4], photo: Vec<u8>) -> Result<Vec<u8>, Box<dyn Error>> {
    let [regular, bold, arabic, hindi] = fonts;
    let mut b = Builder::new(options.with_profile(Profile::PdfUa1));
    b.metadata(Metadata::default().with_title(TITLE));
    b.lang("en");
    let regular = b.font(Font::from_bytes(regular)?)?;
    let body = TextStyle::new(&regular, 11.0)
        .with_fallback(&b.font(Font::from_bytes(arabic)?)?)
        .with_fallback(&b.font(Font::from_bytes(hindi)?)?);
    let heading = TextStyle::new(&b.font(Font::from_collection(bold, 1)?)?, 16.0);
    let small = TextStyle::new(&regular, 9.0);
    let header = PageRegion::new().with_end("{section}");
    let footer = PageRegion::new().with_center("Page {page} of {pages}");
    let pages = PageTemplate::new(PageSize::A5).with_furniture(Some(header), Some(footer), &small);
    let (cover, team) = (heading.clone().with_size(24.0), "A report from the reader's team.");
    let mut flow = Flow::new(pages.with_first_page(PageTemplate::new(PageSize::A5)))
        .with_title(TITLE)
        .block(FlowBlock::Paragraph(Paragraph::plain(TITLE, &cover)))
        .block(FlowBlock::Paragraph(Paragraph::plain(team, &body)))
        .block(FlowBlock::Contents(Contents::new(&body)))
        .block(FlowBlock::PageBreak);
    let photo = Image::jpeg(photo)?;
    for (i, topic) in TOPICS.iter().enumerate() {
        flow = section(flow, i, topic, &body, &heading, &photo);
    }
    b.flow(&flow)?;
    Ok(b.finish()?.save(&SaveOptions::new())?.bytes)
}

/// A section: its heading, paragraphs (one in Arabic, one in Hindi), a list and a photo.
fn section(flow: Flow, i: usize, topic: &str, body: &TextStyle, heading: &TextStyle, photo: &Image) -> Flow {
    let sentence = format!(
        "The {} work of the year changed how readers use the app every day. ",
        topic.to_lowercase()
    );
    let mut flow = flow.block(FlowBlock::Heading(1, Paragraph::plain(topic, heading)));
    let note = Paragraph::plain(
        format!("Measured on our test files for {topic}."),
        &body.clone().with_size(9.0),
    );
    for n in 0..4 {
        let p = Paragraph::new(ParagraphStyle::new().with_space(0.0, 8.0)).text(sentence.repeat(n + 4), body);
        let p = if n == 3 { p.note(note.clone()) } else { p };
        flow = flow.block(FlowBlock::Paragraph(p));
    }
    let foreign = match i {
        1 => Some((format!("{ARABIC} ").repeat(8).trim_end().to_owned(), "ar")),
        3 => Some((format!("{HINDI} ").repeat(12).trim_end().to_owned(), "hi")),
        _ => None,
    };
    if let Some((text, lang)) = foreign {
        let style = body.clone().with_language(lang);
        flow = flow.block(FlowBlock::Paragraph(Paragraph::plain(text, &style)));
    }
    let list = ["faster", "smaller", "easier to share"]
        .iter()
        .fold(List::new(ListKind::Bullet, body), |list, word| {
            list.item(ListItem::new(Paragraph::plain(format!("Files are {word}."), body)))
        });
    let alt = format!("A photo of the team working on {}", topic.to_lowercase());
    flow.block(FlowBlock::List(list))
        .block(FlowBlock::Figure(Figure::new(photo.clone(), alt).with_width(200.0)))
}
```

**Result:** [flowing_report.pdf](https://docs.laminapdf.com/examples/pdf/flowing_report.pdf)

The result is thirteen A5 pages: [open the reference PDF](https://docs.laminapdf.com/examples/pdf/flowing_report.pdf).

## Blocks, styles and the template

A `Flow` is a list of blocks and the `PageTemplate` they flow through.

- **Paragraphs** are made of pieces of text, each in its own `TextStyle`: its fonts, size, colour, language and a baseline rise for superscripts. A bold word is a piece in the bold font. A link is a piece with a target, and a line break or a footnote is a piece of its own.
- **Tables** have header rows, body rows and footer rows; a later section covers them.
- A **contents** block lists the headings with their pages, and footnotes go in paragraphs; both are covered below.
- **Headings** run from level 1 to 6. **Lists** are bulleted or numbered, and an item can hold a nested list. A **figure** is an image with its alternate text, as wide as the page's text column unless you give it a width.
- **Spacers, page breaks and keep groups** shape the layout: a spacer adds space, a page break starts a new page, and a `Keep` group keeps its blocks on one page.

The template gives the page size, the margins, a header and a footer, and optionally a different first page, such as a cover without a header. A header or footer has three slots, at the start, centre and end of the line.

## Where pages break

The flow lays out every page before it draws any, and it follows these rules:

- **Paragraphs split only between lines,** leaving at least two lines at the foot of a page and two at the head of the next. When a page has room for only one line of a paragraph, the paragraph moves to the next page. Change the counts with `with_widows_and_orphans`, or a JSON paragraph's `widows` and `orphans`.
- **A heading stays with the block after it,** so a section never starts on the last line of a page. Any paragraph can do the same with `with_keep_with_next`, or `keepWithNext` in JSON.
- **Figures and keep groups move whole** to the next page when they don't fit.
- **A paragraph continued on a page of another width** is broken again at that width.

When blocks you asked to keep together are taller than a page, the flow splits them anyway and adds a `flow-keep-broken` warning to the report `Builder::flow` returns. A figure or a single line taller than a page fails with `flow-content-too-large`, since no page could hold it.

## Tables across pages

A table can run over as many pages as its rows need, with its header on each page and every row whole. The second example builds a three-page invoice of 60 rows with a total at the end. Again, both languages write the same bytes:

**TypeScript**

`packages/sdk/examples/flowing-invoice.ts`:

```ts
// An invoice whose table runs over three pages: its header row repeats at the top of each
// page, no row is split between pages, and the total stays on the page of the last row.
//
// The columns fit their widest cell, and numbers line up on the right. The document is plain
// objects in the `laminapdf.document/1` format, which names its two fonts; their bytes come beside
// it. The output is tagged (one `Table` element across the pages) and checked against PDF/UA-1.
import { type BlockSpec, Builder, type BuilderOptions, type CellSpec, type FlowDocument, Font } from "@laminapdf/sdk";

const TITLE = "Invoice 2026-114";
const PRODUCTS: readonly (readonly [string, number])[] = [
  ["Paper, A4, 80 g, box of 2,500 sheets", 2_450],
  ["Toner cartridge, black", 6_900],
  ["Desk lamp with a weighted base and an adjustable arm", 4_275],
  ["Stapler", 1_290],
  ["Archive boxes, set of ten, with lids", 3_150],
  ["Ballpoint pens, box of fifty", 1_875],
];
const LINES = 60;
const MARGIN = 56;
const PADDING = 6;
const RULE_WIDTH = 0.75;
const ROW_RULE_WIDTH = 0.25;
/** The frame and the rule under the header, the rules between rows, and the header row's shade. */
const RULE = "#666666";
const ROW_RULE = "#b3b3b3";
const HEADER_SHADE = "#ebebeb";

/**
 * The invoice's bytes. Needs `init({ text: true })`.
 *
 * @param regular - the body's font.
 * @param bold - a collection whose second face (index 1) is the bold one.
 * @param options - can fix the file ID, the producer and the creation date.
 */
export function flowingInvoice(
  regular: Uint8Array,
  bold: Uint8Array,
  options: BuilderOptions = {},
): Uint8Array<ArrayBuffer> {
  using b = new Builder({ ...options, profiles: ["PDF/UA-1"] });
  const document: FlowDocument = {
    schema: "laminapdf.document/1",
    title: TITLE,
    lang: "en",
    page: { size: "A4", margins: MARGIN, footer: { center: "{title}, page {page} of {pages}" }, style: "small" },
    styles: {
      body: { font: "regular", size: 10 },
      strong: { font: "bold", size: 10 },
      title: { font: "bold", size: 18 },
      small: { font: "regular", size: 8 },
    },
    blocks: [
      { type: "heading", level: 1, text: TITLE, style: "title" },
      { type: "paragraph", text: "Office supplies delivered in October, to be paid within 30 days." },
      table(),
    ],
  };
  b.flow(document, { fonts: { regular: Font.fromBytes(regular), bold: Font.fromCollection(bold, 1) } });
  using doc = b.finish();
  return doc.save();
}

/** Sixty lines of products, a header row and a footer row with the total. */
function table(): BlockSpec {
  const number = (text: string, style = "body"): CellSpec => ({ text, style, align: "end" });
  const header = ["Item", "Description", "Qty", "Amount"].map(
    (text, i): CellSpec => ({ text, style: "strong", ...(i >= 2 ? { align: "end" } : {}) }),
  );
  let total = 0;
  const rows = Array.from({ length: LINES }, (_, i) => {
    const [name, cents] = PRODUCTS[i % PRODUCTS.length] ?? ["", 0];
    const qty = (i % 5) + 1;
    total += qty * cents;
    return {
      cells: [
        { text: String(i + 1).padStart(3, "0") },
        { text: name },
        number(String(qty)),
        number(money(qty * cents)),
      ],
    };
  });
  return {
    type: "table",
    columns: ["auto", "auto", "auto", "auto"],
    padding: PADDING,
    frame: { width: RULE_WIDTH, color: RULE },
    headerRule: { width: RULE_WIDTH, color: RULE },
    rowRules: { width: ROW_RULE_WIDTH, color: ROW_RULE },
    headerBackground: HEADER_SHADE,
    header: [{ cells: header }],
    rows,
    footer: [{ cells: [{ text: "Total", style: "strong", span: 3 }, number(money(total), "strong")] }],
  };
}

function money(cents: number): string {
  return `${Math.floor(cents / 100)}.${String(cents % 100).padStart(2, "0")}`;
}
```

**Rust**

`crates/laminapdf/examples/flowing_invoice.rs`:

```rust
//! An invoice whose table runs over three pages: its header row repeats at the top of each
//! page, no row is split between pages, and the total stays on the page of the last row.
//!
//! The columns fit their widest cell, and numbers line up on the right. The output is tagged
//! (one `Table` element across the pages) and checked against PDF/UA-1. Run it with
//! `cargo run -p laminapdf --example flowing_invoice -- regular.ttf bold.ttc`.

use std::error::Error;

use laminapdf::flow::{
    ColumnWidth, Flow, FlowBlock, Margins, PageRegion, PageTemplate, Paragraph, Table, TableCell, TableRow, TableRule,
    TextStyle,
};
use laminapdf::{Alignment, BuildOptions, Builder, Color, Font, Metadata, PageSize, Profile, SaveOptions};

const TITLE: &str = "Invoice 2026-114";
const PRODUCTS: [(&str, usize); 6] = [
    ("Paper, A4, 80 g, box of 2,500 sheets", 2_450),
    ("Toner cartridge, black", 6_900),
    ("Desk lamp with a weighted base and an adjustable arm", 4_275),
    ("Stapler", 1_290),
    ("Archive boxes, set of ten, with lids", 3_150),
    ("Ballpoint pens, box of fifty", 1_875),
];

fn main() -> Result<(), Box<dyn Error>> {
    let paths: Vec<String> = std::env::args().skip(1).collect();
    let [regular, bold] = paths.as_slice() else {
        return Err("arguments: <regular.ttf> <bold.ttc>".into());
    };
    let pdf = build(BuildOptions::new(), std::fs::read(regular)?, std::fs::read(bold)?)?;
    std::fs::create_dir_all("target/examples-out")?;
    Ok(std::fs::write("target/examples-out/flowing_invoice.pdf", pdf)?)
}

/// The invoice's bytes. `options` can fix the file ID, the producer and the creation date.
pub(crate) fn build(options: BuildOptions, regular: Vec<u8>, bold: Vec<u8>) -> Result<Vec<u8>, Box<dyn Error>> {
    let mut b = Builder::new(options.with_profile(Profile::PdfUa1));
    b.metadata(Metadata::default().with_title(TITLE));
    b.lang("en");
    let body = TextStyle::new(&b.font(Font::from_bytes(regular)?)?, 10.0);
    let strong = TextStyle::new(&b.font(Font::from_collection(bold, 1)?)?, 10.0);
    let footer = PageRegion::new().with_center("{title}, page {page} of {pages}");
    let pages = PageTemplate::new(PageSize::A4)
        .with_margins(Margins::all(56.0))
        .with_furniture(None, Some(footer), &body.clone().with_size(8.0));
    let flow = Flow::new(pages)
        .with_title(TITLE)
        .block(FlowBlock::Heading(
            1,
            Paragraph::plain(TITLE, &strong.clone().with_size(18.0)),
        ))
        .block(FlowBlock::Paragraph(Paragraph::plain(
            "Office supplies delivered in October, to be paid within 30 days.",
            &body,
        )))
        .block(FlowBlock::Table(table(&body, &strong)));
    b.flow(&flow)?;
    Ok(b.finish()?.save(&SaveOptions::new())?.bytes)
}

/// Sixty lines of products, a header row and a footer row with the total.
fn table(body: &TextStyle, strong: &TextStyle) -> Table {
    let number = |text: String, style: &TextStyle| TableCell::plain(text, style).with_align(Alignment::End);
    let header = ["Item", "Description", "Qty", "Amount"]
        .iter()
        .enumerate()
        .fold(TableRow::new(), |row, (i, text)| {
            let cell = TableCell::plain(*text, strong);
            row.cell(if i >= 2 { cell.with_align(Alignment::End) } else { cell })
        });
    let mut table = Table::new([ColumnWidth::Auto; 4])
        .with_padding(6.0)
        .with_frame(TableRule::new(0.75, Color::rgb8(0x66, 0x66, 0x66)))
        .with_header_rule(TableRule::new(0.75, Color::rgb8(0x66, 0x66, 0x66)))
        .with_row_rules(TableRule::new(0.25, Color::rgb8(0xb3, 0xb3, 0xb3)))
        .with_header_background(Color::rgb8(0xeb, 0xeb, 0xeb))
        .header(header);
    let mut total = 0;
    for (i, (name, cents)) in PRODUCTS.iter().cycle().take(60).enumerate() {
        let qty = i % 5 + 1;
        total += qty * cents;
        table = table.row(
            TableRow::new()
                .cell(TableCell::plain(format!("{:03}", i + 1), body))
                .cell(TableCell::plain(*name, body))
                .cell(number(qty.to_string(), body))
                .cell(number(money(qty * cents), body)),
        );
    }
    let total_row = TableRow::new()
        .cell(TableCell::plain("Total", strong).with_span(3))
        .cell(number(money(total), strong));
    table.footer(total_row)
}

fn money(cents: usize) -> String {
    format!("{}.{:02}", cents / 100, cents % 100)
}
```

**Result:** [flowing_invoice.pdf](https://docs.laminapdf.com/examples/pdf/flowing_invoice.pdf)

The result is three A4 pages: [open the reference PDF](https://docs.laminapdf.com/examples/pdf/flowing_invoice.pdf).

A table follows three rules when it crosses a page:

- **The header rows repeat** at the top of every page the table continues on.
- **A body row never splits.** A row that doesn't fit what is left of a page starts the next one, and a row taller than a page fails with `flow-content-too-large`.
- **The footer rows stay with the last body row,** so a total never starts a page alone. When the footer doesn't fit under the last row, that row moves to the next page with the footer.

Columns get their widths once, for the whole table, so they line up from page to page. A column is `Fixed` (a width in points), `Auto` (as wide as its widest cell) or a `Fraction` (a share of the width the others leave). When automatic columns are too wide together, the widest ones narrow first, so a column of numbers keeps its width while the descriptions wrap.

A cell holds one or more paragraphs. `with_align` lines its text up at the start, centre or end (numbers at the end), and `with_vertical_align` places it in the row. `with_span` stretches a cell over several columns, as the invoice's "Total" does. Rules come from `with_frame`, `with_header_rule` and `with_row_rules`, and `with_header_background` shades the header rows. In a JSON document, as in the TypeScript invoice, a cell takes `align`, `valign` and `span`. The table takes `padding`, `frame`, `headerRule`, `rowRules` and `headerBackground`, with colours written as `#rrggbb`.

## Footnotes, a contents page and bookmarks

A footnote is part of a paragraph: `Paragraph::note(body)` puts a raised number where you add it and the note's body at the foot of the same page. In JSON, a span's `note` does the same. Notes are numbered 1, 2, 3 through the flow. A short rule separates them from the text above, and the page's footer stays below them. When a line's notes don't fit at the foot of its page, the line moves to the next page with them.

A contents block, `FlowBlock::Contents`, lists the flow's headings with the page each one starts on. Each entry is indented by its heading's level, with dots leading to its number, and the entry links to its heading. By default it lists levels 1 and 2; `Contents::with_levels` changes that, as does `levels` in JSON. The report in the example puts its contents on the cover.

The flow can't know the page numbers before it has laid the pages out. So it lays everything out, writes the numbers it found into the contents page, and lays out again to check them. Each entry keeps room for a number of up to four digits, so the contents page keeps its length and the second layout confirms the numbers.

Every heading also gets a bookmark. PDF readers list the bookmarks in their sidebar, nested by heading level, and each one goes to its heading. Turn them off with `Flow::with_outline(false)`, or `outline: false` in JSON.

## Write it in Markdown

You can also write the document in Markdown and let `Builder::markdown` turn it into blocks. Each Markdown construct becomes the block you would build by hand: a `#` heading becomes a heading, a list becomes a list, and a table becomes a table that runs across pages, its header row repeated on each. A link to `#a-heading` goes to the heading of that name, written the way GitHub writes it: lower case, with hyphens for spaces. Front matter at the top, plain `key: value` lines between two `---` lines, sets the document's title, language, author and subject.

The example below builds a short report from a Markdown file. It gives the fonts, the page template and a contents page in `MarkdownBuildOptions`, and it passes the one image the Markdown names. The builder reads no files and fetches nothing, so it leaves out an image you don't pass and adds a `markdown-image-missing` warning. It leaves out raw HTML too, with `markdown-html-ignored`. Both warnings give the position in the Markdown, never its text.

In TypeScript, `builder.markdown(text, options)` takes the same options, with the fonts as `Font`s and the images as `Image`s by the name the Markdown uses. It needs the text module, `init({ text: true })`. Both languages write the same bytes:

**TypeScript**

`packages/sdk/examples/markdown-to-pdf.ts`:

```ts
// A report written in Markdown, turned into tagged pages that flow: headings, paragraphs with
// emphasis, code and links, a photo, lists and a task list, a block quote, a table that runs
// across pages, a code block, footnotes, and paragraphs in Arabic and Hindi.
//
// The front matter gives the title and language, a contents page lists the headings, and the
// output meets PDF/UA-1, so every font is embedded. The builder reads no files: the fonts and the
// one photo the Markdown names are passed in, and nothing is fetched.
import { Builder, type BuilderOptions, Font, Image } from "@laminapdf/sdk";

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
 * @param markdown - the report's Markdown.
 * @param fonts - the fonts to set it in.
 * @param photo - the JPEG the Markdown shows as `photo.jpg`.
 * @param options - can fix the file ID, the producer and the creation date.
 */
export function markdownToPdf(
  markdown: string,
  fonts: ReportFonts,
  photo: Uint8Array,
  options: BuilderOptions = {},
): Uint8Array<ArrayBuffer> {
  using b = new Builder({ ...options, profiles: ["PDF/UA-1"] });
  const report = b.markdown(markdown, {
    fonts: {
      regular: Font.fromBytes(fonts.regular),
      bold: Font.fromCollection(fonts.bold, 1),
      fallbacks: [Font.fromBytes(fonts.arabic), Font.fromBytes(fonts.devanagari)],
    },
    images: { "photo.jpg": Image.jpeg(photo) },
    // A5 pages, so the table runs across two of them, with the page number in the footer.
    page: { size: "A5", footer: { center: "Page {page} of {pages}" }, furnitureSize: 8 },
    contents: 2,
  });
  using doc = b.finish();
  const [warning] = doc.warnings;
  if (warning !== undefined) throw new Error(`the Markdown has something the PDF leaves out: ${warning.code}`);
  if (report.pages.length === 0) throw new Error("the report has no pages");
  return doc.save();
}
```

**Rust**

`crates/laminapdf/examples/markdown_to_pdf.rs`:

```rust
//! A report written in Markdown, turned into tagged pages that flow: headings, paragraphs with
//! emphasis, code and links, a photo, lists and a task list, a block quote, a table that runs
//! across pages, a code block, footnotes, and paragraphs in Arabic and Hindi.
//!
//! The front matter gives the title and language, a contents page lists the headings, and the
//! output is checked against PDF/UA-1, so every font is embedded. Run it with `cargo run -p
//! laminapdf --example markdown_to_pdf -- report.md regular.ttf bold.ttc arabic.ttf
//! devanagari.ttf photo.jpg`.

use std::error::Error;

use laminapdf::flow::{PageRegion, PageTemplate, TextStyle};
use laminapdf::markdown::{MarkdownBuildOptions, MarkdownFonts};
use laminapdf::{BuildOptions, Builder, Font, Image, PageSize, Profile, SaveOptions};

fn main() -> Result<(), Box<dyn Error>> {
    let paths: Vec<String> = std::env::args().skip(1).collect();
    let [markdown, regular, bold, arabic, hindi, photo] = paths.as_slice() else {
        return Err("arguments: <report.md> <regular.ttf> <bold.ttc> <arabic.ttf> <devanagari.ttf> <photo.jpg>".into());
    };
    let read = |path: &String| std::fs::read(path);
    let fonts = [read(regular)?, read(bold)?, read(arabic)?, read(hindi)?];
    let markdown = std::fs::read_to_string(markdown)?;
    let pdf = build(BuildOptions::new(), &markdown, fonts, read(photo)?)?;
    std::fs::create_dir_all("target/examples-out")?;
    Ok(std::fs::write("target/examples-out/markdown_to_pdf.pdf", pdf)?)
}

/// The report's bytes. `options` can fix the file ID, the producer and the creation date.
pub(crate) fn build(
    options: BuildOptions,
    markdown: &str,
    fonts: [Vec<u8>; 4],
    photo: Vec<u8>,
) -> Result<Vec<u8>, Box<dyn Error>> {
    let [regular, bold, arabic, hindi] = fonts;
    let mut b = Builder::new(options.with_profile(Profile::PdfUa1));
    let regular = b.font(Font::from_bytes(regular)?)?;
    let fonts = MarkdownFonts::new(&regular)
        .with_bold(&b.font(Font::from_collection(bold, 1)?)?)
        .with_fallback(&b.font(Font::from_bytes(arabic)?)?)
        .with_fallback(&b.font(Font::from_bytes(hindi)?)?);
    // A5 pages, so the table runs across two of them, with the page number in the footer.
    let footer = PageRegion::new().with_center("Page {page} of {pages}");
    let pages = PageTemplate::new(PageSize::A5).with_furniture(None, Some(footer), &TextStyle::new(&regular, 8.0));
    let options = MarkdownBuildOptions::new()
        .with_fonts(fonts)
        .with_template(pages)
        .with_contents(2)
        .with_image("photo.jpg", Image::jpeg(photo)?);
    let report = b.markdown(markdown, &options)?;
    if let Some(warning) = report.warnings.first() {
        return Err(format!("the Markdown has something the PDF leaves out: {}", warning.code()).into());
    }
    Ok(b.finish()?.save(&SaveOptions::new())?.bytes)
}
```

**Result:** [markdown_to_pdf.pdf](https://docs.laminapdf.com/examples/pdf/markdown_to_pdf.pdf)

The result is four A5 pages: [open the reference PDF](https://docs.laminapdf.com/examples/pdf/markdown_to_pdf.pdf).

The `laminapdf` command line does the same without code. It reads the images from the Markdown's folder and the fonts from `--font`:

```sh
laminapdf build report.md -o report.pdf --font regular=NotoSans-Regular.ttf --profile pdf-ua-1
```

If you pass no fonts, the builder sets the Markdown in Helvetica and Courier, two fonts every PDF reader has. They set only Latin text, and since they aren't embedded, a PDF/UA-1 build refuses them with `font-not-embedded`. Pass your own fonts with `MarkdownFonts` for other scripts or an accessible document.

## Write it as JSON

You can also describe the document as JSON, in the format `laminapdf.document/1`, and lay it out with `Builder::document`. The JSON names what you would build by hand. A `page` sets the size, margins, header and footer, `styles` each name a font and a size, and `blocks` are tagged by `type`, such as `paragraph`, `heading`, `list`, `table`, `figure` and `contents`. Fonts and images are named in the document, and their bytes come beside it in `Resources`, so the JSON holds no files.

A format with a schema suits programs and language models: they write the document, and the schema checks it before anything is drawn. Lamina PDF reads only the keys the format defines. A misspelt one fails with `invalid-document`, whose `data.pointer` names the value and whose `didYouMean` suggests the key it was close to.

A template is the same JSON with placeholders that `fill` replaces from your data. `$data` puts in a value and `$each` repeats a block or table row per item, with `$item` for a value of the current item. `$if` keeps a block only when a value is there. The [Fill an invoice from JSON](../../pages/recipes/invoice-from-json.md) recipe builds a two-page invoice this way.

In TypeScript, `builder.flow(document, { data, fonts, images })` takes the same document as plain objects, with the fonts as `Font`s and the images as `Image`s by name. The package's types follow the format, so your editor flags a misspelt key or an unknown block `type` before anything runs. Type a document as `FlowDocument`, and a template with placeholders as `FlowTemplate`. Given `data`, `flow` fills the template first. It needs the text module, `init({ text: true })`.

## Fields in headers and footers

A slot's text can hold fields, which the flow fills in on each page:

- `{page}`: the page's number in the document, from 1.
- `{pages}`: the number of pages in the document once the flow's pages are added.
- `{section}`: the title of the last level-1 heading that started on or before the page. Choose another level with `with_section_level`.
- `{title}`: the title you gave the flow.

Write `{{` and `}}` for a literal brace. `{pages}` is right on every page because no page is drawn until all of them are laid out.

## What the flow tags

The output is tagged unless you turn tagging off with `with_tagged(false)`. Paragraphs are `P` elements, headings `H1` to `H6`, lists `L` with their items, figures `Figure` with their alternate text, and links `Link`. A paragraph's language, taken from its first piece, is set on its element; a piece in another language is a `Span` with that language. A paragraph split across pages stays one element, so a screen reader reads it as one.

A table is one `Table` element however many pages it crosses, with `THead`, `TBody` and `TFoot` around its rows. Header cells are `TH` elements that head their columns, and a cell across several columns records how many it spans.

The header rows drawn again on later pages are marked as page furniture, like headers and footers, so a screen reader and Lamina PDF's extraction read the table's header once. The table's rules and background are marked as decoration too.

A footnote's number is a `Reference` and its body a `Note` with its own identifier, inside the paragraph that cites it. A contents page is a `TOC` element with a `TOCI` for each entry, each holding its links; its dots are marked as decoration.

Headers and footers are marked as page furniture rather than content: screen readers skip them, and Lamina PDF's own extraction leaves them out of the text.

## What to change

- **A cover:** give the template a first page with `with_first_page`, and start the body with a page break.
- **Another typeface for a word:** add a piece of text in a `TextStyle` with that font. List fallback fonts with `with_fallback` for characters the first font lacks.
- **Page numbers only:** pass a footer with one centre slot, `"{page}"`, and no header.

Next: [Accessible PDFs](../../pages/guides/accessible-pdfs.md), for what PDF/UA-1 checks and how to meet it.
