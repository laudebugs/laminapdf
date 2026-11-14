---
title: "Mark up a page"
description: "Highlight text in a PDF, leave a sticky note and propose a replacement word, each mark drawn the same in every PDF reader."
version: "0.4.0"
url: "https://docs.laminapdf.com/recipes/mark-up/"
---

# Mark up a page

Highlight text in a PDF, leave a sticky note and propose a replacement word, each mark drawn the same in every PDF reader.

You'll mark up a report's first page the way a reviewer does: highlight a line, leave a note beside the chart, and propose a better word. You need this for a review tool in an editor, or a script that flags every sentence a checker finds.

Viewers report a text selection as one box per line, and a click as a point, in page space: points from the page's top-left corner. `markText`, `noteAt` and `caretAt` turn those into the edit that adds the mark, in the page's [user space](../../pages/reference/glossary.md#user-space). `replaceText` adds a strike-out and a caret together:

**TypeScript**

`packages/sdk/examples/mark-up.ts`:

```ts
// Mark up a page: highlight a line of text, leave a note beside the chart, and propose a better word.
// The boxes are in page space (points from the page's top-left), as a viewer reports a text
// selection and a click. Each mark records who made it and when. In a tagged document a
// highlight's description, which screen readers read, is the text it marks.
import { Document } from "@laminapdf/sdk";

// Where the marks go on the example's input, the quarterly report, in points from the page's
// top-left: for your own file, take them from your viewer's selection and clicks.
/** The line "Widgets sold out in May.", as a selection gives it: `[x0, y0, x1, y1]`. */
const WIDGETS_LINE = [56, 358.241, 196.335, 373.223] as const;
/** The words "sold out" on that line. */
const SOLD_OUT = [114.902, 358.241, 156.713, 373.223] as const;
/** The note's spot, right of the chart. */
const BESIDE_CHART = [310, 140] as const;
/** The colour of a paper sticky note. */
const STICKY_YELLOW: [number, number, number] = [1, 0.82, 0];

/**
 * The document with a highlight, a note and a proposed word on its first page, saved
 * incrementally. `at` is when the marks were made, as a PDF date. Needs `init({ extract: true })`.
 */
export function markUp(bytes: Uint8Array, at: string): Uint8Array<ArrayBuffer> {
  using doc = Document.open(bytes);
  const byMe = { author: "Ada Lovelace", created: at, modified: at };

  // In a tagged document the highlight's description is already the marked text; add who made it.
  const highlight = doc.markText(0, "highlight", [WIDGETS_LINE]);
  highlight.common = { ...highlight.common, ...byMe };

  const note = doc.noteAt(0, BESIDE_CHART, "Add June's figures?", "comment");
  note.common = { ...note.common, ...byMe, color: STICKY_YELLOW };

  doc.apply([
    { op: "addAnnot", args: highlight },
    { op: "addAnnot", args: note },
  ]);

  // "sold out" reads better as "ran out": a strike-out and a caret, applied as one undo step.
  doc.replaceText(0, [SOLD_OUT], "ran out", byMe);

  return doc.save({ mode: "incremental" });
}
```

**Rust**

`crates/laminapdf/examples/mark_up.rs`:

````rust
//! Mark up a page: highlight a line of text, leave a note beside the chart, and propose a better word.
//!
//! The boxes are in page space (points from the page's top-left), as a viewer reports a text
//! selection and a click. Each mark records who made it and when. In a tagged document a
//! highlight's description, which screen readers read, is the text it marks.
//!
//! ```sh
//! cargo run -p laminapdf --features extract --example mark_up -- in.pdf <time> [out-dir]   # the time as a PDF date
//! ```

use std::error::Error;
use std::path::PathBuf;

use laminapdf::{
    AnnotColor, AnnotCommon, Document, MarkupKind, NoteIcon, Op, OpenOptions, PdfDate, PointPt, Pt, RectPt, SaveMode,
    SaveOptions,
};

fn main() -> Result<(), Box<dyn Error>> {
    let mut args = std::env::args().skip(1);
    let usage = "usage: mark_up in.pdf <time> [out-dir]";
    let input = PathBuf::from(args.next().ok_or(usage)?);
    // When the marks were made, which your app takes from its clock.
    let at: PdfDate = args.next().ok_or(usage)?.parse()?;
    let out = PathBuf::from(args.next().unwrap_or_else(|| "target/examples-out".into()));
    std::fs::create_dir_all(&out)?;
    let marked = mark_up(std::fs::read(input)?, at)?;
    std::fs::write(out.join("mark_up.pdf"), marked)?;
    Ok(())
}

// Where the marks go on the example's input, the quarterly report, in points from the page's
// top-left: for your own file, take them from your viewer's selection and clicks, or from
// `page_text`'s lines.
/// The line "Widgets sold out in May.", as a selection gives it: `[x0, y0, x1, y1]`.
const WIDGETS_LINE: [f64; 4] = [56.0, 358.241, 196.335, 373.223];
/// The words "sold out" on that line.
const SOLD_OUT: [f64; 4] = [114.902, 358.241, 156.713, 373.223];
/// The note's spot, right of the chart.
const BESIDE_CHART: [f64; 2] = [310.0, 140.0];
/// The colour of a paper sticky note.
const STICKY_YELLOW: AnnotColor = AnnotColor::Rgb(1.0, 0.82, 0.0);

/// The document with a highlight, a note and a proposed word on its first page, saved
/// incrementally.
pub(crate) fn mark_up(bytes: Vec<u8>, at: PdfDate) -> Result<Vec<u8>, Box<dyn Error>> {
    let mut doc = Document::open_bytes(bytes, OpenOptions::new())?;
    let mut by_me = AnnotCommon::default();
    by_me.author = Some("Ada Lovelace".into());
    by_me.created = Some(at);
    by_me.modified = Some(at);

    // In a tagged document the highlight's description is already the marked text; add who
    // made it.
    let mut highlight = doc.mark_text(0, MarkupKind::Highlight, &[area(WIDGETS_LINE)])?;
    sign(&mut highlight.common, &by_me);

    let [x, y] = BESIDE_CHART;
    let spot = PointPt::new(Pt::new(x), Pt::new(y));
    let mut note = doc.note_at(0, spot, "Add June's figures?", NoteIcon::Comment)?;
    sign(&mut note.common, &by_me);
    note.common.color = Some(STICKY_YELLOW);

    doc.apply(vec![Op::from(highlight), Op::from(note)])?;

    // "sold out" reads better as "ran out": a strike-out and a caret, applied as one undo step.
    doc.replace_text(0, &[area(SOLD_OUT)], "ran out", by_me)?;

    let options = SaveOptions::new().with_mode(SaveMode::Incremental);
    Ok(doc.save(&options)?.bytes)
}

/// A box in page space from its corners.
fn area([x0, y0, x1, y1]: [f64; 4]) -> RectPt {
    RectPt::new(Pt::new(x0), Pt::new(y0), Pt::new(x1), Pt::new(y1))
}

/// Records on a mark who made it and when, from `by`.
fn sign(common: &mut AnnotCommon, by: &AnnotCommon) {
    common.author.clone_from(&by.author);
    common.created.clone_from(&by.created);
    common.modified.clone_from(&by.modified);
}
````

**Result:** [mark_up.pdf](https://docs.laminapdf.com/examples/pdf/mark_up.pdf)

The result is the original bytes, unchanged, followed by two revisions. "Widgets sold out in May." is highlighted in yellow, a speech-bubble note sits right of the chart, and "sold out" is struck through with a caret after it that holds "ran out".

## How it works

- `markText` takes one box per line of the selected text and a kind: `highlight`, `underline`, `strikeOut` or `squiggly`. Each box becomes one of the mark's quadrilaterals, so a mark over three lines covers only those lines.
- Lamina PDF draws every mark itself and stores the drawing in the file, so every reader that draws stored appearances, as pdf.js, PDFium and Acrobat do, shows the same mark. A highlight multiplies its colour with the page, which keeps the text under it legible.
- The report is a tagged PDF, the kind screen readers follow. In a tagged file, a mark's description (`contents`) is the text it marks, or the kind of mark when its boxes hold no text. The mark joins the document's structure right after that text.
- `noteAt` puts a 20-point icon with its top-left corner on the point, and a pop-up beside it that shows the comment. The note keeps its size at any zoom and stays upright when the page turns, as Acrobat's notes do.
- `replaceText` groups the strike-out with a caret, so a reader shows, moves and deletes them as one correction. The caret holds the replacement text. Both edits are one step: a single undo removes both.
- The author and dates come from your code, never from a clock inside Lamina PDF, so the same edit gives the same file wherever it runs.

## What to change

- **Another mark:** pass `"underline"`, `"strikeOut"` or `"squiggly"` to `markText`.
- **A colour or opacity:** set `common.color` (`[r, g, b]` from 0 to 1) and `common.opacity` before applying the edit.
- **A note's icon:** pass `"note"`, `"comment"`, `"key"`, `"help"`, `"paragraph"`, `"newParagraph"` or `"insert"`.
- **Change a mark later:** `updateAnnot` takes the ID from `createdAnnotations` and a new `color`, `opacity` or `spec`. Lamina PDF draws the mark again. `deleteAnnot` removes a mark, with its pop-up and the rest of its group.
- **A PDF/A or PDF/UA file:** the marks keep the file's claims, and `doc.plan()` warns before you save when an edit breaks one. A highlight breaks PDF/A-1, which forbids transparency ([`pdfa-transparency`](../../errors/pdfa-transparency.md)). A mark with an empty description breaks PDF/UA ([`annot-without-contents`](../../errors/annot-without-contents.md)).
- **A signed file:** you can mark up a [certified](../../pages/reference/glossary.md#docmdp) file that allows comments, or one with only approval signatures. Lamina PDF appends the marks as a new revision, and the signatures stay valid.

Next: [add links](../../pages/recipes/add-links.md), another kind of annotation made the same way.
