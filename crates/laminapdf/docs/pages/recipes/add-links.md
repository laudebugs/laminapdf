---
title: "Add links"
description: "Add links to a PDF page on a drawn box or on a line of text, give each a description screen readers read, and change where a link goes."
version: "0.4.0"
url: "https://docs.laminapdf.com/recipes/add-links/"
---

# Add links

Add links to a PDF page on a drawn box or on a line of text, give each a description screen readers read, and change where a link goes.

You'll add two links to a report's first page: one on a box over its chart, one on a line of text. You need this for a link tool in an editor, or a script that links every product name in a catalogue to its web page.

Viewers report a drawn box or a text selection in page space: points from the page's top-left corner. `linkOnArea` and `linkOnText` turn those boxes into the edit that adds the link, in the page's [user space](../../pages/reference/glossary.md#user-space). The example links the chart and the line "Widgets sold out in May." to two web pages, then changes the chart link's address:

**TypeScript**

`packages/sdk/examples/add-links.ts`:

```ts
// Add links to a page: one on a box drawn over a chart, one on a line of text, then point the first somewhere else.
// The boxes are in page space (points from the page's top-left), as a viewer reports a drawn box
// and a text selection. Each link gets a description, which screen readers read. In a tagged
// document the link on text joins the structure tree around the words it covers.
import { Document } from "@laminapdf/sdk";

/** The document with two links added to its first page, saved incrementally. Needs `init({ extract: true })`. */
export function addLinks(bytes: Uint8Array): Uint8Array<ArrayBuffer> {
  using doc = Document.open(bytes);

  // A box drawn over the chart.
  const onChart = doc.linkOnArea(0, [56, 140, 300, 240], { uri: "https://example.com/sales" });
  onChart.contents = "Sales figures";

  // The line "Widgets sold out in May.", as a selection gives it.
  const onText = doc.linkOnText(0, [[56, 358.241, 196.335, 373.223]], { uri: "https://example.com/widgets" });
  onText.contents = "Widget stock";

  const added = doc.apply([
    { op: "addLink", args: onChart },
    { op: "addLink", args: onText },
  ]);

  // The new links' IDs come back with the edit: change the first one's address.
  const chartLink = added.createdAnnotations[0];
  if (chartLink !== undefined) {
    doc.apply([{ op: "updateLink", args: { link: chartLink, target: { uri: "https://example.com/sales/2026" } } }]);
  }

  return doc.save({ mode: "incremental" });
}
```

**Rust**

`crates/laminapdf/examples/add_links.rs`:

````rust
//! Add links to a page: one on a box drawn over a chart, one on a line of text, then point the first somewhere else.
//!
//! The boxes are in page space (points from the page's top-left), as a viewer reports a drawn
//! box and a text selection. Each link gets a description, which screen readers read. In a
//! tagged document the link on text joins the structure tree around the words it covers.
//!
//! ```sh
//! cargo run -p laminapdf --features extract --example add_links -- in.pdf [out-dir]
//! ```

use std::error::Error;
use std::path::PathBuf;

use laminapdf::{Document, Op, OpenOptions, OutlineTarget, Pt, RectPt, SaveMode, SaveOptions, UpdateLink};

fn main() -> Result<(), Box<dyn Error>> {
    let mut args = std::env::args().skip(1);
    let input = PathBuf::from(args.next().ok_or("usage: add_links in.pdf [out-dir]")?);
    let out = PathBuf::from(args.next().unwrap_or_else(|| "target/examples-out".into()));
    std::fs::create_dir_all(&out)?;
    let edited = add(std::fs::read(input)?)?;
    std::fs::write(out.join("add_links.pdf"), edited)?;
    Ok(())
}

/// The document with two links added to its first page, saved incrementally.
pub(crate) fn add(bytes: Vec<u8>) -> Result<Vec<u8>, Box<dyn Error>> {
    let mut doc = Document::open_bytes(bytes, OpenOptions::new())?;
    // These boxes fit the example's input, the quarterly report; for your own file, take them
    // from your viewer's drawn box and selection, or from `page_text`'s lines.
    let points = |x0, y0, x1, y1| RectPt::new(Pt::new(x0), Pt::new(y0), Pt::new(x1), Pt::new(y1));

    // A box drawn over the chart.
    let chart = points(56.0, 140.0, 300.0, 240.0);
    let mut on_chart = doc.link_on_area(0, chart, OutlineTarget::Uri("https://example.com/sales".into()))?;
    on_chart.contents = Some("Sales figures".into());

    // The line "Widgets sold out in May.", as a selection gives it.
    let line = points(56.0, 358.241, 196.335, 373.223);
    let mut on_text = doc.link_on_text(0, &[line], OutlineTarget::Uri("https://example.com/widgets".into()))?;
    on_text.contents = Some("Widget stock".into());

    let added = doc.apply(vec![Op::from(on_chart), Op::from(on_text)])?;

    // The new links' IDs come back with the edit: change the first one's address.
    if let Some(chart_link) = added.created_annotations.first() {
        let mut update = UpdateLink::new(chart_link.clone());
        update.target = Some(OutlineTarget::Uri("https://example.com/sales/2026".into()));
        doc.apply(vec![Op::from(update)])?;
    }

    let options = SaveOptions::new().with_mode(SaveMode::Incremental);
    Ok(doc.save(&options)?.bytes)
}
````

**Result:** [add_links.pdf](https://docs.laminapdf.com/examples/pdf/add_links.pdf)

The result is the original bytes, unchanged, followed by two revisions. Neither link draws a border, so the page looks the same, but a click on the chart opens the 2026 sales page and a click on the line opens the widgets page.

## How it works

- `linkOnArea` takes one box, and `linkOnText` takes one box per line of the selected text. A link on text follows only its lines, so the gaps between them stay plain text.
- The report is a tagged PDF, the kind screen readers follow. In a tagged file, `linkOnText` also finds the words under the boxes, and the new link joins the document's structure around them.
- Each link takes a target: a page (`{ destination }`), a named destination (`{ named }`) or a web address (`{ uri }`), as a bookmark does.
- `contents` is the link's description, which screen readers read out. The accessible PDF standard, PDF/UA, asks every link for one.
- The edit's result lists each new link's ID in `createdAnnotations`. `updateLink` takes that ID to change a link's boxes, target or description, and `deleteLink` removes it. `pageLinks` lists the IDs of the links a page already has.

## What to change

- **A page of the file:** pass `{ destination: doc.destinationAt(page, view) }` as the target, with the view your reader shows.
- **An email address:** pass `{ uri: "mailto:sales@example.com" }`. Only absolute `http`, `https` and `mailto` addresses are allowed. Any other address, such as a `javascript:` one, fails with [`link-uri-refused`](../../errors/link-uri-refused.md).
- **Several lines:** pass every line's box to `linkOnText`, each `[x0, y0, x1, y1]`, in reading order.
- **A file that claims PDF/UA but isn't tagged:** the edit fails with [`link-not-tagged`](../../errors/link-not-tagged.md), since an untagged link would break the claim.
- **A signed file:** you can add links to a [certified](../../pages/reference/glossary.md#docmdp) file that allows comments, or to one with only approval signatures. Lamina PDF appends them as a new revision, and the signatures stay valid. A certification that forbids comments makes the save fail.

Next: [edit the bookmarks](../../pages/recipes/edit-bookmarks.md), which go to places in the same way.
