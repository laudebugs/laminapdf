---
title: "Draw on a page"
description: "Draw freehand ink, boxes, clouds, arrows and polygons on a PDF page, each drawn the same in every PDF reader."
version: "0.8.0"
url: "https://docs.laminapdf.com/recipes/draw-on-page/"
---

# Draw on a page

Draw freehand ink, boxes, clouds, arrows and polygons on a PDF page, each drawn the same in every PDF reader.

You'll draw on a report's first page as a reviewer does: a cloud around a row of the table, an arrow at the chart and a wavy line under the total. You need this for a drawing tool in an editor, or a script that circles what a checker finds.

Viewers report where a pointer went as points in page space: points from the page's top-left corner. `inkAt`, `shapeAt`, `lineAt` and `polyAt` turn those points into the page's [user space](../../pages/reference/glossary.md#user-space) and give the edit that adds the drawing:

**TypeScript**

`packages/sdk/examples/draw-on-page.ts`:

```ts
// Draw on a page: a cloud around a row of the table, an arrow pointing at the chart, and a
// freehand underline below the total.
// The points and boxes are in page space (points from the page's top-left), as a viewer reports
// where a pointer went. Each mark records who drew it and when, and describes itself for screen
// readers.
import { type AnnotColor, type AnnotCommon, Document } from "@laminapdf/sdk";

// Where the marks go on the example's input, the quarterly report, in points from the page's
// top-left: for your own file, take them from your viewer's pointer.
/** Around the table's Q2 row: `[x0, y0, x1, y1]`. */
const Q2_ROW = [48, 317, 340, 338] as const;
/** From the empty space right of the chart to its right edge: `[x, y]` each. */
const ARROW = [
  [420, 180],
  [264, 200],
] as const;
/** A wavy underline below "Total sales", as a pen traced it. */
const UNDERLINE = [
  [56, 482],
  [72, 478],
  [88, 484],
  [104, 478],
  [120, 484],
  [136, 478],
  [152, 484],
  [168, 478],
  [180, 482],
] as const;
/** A reviewer's red pen. */
const RED: AnnotColor = [0.85, 0.1, 0.1];

/**
 * The document with a cloud, an arrow and an underline drawn on its first page, saved
 * incrementally. `at` is when the marks were drawn, as a PDF date.
 */
export function drawOnPage(bytes: Uint8Array, at: string): Uint8Array<ArrayBuffer> {
  using doc = Document.open(bytes);
  const byMe = (contents: string): AnnotCommon => ({
    contents,
    author: "Ada Lovelace",
    created: at,
    modified: at,
    color: RED,
  });

  const cloud = doc.shapeAt(0, "square", Q2_ROW);
  if (cloud.spec.type === "shape") cloud.spec.border = { width: 1.5, style: { type: "cloudy", intensity: 0.5 } };
  cloud.common = byMe("Check the Q2 figures");

  const [from, to] = ARROW;
  const arrow = doc.lineAt(0, from, to, ["none", "closedArrow"]);
  if (arrow.spec.type === "line") {
    arrow.spec.border = { width: 2, style: { type: "solid" } };
    arrow.spec.interior = RED;
  }
  arrow.common = byMe("Label the chart's axes");

  const underline = doc.inkAt(0, [UNDERLINE], 1.5);
  underline.common = byMe("Underlined: the total");

  doc.apply([
    { op: "addAnnot", args: cloud },
    { op: "addAnnot", args: arrow },
    { op: "addAnnot", args: underline },
  ]);
  return doc.save({ mode: "incremental" });
}
```

**Rust**

`crates/laminapdf/examples/draw_on_page.rs`:

````rust
//! Draw on a page: a cloud around a row of the table, an arrow pointing at the chart, and a
//! freehand underline below the total.
//!
//! The points and boxes are in page space (points from the page's top-left), as a viewer reports
//! where a pointer went. Each mark records who drew it and when, and describes itself for screen
//! readers.
//!
//! ```sh
//! cargo run -p laminapdf --example draw_on_page -- in.pdf <time> [out-dir]   # the time as a PDF date
//! ```

use std::error::Error;
use std::path::PathBuf;

use laminapdf::{
    AnnotColor, AnnotCommon, AnnotSpec, Border, Document, LineEnding, Op, OpenOptions, PdfDate, PointPt, Pt, RectPt,
    SaveMode, SaveOptions, ShapeKind,
};

fn main() -> Result<(), Box<dyn Error>> {
    let mut args = std::env::args().skip(1);
    let usage = "usage: draw_on_page in.pdf <time> [out-dir]";
    let input = PathBuf::from(args.next().ok_or(usage)?);
    // When the marks were drawn, which your app takes from its clock.
    let at: PdfDate = args.next().ok_or(usage)?.parse()?;
    let out = PathBuf::from(args.next().unwrap_or_else(|| "target/examples-out".into()));
    std::fs::create_dir_all(&out)?;
    let drawn = draw_on_page(std::fs::read(input)?, at)?;
    std::fs::write(out.join("draw_on_page.pdf"), drawn)?;
    Ok(())
}

// Where the marks go on the example's input, the quarterly report, in points from the page's
// top-left: for your own file, take them from your viewer's pointer.
/// Around the table's Q2 row: `[x0, y0, x1, y1]`.
const Q2_ROW: [f64; 4] = [48.0, 317.0, 340.0, 338.0];
/// From the empty space right of the chart to its right edge: `[x, y]` each.
const ARROW: [[f64; 2]; 2] = [[420.0, 180.0], [264.0, 200.0]];
/// A wavy underline below "Total sales", as a pen traced it.
const UNDERLINE: [[f64; 2]; 9] = [
    [56.0, 482.0],
    [72.0, 478.0],
    [88.0, 484.0],
    [104.0, 478.0],
    [120.0, 484.0],
    [136.0, 478.0],
    [152.0, 484.0],
    [168.0, 478.0],
    [180.0, 482.0],
];
/// A reviewer's red pen.
const RED: AnnotColor = AnnotColor::Rgb(0.85, 0.1, 0.1);

/// The document with a cloud, an arrow and an underline drawn on its first page, saved
/// incrementally.
pub(crate) fn draw_on_page(bytes: Vec<u8>, at: PdfDate) -> Result<Vec<u8>, Box<dyn Error>> {
    let mut doc = Document::open_bytes(bytes, OpenOptions::new())?;
    let by_me = |description: &str| {
        let mut common = AnnotCommon::default();
        common.author = Some("Ada Lovelace".into());
        common.created = Some(at);
        common.modified = Some(at);
        common.color = Some(RED);
        common.contents = Some(description.into());
        common
    };

    let mut cloud = doc.shape_at(0, ShapeKind::Square, area(Q2_ROW))?;
    if let AnnotSpec::Shape(shape) = &mut cloud.spec {
        shape.border = Border::cloudy(1.5, 0.5);
    }
    cloud.common = by_me("Check the Q2 figures");

    let [from, to] = ARROW.map(point);
    let mut arrow = doc.line_at(0, from, to, [LineEnding::None, LineEnding::ClosedArrow])?;
    if let AnnotSpec::Line(line) = &mut arrow.spec {
        line.border = Border::solid(2.0);
        line.interior = Some(RED);
    }
    arrow.common = by_me("Label the chart's axes");

    let mut underline = doc.ink_at(0, &[UNDERLINE.map(point).to_vec()], 1.5)?;
    underline.common = by_me("Underlined: the total");

    doc.apply(vec![Op::from(cloud), Op::from(arrow), Op::from(underline)])?;
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

**Result:** [draw_on_page.pdf](https://docs.laminapdf.com/examples/pdf/draw_on_page.pdf)

The result is the original bytes, unchanged, followed by one revision. The Q2 row sits in a red cloud, a red arrow points at the chart from the space beside it, and a hand-drawn wave runs under "Total sales".

## How it works

- Each method gives an edit with a 1-point red outline and nothing filled. Change its `spec` before you apply it: the border's width and style (`solid`, `dashed` with a pattern, or `cloudy` with an intensity), the `interior` colour that fills a shape or a closed arrowhead, and a line's `ends`.
- `inkAt` takes one list of points per stroke and draws a smooth curve through them, as a pen does. Lamina PDF drops each point less than 0.25 pt from the last one it kept, so a pen that reports every pixel doesn't bloat the file.
- Lamina PDF draws every mark itself and stores the drawing in the file, so every reader that draws stored appearances, as pdf.js, PDFium and Acrobat do, shows the same mark. The cloud's bumps and the arrowheads are drawn at sizes taken from the border's width and intensity.
- The report is a tagged PDF, the kind screen readers follow. A drawing marks no text, so give each one a description (`contents`) that says what it means. The mark joins the document's structure, and a screen reader reads the description.
- The author and dates come from your code, never from a clock inside Lamina PDF, so the same edit gives the same file wherever it runs.

## What to change

- **A circle:** pass `"circle"` to `shapeAt`. It draws the ellipse inside the box.
- **Another line ending:** `ends` takes two of `none`, `square`, `circle`, `diamond`, `openArrow`, `closedArrow`, `butt`, `rOpenArrow`, `rClosedArrow` and `slash`, for the start and the end.
- **A measured line:** set the line's `leader` to draw it a set distance from its two points, with a leader line back to each, like a dimension on a technical drawing.
- **A polygon or polyline:** `polyAt` with `"polygon"` closes the shape through at least three points, and `"polyLine"` leaves it open, with endings like a line's.
- **A pressure-sensitive pen:** give each stroke of the ink spec its `widths`, one per point. Lamina PDF fills the stroke as an outline that swells and thins with them. Lamina PDF stores the widths under `/LMNA_Widths`, a key of its own. Other readers show the stored drawing. A tool that draws the mark again from its keys uses the single `width`.
- **Erase a stroke:** `updateAnnot` with an ink patch removes strokes by their place in the list (`remove`) and adds new ones (`add`). Lamina PDF draws the mark again.
- **A PDF/A or PDF/UA file:** the marks keep the file's claims, and `doc.plan()` warns before you save when an edit breaks one. A mark with no description breaks PDF/UA ([`annot-without-contents`](../../errors/annot-without-contents.md)).

Next: [answer a review](../../pages/recipes/review-comments.md), which reads these marks back as comments.
