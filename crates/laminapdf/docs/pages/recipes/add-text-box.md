---
title: "Add a text box"
description: "Type on a PDF page with a text box or a callout that points at something, drawn the same in every PDF reader."
version: "0.7.0"
url: "https://docs.laminapdf.com/recipes/add-text-box/"
---

# Add a text box

Type on a PDF page with a text box or a callout that points at something, drawn the same in every PDF reader.

You'll type on a report's first page: a callout pointing at the chart, and a yellow note beside the total with one word in bold. You need this for a text tool in an editor, or a script that adds a review note to every file it checks.

Viewers report where a pointer went in page space: points from the page's top-left corner. `textBoxAt` and `calloutAt` turn a box and points there into the edit that adds a text box, in the page's [user space](../../pages/reference/glossary.md#user-space):

**TypeScript**

`packages/sdk/examples/add-text-box.ts`:

```ts
// Type on a page: a callout pointing at the chart, and a filled note beside the total with one
// word in bold.
// The boxes and points are in page space (points from the page's top-left), as a viewer reports
// where a pointer went. The boxes are in Helvetica, which every reader has, so the WebAssembly
// extract module can draw them too.
import { type AnnotColor, type AnnotCommon, Document, type RichText } from "@laminapdf/sdk";

// Where the boxes go on the example's input, the quarterly report, in points from the page's
// top-left: for your own file, take them from your viewer's pointer.
/** The callout's box, in the space right of the chart: `[x0, y0, x1, y1]`. */
const CALLOUT = [330, 168, 500, 190] as const;
/** The callout's line: from the chart's right edge, through a knee, to the box. */
const LEADER = [
  [257, 200],
  [300, 179],
  [330, 179],
] as const;
/** The note beside the total. */
const NOTE = [300, 452, 520, 486] as const;
/** A pale yellow, like a sticky note. */
const YELLOW: AnnotColor = [1, 0.96, 0.7];

/**
 * The document with a callout and a note typed on its first page, saved incrementally. `at` is
 * when the boxes were typed, as a PDF date.
 */
export function addTextBox(bytes: Uint8Array, at: string): Uint8Array<ArrayBuffer> {
  using doc = Document.open(bytes);
  const byMe = (): AnnotCommon => ({ author: "Ada Lovelace", created: at, modified: at });

  const callout = doc.calloutAt(0, CALLOUT, LEADER, "Label the chart's axes");
  callout.common = byMe();

  const text: RichText = [
    [{ text: "Check: " }, { text: "€ 4,300", bold: true }, { text: " should match the table's total." }],
  ];
  const note = doc.textBoxAt(0, NOTE, text);
  if (note.spec.type === "freeText") note.spec.fill = YELLOW;
  note.common = byMe();

  doc.apply([
    { op: "addAnnot", args: callout },
    { op: "addAnnot", args: note },
  ]);
  return doc.save({ mode: "incremental" });
}
```

**Rust**

`crates/laminapdf/examples/add_text_box.rs`:

````rust
//! Type on a page: a callout pointing at the chart, and a filled note beside the total with one
//! word in bold.
//!
//! The boxes and points are in page space (points from the page's top-left), as a viewer reports
//! where a pointer went. The boxes are in Helvetica, which every reader has, so the WebAssembly
//! extract module can draw them too.
//!
//! ```sh
//! cargo run -p laminapdf --example add_text_box -- in.pdf <time> [out-dir]   # the time as a PDF date
//! ```

use std::error::Error;
use std::path::PathBuf;

use laminapdf::{
    AnnotColor, AnnotCommon, AnnotSpec, Document, Op, OpenOptions, PdfDate, PointPt, Pt, RectPt, RichText, SaveMode,
    SaveOptions, TextSpan,
};

fn main() -> Result<(), Box<dyn Error>> {
    let mut args = std::env::args().skip(1);
    let usage = "usage: add_text_box in.pdf <time> [out-dir]";
    let input = PathBuf::from(args.next().ok_or(usage)?);
    // When the boxes were typed, which your app takes from its clock.
    let at: PdfDate = args.next().ok_or(usage)?.parse()?;
    let out = PathBuf::from(args.next().unwrap_or_else(|| "target/examples-out".into()));
    std::fs::create_dir_all(&out)?;
    let typed = add_text_box(std::fs::read(input)?, at)?;
    std::fs::write(out.join("add_text_box.pdf"), typed)?;
    Ok(())
}

// Where the boxes go on the example's input, the quarterly report, in points from the page's
// top-left: for your own file, take them from your viewer's pointer.
/// The callout's box, in the space right of the chart: `[x0, y0, x1, y1]`.
const CALLOUT: [f64; 4] = [330.0, 168.0, 500.0, 190.0];
/// The callout's line: from the chart's right edge, through a knee, to the box.
const LEADER: [[f64; 2]; 3] = [[257.0, 200.0], [300.0, 179.0], [330.0, 179.0]];
/// The note beside the total.
const NOTE: [f64; 4] = [300.0, 452.0, 520.0, 486.0];
/// A pale yellow, like a sticky note.
const YELLOW: AnnotColor = AnnotColor::Rgb(1.0, 0.96, 0.7);

/// The document with a callout and a note typed on its first page, saved incrementally.
pub(crate) fn add_text_box(bytes: Vec<u8>, at: PdfDate) -> Result<Vec<u8>, Box<dyn Error>> {
    let mut doc = Document::open_bytes(bytes, OpenOptions::new())?;
    let by_me = || {
        let mut common = AnnotCommon::default();
        common.author = Some("Ada Lovelace".into());
        common.created = Some(at);
        common.modified = Some(at);
        common
    };

    let leader: Vec<PointPt> = LEADER.iter().map(|&p| point(p)).collect();
    let mut callout = doc.callout_at(0, area(CALLOUT), &leader, RichText::plain("Label the chart's axes"))?;
    callout.common = by_me();

    let text = RichText {
        paragraphs: vec![vec![
            TextSpan::plain("Check: "),
            TextSpan::plain("€ 4,300").with_bold(true),
            TextSpan::plain(" should match the table's total."),
        ]],
    };
    let mut note = doc.text_box_at(0, area(NOTE), text)?;
    if let AnnotSpec::FreeText(spec) = &mut note.spec {
        spec.fill = Some(YELLOW);
    }
    note.common = by_me();

    doc.apply(vec![Op::from(callout), Op::from(note)])?;
    let options = SaveOptions::new().with_mode(SaveMode::Incremental);
    Ok(doc.save(&options)?.bytes)
}

/// A point in page space.
fn point([x, y]: [f64; 2]) -> PointPt {
    PointPt::new(Pt::new(x), Pt::new(y))
}

/// A box in page space from its corners.
fn area([x0, y0, x1, y1]: [f64; 4]) -> RectPt {
    RectPt::new(Pt::new(x0), Pt::new(y0), Pt::new(x1), Pt::new(y1))
}
````

**Result:** [add_text_box.pdf](https://docs.laminapdf.com/examples/pdf/add_text_box.pdf)

The result is the original bytes, unchanged, followed by one revision. A callout reading "Label the chart's axes" points at the chart's right edge, and a yellow note beside the total puts "€ 4,300" in bold.

## How it works

- Each method gives an edit for a box in 12-point Helvetica, left-aligned, with a 1-point border. Change its `spec` before you apply it: the `font`, `size`, `align`, the `fill` behind the text and the `border`.
- The text is a string, a paragraph per line, or rich text: an array of paragraphs, each an array of spans that can be `bold` or `italic`. Lamina PDF breaks lines at the box's width.
- Lamina PDF draws every box itself and stores the drawing in the file, so every reader that draws stored appearances, as pdf.js, PDFium and Acrobat do, shows the same box. The text is upright as the page is shown, even on a turned page.
- A box keeps its text in the keys other apps read too: the plain text as its description, its font, size and colour, and the bold and italic words as rich text. An app that draws boxes itself from those keys shows plain text where it ignores rich text.
- The author and dates come from your code, never from a clock inside Lamina PDF, so the same edit gives the same file wherever it runs.

## What to change

- **Another font:** set `spec.font` to `"times"` or `"courier"`, which every reader has. For any other font, add its bytes with `addBlob` and set `spec.font` to the ID it returns. Lamina PDF embeds the characters the box uses.
- **Characters the font lacks:** set `spec.fallbacks` to the IDs of fonts that have them, for example a Noto font for Chinese or Arabic. A character no font has fails the edit with [`annotation-glyph-missing`](../../errors/annotation-glyph-missing.md).
- **Typewriter text:** set `spec.intent` to `{ type: "typewriter" }` for text with no border or fill, as when filling in a form that has no fields. Its lines end only where yours do, never at the box's edge, as in Acrobat's typewriter tool.
- **Change a box later:** `updateAnnot` with a `freeText` patch replaces its text, font, size or any other field, and Lamina PDF draws it again. A box's contents are its text, so change them with the patch's `text`. A box another app made keeps its look until you change it. Then Lamina PDF keeps its paragraphs, bold and italic, and warns with [`rich-text-simplified`](../../errors/rich-text-simplified.md) when the box held more.
- **In the browser:** boxes in Helvetica, Times and Courier work in the extract and text modules. A font given by its bytes, and fallback fonts, need the text module: open the document after `init({ text: true })`. Elsewhere the edit fails with [`appearance-needs-text`](../../errors/appearance-needs-text.md).
- **A PDF/A file:** Helvetica, Times and Courier aren't embedded, which PDF/A forbids, so `doc.plan()` warns with [`pdfa-font-not-embedded`](../../errors/pdfa-font-not-embedded.md). Give the box a font by its bytes instead.

Next: [draw on a page](../../pages/recipes/draw-on-page.md), with ink, shapes and arrows made the same way.
