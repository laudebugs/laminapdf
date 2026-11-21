---
title: "Place images and shapes on a page"
description: "Add a logo, a picture, a shape or a page of another PDF to an existing PDF page as page content, then move, fade, crop or remove it after saving."
version: "0.7.0"
url: "https://docs.laminapdf.com/recipes/place-content/"
---

# Place images and shapes on a page

Add a logo, a picture, a shape or a page of another PDF to an existing PDF page as page content, then move, fade, crop or remove it after saving.

You'll brand a report: a pale band behind its title, the company's logo over the band's right end, and its wordmark at the foot of the page. You need this to letterhead documents, put a logo on generated reports, or let people place pictures and shapes on pages.

Each object becomes part of the page, as if the page had been made with it. Unlike a stamp or a comment, it prints, every reader draws it, and no one can hide it. The code adds a band, a logo and a wordmark to the report's first page:

**TypeScript**

`packages/sdk/examples/place-content.ts`:

```ts
// Brand a report: a pale band behind its title, the company's logo over the band's right end,
// and its wordmark, a page of another PDF kept as vectors and text, at the foot of the page.
// Each is placed as page content, not as a comment: it prints and every reader draws it. The
// boxes are in page space (points from the page's top-left), as a viewer reports where a
// pointer went. In this tagged report the logo and the wordmark are figures with alternate
// text, and the band is decoration, which screen readers skip.
import { Document } from "@laminapdf/sdk";

// Where each goes on the example's input, the quarterly report, in points from the page's
// top-left: for your own file, take them from your viewer's pointer.
/** The band behind the title: `[x0, y0, x1, y1]`. */
const BAND = [44, 68, 551, 110] as const;
/** The logo, over the band's right end, as wide again as it is tall, as the image is. */
const LOGO = [455, 65, 551, 113] as const;
/** The wordmark, at the foot of the page, the size of its page. */
const WORDMARK = [56, 740, 176, 800] as const;

/** The matrix that stretches an image's or a page's unit square to the box `[x0, y0, x1, y1]`. */
function to([x0, y0, x1, y1]: readonly [number, number, number, number]) {
  return [x1 - x0, 0, 0, y1 - y0, x0, y0] as const;
}

/**
 * The document branded: a band, a logo and a wordmark on its first page, saved incrementally.
 * `at` is when it was branded, as a PDF date.
 */
export function placeContent(
  bytes: Uint8Array,
  logo: Uint8Array,
  wordmark: Uint8Array,
  at: string,
): Uint8Array<ArrayBuffer> {
  using doc = Document.open(bytes);

  // Behind the page's own content, so the title stays on top of it.
  doc.content.add(0, {
    id: "title-band",
    object: {
      shape: { path: { rect: { rect: [...BAND], radius: 6 } }, fill: [0.89, 0.93, 0.98] },
    },
    matrix: [1, 0, 0, 1, 0, 0],
    layer: "behind",
    alt: "decorative",
    modified: at,
  });

  doc.content.add(0, {
    id: "logo",
    object: { image: { blob: doc.addBlob(logo) } },
    matrix: to(LOGO),
    alt: { text: "Northwind Traders" },
    modified: at,
  });

  doc.content.add(0, {
    id: "wordmark",
    object: { form: { blob: doc.addBlob(wordmark), page: 0 } },
    matrix: to(WORDMARK),
    alt: { text: "Northwind, the company's wordmark" },
    modified: at,
  });

  return doc.save({ mode: "incremental" });
}
```

**Rust**

`crates/laminapdf/examples/place_content.rs`:

````rust
//! Brand a report: a pale band behind its title, the company's logo over the band's right end,
//! and its wordmark, a page of another PDF kept as vectors and text, at the foot of the page.
//!
//! Each is placed as page content, not as a comment: it prints and every reader draws it. The
//! boxes are in page space (points from the page's top-left), as a viewer reports where a
//! pointer went. In this tagged report the logo and the wordmark are figures with alternate
//! text, and the band is decoration, which screen readers skip.
//!
//! ```sh
//! cargo run -p laminapdf --example place_content -- in.pdf logo.png wordmark.pdf <time> [out-dir]   # the time as a PDF date
//! ```

use std::error::Error;
use std::path::PathBuf;

use laminapdf::{
    Alt, AnnotColor, ContentId, ContentLayer, Document, Op, OpenOptions, PathSpec, PdfDate, Pt, RectPt, SaveMode,
    SaveOptions,
};

fn main() -> Result<(), Box<dyn Error>> {
    let mut args = std::env::args().skip(1);
    let usage = "usage: place_content in.pdf logo.png wordmark.pdf <time> [out-dir]";
    let input = PathBuf::from(args.next().ok_or(usage)?);
    let logo = std::fs::read(args.next().ok_or(usage)?)?;
    let wordmark = std::fs::read(args.next().ok_or(usage)?)?;
    // When the report was branded, which your app takes from its clock.
    let at: PdfDate = args.next().ok_or(usage)?.parse()?;
    let out = PathBuf::from(args.next().unwrap_or_else(|| "target/examples-out".into()));
    std::fs::create_dir_all(&out)?;
    let branded = place_content(std::fs::read(input)?, logo, wordmark, at)?;
    std::fs::write(out.join("place_content.pdf"), branded)?;
    Ok(())
}

// Where each goes on the example's input, the quarterly report, in points from the page's
// top-left: for your own file, take them from your viewer's pointer.
/// The band behind the title: `[x0, y0, x1, y1]`.
const BAND: [f64; 4] = [44.0, 68.0, 551.0, 110.0];
/// The logo, over the band's right end, as wide again as it is tall, as the image is.
const LOGO: [f64; 4] = [455.0, 65.0, 551.0, 113.0];
/// The wordmark, at the foot of the page, the size of its page.
const WORDMARK: [f64; 4] = [56.0, 740.0, 176.0, 800.0];

/// The document branded: a band, a logo and a wordmark on its first page, saved incrementally.
pub(crate) fn place_content(
    bytes: Vec<u8>,
    logo: Vec<u8>,
    wordmark: Vec<u8>,
    at: PdfDate,
) -> Result<Vec<u8>, Box<dyn Error>> {
    let mut doc = Document::open_bytes(bytes, OpenOptions::new())?;
    let id = |name: &str| ContentId::new(name).ok_or("an ID is ASCII letters, digits, '-', '_' or '.'");

    // Behind the page's own content, so the title stays on top of it.
    let band = PathSpec::Rect {
        rect: area(BAND),
        radius: 6.0,
    };
    let pale_blue = AnnotColor::Rgb(0.89, 0.93, 0.98);
    let band = doc
        .shape_on(0, id("title-band")?, band, Some(pale_blue), None, at)?
        .with_layer(ContentLayer::Behind)
        .with_alt(Alt::Decorative);

    let logo = doc
        .image_at(0, id("logo")?, area(LOGO), logo, at)?
        .with_alt(Alt::Text("Northwind Traders".into()));

    let wordmark = doc
        .page_at(0, id("wordmark")?, area(WORDMARK), wordmark, 0, at)?
        .with_alt(Alt::Text("Northwind, the company's wordmark".into()));

    doc.apply(vec![Op::from(band), Op::from(logo), Op::from(wordmark)])?;
    let options = SaveOptions::new().with_mode(SaveMode::Incremental);
    Ok(doc.save(&options)?.bytes)
}

/// A box in page space from its corners.
fn area([x0, y0, x1, y1]: [f64; 4]) -> RectPt {
    RectPt::new(Pt::new(x0), Pt::new(y0), Pt::new(x1), Pt::new(y1))
}
````

**Result:** [place_content.pdf](https://docs.laminapdf.com/examples/pdf/place_content.pdf)

The result is the original bytes, unchanged, followed by one revision. The band sits behind the title, the logo's transparent PNG edges let the band show through at its right end, and the wordmark stays sharp at any zoom because it is still vectors and text.

## How it works

- You name each object with an ID of your own, unique on its page. Lamina PDF stores the ID with the object, so `content.update` and `content.remove` find it again after the file is saved and reopened, and `content.added` lists what a page holds.
- An object's matrix places it on the page, in page space: points from the page's top-left corner as shown, with y going down. A picture or a page of another PDF fills a square one point wide, so the matrix `[w, 0, 0, h, x, y]` draws it `w` by `h` points with its top-left corner at `(x, y)`. A shape's points are already in page space, so the identity matrix `[1, 0, 0, 1, 0, 0]` draws it where they are. Turn or mirror an object by changing the matrix.
- Lamina PDF never rewrites the page's own content. It draws each object with a few drawing instructions of its own, added after the page's content, or before it for the `behind` layer. The page's text and pictures keep their bytes, and an object moves without touching anything else.
- In a tagged file, an object with alternate text becomes a figure that screen readers announce, and one marked `"decorative"` is skipped. A file that claims PDF/UA-1 requires one or the other for a picture, and refuses it otherwise with [`content-needs-alt-text`](../../errors/content-needs-alt-text.md).

## Change, fade and remove objects

- **A shape:** give its path as `{ line }`, `{ rect }` (with an optional corner `radius`), `{ ellipse }` or `{ polygon }`, filled with `fill`, outlined with `stroke`, or both. A stroke's `dash` is `"solid"`, `"dashed"` or `"dotted"`.
- **Move, resize or turn an object later:** `content.update(index, id, { matrix, modified })`. The same call takes `opacity`, an image's `crop` (the part of its square that shows), a shape's `fill` and `stroke`, and new bytes in `blob`. Its `order` is the object's place among those on its layer: `0` sends it to the back and `-1` brings it to the front.
- **Fade it:** give `opacity` from 0 to 1. A file that claims PDF/A-1 forbids see-through objects, so `doc.plan()` warns with [`pdfa-transparency`](../../errors/pdfa-transparency.md); later PDF/A parts allow them.
- **Remove it:** `content.remove(index, id)`. An [incremental save](../../pages/reference/glossary.md#incremental-save) keeps the file's earlier revisions, and the object stays in them; save with `mode: "fullRewrite"` to write the file without it.

## Limits

- **Image formats:** JPEG and PNG work, and a PNG keeps its transparency. Convert other formats first; anything else fails with [`content-object-invalid`](../../errors/content-object-invalid.md).
- **In the browser:** placing objects works in every module, the base module included.

Next: [edit what a page draws](../../pages/recipes/edit-objects.md), to move and change the objects a page already had.
