---
title: "Links, outlines and metadata"
description: "Add links, named destinations, an outline, page labels, viewer preferences and metadata to a document."
version: "0.2.0"
url: "https://docs.laminapdf.com/guides/links-outlines-and-metadata/"
---

# Links, outlines and metadata

Add links, named destinations, an outline, page labels, viewer preferences and metadata to a document.

You'll make a document readers can find their way around: links between pages and to the web, an outline (the bookmarks panel), page numbers that read i, ii, 1, 2, and a title. You need this for any document longer than a page or two.

## Link to a page or the web

`surface.link(rect, target, contents)` makes a clickable area. The target is a web address (`{ type: "uri" }`), a page (`{ type: "page", index }`) or a named destination (`{ type: "named", name }`). A text run takes a `link` option too. The builder checks every target when it finishes: a missing page fails with `page-index-out-of-range`, and a missing name with `unknown-destination`.

`builder.destination(name, index)` names a page, so links can point to it by name.

## Add an outline and page labels

- **Outline:** `builder.outline([...])` builds the bookmarks a viewer shows beside the pages, with nested entries, colour, bold, italic and `open` (an entry shown expanded).
- **Page labels:** `builder.pageLabels([{ start, style }])` sets the page numbers a viewer shows. The example below uses roman numerals for the front matter, then decimal from page 3.
- **Viewer preferences:** for example, `pageMode: "outlines"` opens the file with the outline showing.

## Set the metadata

`builder.metadata({ title, author, … })` sets the document's title, author and the rest (see Details).

This example builds four pages that link to each other and to the web, with an outline, page labels and the outline shown on open. It draws its labels as text, so it needs the text module. It sets no metadata ([Edit a PDF](../../pages/guides/edit-a-pdf.md) does). The example:

**TypeScript**

`packages/sdk/examples/links-and-outlines.ts`:

```ts
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
```

**Rust**

`crates/laminapdf/examples/links_and_outlines.rs`:

````rust
//! Links and outlines: four A5 pages joined by links, bookmarks and page labels.
//!
//! The title page has a URI link, a link to a page and one through a named destination; every
//! other page has a linked text run back to it. The outline (bookmarks) is nested and open in
//! the viewer, and the pages are labelled i, ii, 1, 2.
//!
//! ```sh
//! cargo run -p laminapdf --example links_and_outlines -- [out-dir]   # writes <out-dir>/links_and_outlines.pdf
//! ```

use std::error::Error;
use std::path::PathBuf;

use laminapdf::{
    BuildOptions, Builder, Color, Fit, FontHandle, LabelStyle, LinkTarget, OutlineEntry, PageLabel, PageMode, PageSize,
    PointPt, Pt, RectPt, SaveOptions, Standard14, Surface, ViewerPreferences,
};

const TITLES: [&str; 4] = ["Title page", "Contents", "Chapter 1", "Chapter 2"];
const MARGIN: f64 = 36.0;
const TITLE_AT: PointPt = PointPt::from_points(MARGIN, 60.0);
const BACK_AT: PointPt = PointPt::from_points(MARGIN, 100.0);
const TITLE_SIZE: Pt = Pt::new(18.0);
const LINK_SIZE: Pt = Pt::new(12.0);
/// The baselines of the title page's three links.
const LINK_BASELINES: [f64; 3] = [100.0, 125.0, 150.0];
/// A link's clickable box: this wide, this tall, starting this far above the baseline.
const LINK_WIDTH: f64 = 220.0;
const LINK_HEIGHT: f64 = 20.0;
const LINK_RISE: f64 = 15.0;

fn main() -> Result<(), Box<dyn Error>> {
    let out = PathBuf::from(std::env::args().nth(1).unwrap_or_else(|| "target/examples-out".into()));
    std::fs::create_dir_all(&out)?;
    std::fs::write(out.join("links_and_outlines.pdf"), build(BuildOptions::new())?)?;
    Ok(())
}

/// The document's bytes. `options` can fix the file ID, the producer and the creation date.
pub(crate) fn build(options: BuildOptions) -> Result<Vec<u8>, Box<dyn Error>> {
    let mut b = Builder::new(options);
    let helvetica = b.standard_font(Standard14::Helvetica)?;
    b.destination("chapter-2", 3, Fit::Page);
    for (index, title) in (0_u32..).zip(TITLES) {
        let mut page = b.page(PageSize::A5);
        let s = page.surface();
        s.text(title, &helvetica, TITLE_SIZE, TITLE_AT)?;
        if index == 0 {
            title_page_links(s, &helvetica)?;
        } else {
            s.text("Back to the title page", &helvetica, LINK_SIZE, BACK_AT)?
                .fill(Color::rgb(0.0, 0.2, 0.8))
                .link(LinkTarget::page(0), Some("Title page".into()));
        }
        page.finish()?;
    }
    navigation(&mut b)?;
    let mut doc = b.finish()?;
    Ok(doc.save(&SaveOptions::new())?.bytes)
}

/// The title page's links: to a website, to a page, and to a named destination.
fn title_page_links(s: &mut Surface, font: &FontHandle) -> Result<(), Box<dyn Error>> {
    let site = LinkTarget::Uri("https://example.com/".into());
    let links = [
        ("Visit example.com", site, Some("Example")),
        ("Go to chapter 1", LinkTarget::page(2), None),
        ("Go to chapter 2 (named)", LinkTarget::Named("chapter-2".into()), None),
    ];
    for ((label, target, contents), baseline) in links.into_iter().zip(LINK_BASELINES) {
        s.text(label, font, LINK_SIZE, PointPt::from_points(MARGIN, baseline))?;
        let area = RectPt::from_points(MARGIN, baseline - LINK_RISE, LINK_WIDTH, LINK_HEIGHT);
        s.link(area, target, contents.map(String::from));
    }
    Ok(())
}

/// The outline (bookmarks), the page labels, and the viewer opening with the outline shown.
fn navigation(b: &mut Builder) -> Result<(), Box<dyn Error>> {
    b.outline(vec![
        OutlineEntry::new("Title page", LinkTarget::page(0)),
        OutlineEntry::new("Contents", LinkTarget::page(1)).with_color(0.0, 0.0, 0.6),
        OutlineEntry::heading("Chapters")
            .open(true)
            .bold()
            .with_child(OutlineEntry::new("Chapter 1", LinkTarget::page(2)))
            .with_child(OutlineEntry::new("Chapter 2", LinkTarget::Named("chapter-2".into())).italic()),
    ])?;
    b.page_labels(vec![
        PageLabel::new(0, Some(LabelStyle::LowerRoman)),
        PageLabel::new(2, Some(LabelStyle::Decimal)),
    ])
    .viewer_preferences(ViewerPreferences::default().with_page_mode(PageMode::Outlines));
    Ok(())
}
````

**Result:** [links_and_outlines.pdf](https://docs.laminapdf.com/examples/pdf/links_and_outlines.pdf)

[See the output](../../pages/recipes/links-and-outlines.md).

> **Details:** The metadata goes into two places: an XMP packet, the XML that describes the file, and the Info dictionary, the older list of the same facts. Lamina PDF writes both from the same values, so they always agree.

Next: [accessible PDFs](../../pages/guides/accessible-pdfs.md).
