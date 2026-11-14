---
title: "Multilingual text"
description: "Latin, Arabic, Devanagari, Hebrew and Chinese on one page, each in its own embedded font, as a shaped line and a wrapped paragraph."
version: "0.3.0"
url: "https://docs.laminapdf.com/recipes/multilingual/"
---

# Multilingual text

Latin, Arabic, Devanagari, Hebrew and Chinese on one page, each in its own embedded font, as a shaped line and a wrapped paragraph.

You'll put five scripts on one page: Latin, Arabic, Devanagari, Hebrew and Chinese, each as a single line and as a paragraph wrapped in a box. Start from it whenever your text goes beyond Latin, or mixes left-to-right and right-to-left writing.

The function takes one font file per script, draws each script's line and paragraph, and returns the PDF's bytes. In TypeScript it needs the text module, `init({ text: true })`. Both languages write the same bytes:

**TypeScript**

`packages/sdk/examples/multilingual.ts`:

```ts
// Five scripts on one page, each as a single line and as a wrapped paragraph. The scripts are
// Latin, Arabic and Hebrew (right to left), Devanagari and simplified Chinese; each is shaped and
// bidi-resolved, and each paragraph wraps in a text box. Each script needs a font that covers it,
// such as the Noto fonts (OFL).
import { Builder, type BuilderOptions, type Direction, Font } from "@laminapdf/sdk";

/** Each script's line, its paragraph and the paragraph's direction, in the order of the fonts. */
export const SCRIPTS: readonly (readonly [line: string, paragraph: string, direction: Direction])[] = [
  [
    "Latin: the office staff fill the affine waffle flask",
    "A paragraph breaks at spaces and fits the width of its box, line by line.",
    "ltr",
  ],
  ["مرحبا بالعالم السلام عليكم", "مرحبا بالعالم السلام عليكم مرحبا بالعالم السلام عليكم", "rtl"],
  ["नमस्ते दुनिया। हिन्दी पाठ का परीक्षण", "नमस्ते दुनिया। हिन्दी पाठ का परीक्षण नमस्ते दुनिया।", "ltr"],
  ["שלום עולם וברוכים הבאים", "שלום עולם וברוכים הבאים שלום עולם וברוכים הבאים", "rtl"],
  ["你好世界，这是中文字体子集测试。", "你好世界，这是中文字体子集测试。你好世界，这是中文。", "ltr"],
];
const MARGIN = 56;
const FIRST_BASELINE = 80;
/** The distance from one script's line to the next one's. */
const SCRIPT_HEIGHT = 140;
/** The paragraph's box: this far below the line, this wide and this tall. */
const PARAGRAPH_DROP = 14;
const PARAGRAPH_WIDTH = 220;
const PARAGRAPH_HEIGHT = 100;

/**
 * The document's bytes. Needs `init({ text: true })`.
 *
 * @param fonts - one font file per script, in the order of {@link SCRIPTS}.
 * @param options - can fix the file ID, the producer and the creation date.
 */
export function multilingual(fonts: readonly Uint8Array[], options: BuilderOptions = {}): Uint8Array<ArrayBuffer> {
  if (fonts.length !== SCRIPTS.length) throw new Error(`${SCRIPTS.length} fonts, one per script, are needed`);
  using b = new Builder(options);
  const loaded = fonts.map((bytes) => Font.fromBytes(bytes));
  for (const font of loaded) b.font(font);
  const page = b.page("A4");
  const s = page.surface();
  SCRIPTS.forEach(([line, paragraph, direction], row) => {
    const font = loaded[row];
    if (font === undefined) return;
    const y = FIRST_BASELINE + SCRIPT_HEIGHT * row;
    s.text(line, { font, size: 16, at: [MARGIN, y] });
    // A narrow box breaks the paragraph into lines; right-to-left paragraphs start at the right.
    const box = [MARGIN, y + PARAGRAPH_DROP, PARAGRAPH_WIDTH, PARAGRAPH_HEIGHT] as const;
    s.textBox(paragraph, { font, size: 12, box, direction, align: "start" });
  });
  page.finish();
  using doc = b.finish();
  return doc.save();
}
```

**Rust**

`crates/laminapdf/examples/multilingual.rs`:

````rust
//! Five scripts on one page, each as a single line and as a wrapped paragraph.
//!
//! The scripts are Latin, Arabic and Hebrew (right to left), Devanagari and simplified Chinese;
//! each is shaped and bidi-resolved, and each paragraph wraps in a text box. Each script needs a
//! font that covers it, such as the Noto fonts (OFL).
//!
//! ```sh
//! cargo run -p laminapdf --example multilingual -- latin.ttf arabic.ttf devanagari.ttf hebrew.ttf chinese.otf
//! ```

use std::error::Error;
use std::path::PathBuf;

use laminapdf::{
    Alignment, BuildOptions, Builder, Direction, PageSize, PointPt, Pt, RectPt, SaveOptions, TextBoxOptions,
};

/// Each script's line, its paragraph and the paragraph's direction, in the order of the fonts.
const SCRIPTS: [(&str, &str, Direction); 5] = [
    (
        "Latin: the office staff fill the affine waffle flask",
        "A paragraph breaks at spaces and fits the width of its box, line by line.",
        Direction::LeftToRight,
    ),
    (
        "مرحبا بالعالم السلام عليكم",
        "مرحبا بالعالم السلام عليكم مرحبا بالعالم السلام عليكم",
        Direction::RightToLeft,
    ),
    (
        "नमस्ते दुनिया। हिन्दी पाठ का परीक्षण",
        "नमस्ते दुनिया। हिन्दी पाठ का परीक्षण नमस्ते दुनिया।",
        Direction::LeftToRight,
    ),
    (
        "שלום עולם וברוכים הבאים",
        "שלום עולם וברוכים הבאים שלום עולם וברוכים הבאים",
        Direction::RightToLeft,
    ),
    (
        "你好世界，这是中文字体子集测试。",
        "你好世界，这是中文字体子集测试。你好世界，这是中文。",
        Direction::LeftToRight,
    ),
];
const MARGIN: f64 = 56.0;
const FIRST_BASELINE: f64 = 80.0;
/// The distance from one script's line to the next one's.
const SCRIPT_HEIGHT: f64 = 140.0;
/// The paragraph's box: this far below the line, this wide and this tall.
const PARAGRAPH_DROP: f64 = 14.0;
const PARAGRAPH_WIDTH: f64 = 220.0;
const PARAGRAPH_HEIGHT: f64 = 100.0;

fn main() -> Result<(), Box<dyn Error>> {
    let mut args: Vec<String> = std::env::args().skip(1).collect();
    let out = PathBuf::from(if args.len() > SCRIPTS.len() {
        args.remove(SCRIPTS.len())
    } else {
        "target/examples-out".into()
    });
    let fonts = args.iter().map(std::fs::read).collect::<Result<Vec<_>, _>>()?;
    std::fs::create_dir_all(&out)?;
    std::fs::write(out.join("multilingual.pdf"), build(BuildOptions::new(), fonts)?)?;
    Ok(())
}

/// The document's bytes, with one font per script in `fonts` (in the order of [`SCRIPTS`]).
/// `options` can fix the file ID, the producer and the creation date.
pub(crate) fn build(options: BuildOptions, fonts: Vec<Vec<u8>>) -> Result<Vec<u8>, Box<dyn Error>> {
    if fonts.len() != SCRIPTS.len() {
        return Err(format!("{} fonts, one per script, are needed", SCRIPTS.len()).into());
    }
    let mut b = Builder::new(options);
    let fonts = fonts
        .into_iter()
        .map(|bytes| b.font_bytes(bytes))
        .collect::<Result<Vec<_>, _>>()?;
    let mut page = b.page(PageSize::A4);
    let s = page.surface();
    for (row, ((line, paragraph, direction), font)) in (0_u32..).zip(SCRIPTS.iter().zip(&fonts)) {
        let y = FIRST_BASELINE + SCRIPT_HEIGHT * f64::from(row);
        s.text(line, font, Pt::new(16.0), PointPt::from_points(MARGIN, y))?;
        // A narrow box breaks the paragraph into lines; right-to-left paragraphs start at the right.
        let options = TextBoxOptions::new()
            .with_direction(*direction)
            .with_align(Alignment::Start);
        let area = RectPt::from_points(MARGIN, y + PARAGRAPH_DROP, PARAGRAPH_WIDTH, PARAGRAPH_HEIGHT);
        s.text_box(paragraph, font, Pt::new(12.0), area, &options)?;
    }
    page.finish()?;
    let mut doc = b.finish()?;
    Ok(doc.save(&SaveOptions::new())?.bytes)
}
````

**Result:** [multilingual.pdf](https://docs.laminapdf.com/examples/pdf/multilingual.pdf)

The result is one A4 page, with each script set in its own Noto font. The Result tab shows it, with a link to the PDF.

## How it works

- `Font.fromBytes` (`font_bytes` in Rust) loads each font, which must cover its script. The builder embeds only the glyphs the page draws.
- Lamina PDF shapes the text with each font, so Arabic letters join, Devanagari letters combine into clusters and the Latin line forms ligatures.
- `textBox` breaks each paragraph into lines that fit its 220-point-wide box. It breaks where Unicode's line-breaking rules allow: between words, and between Chinese characters.
- `direction: "rtl"` (`Direction::RightToLeft`) makes the Arabic and Hebrew paragraphs right to left, so `align: "start"` starts their lines at the box's right edge.
- Each font is embedded with its [ToUnicode](../../pages/reference/glossary.md#tounicode) mapping, so text copied or extracted from the PDF equals the strings you passed in, ligatures included.

## What to change

- **Your own scripts:** change `SCRIPTS` and pass one font per row, in the same order. A character the font lacks fails with `missing-glyph`.
- **The paragraph's box:** `PARAGRAPH_WIDTH` and `PARAGRAPH_HEIGHT` set its size. Lines that don't fit are left out, unless you pass `overflow: "error"` (`Overflow::Error`) to fail with `text-overflow` instead.
- **Alignment and spacing:** `align` also takes `"end"`, `"left"`, `"right"` or `"center"`, and `lineHeight` (`with_line_height`) sets the distance between baselines, 1.2 times the size by default.
- **Ligatures and other OpenType features:** `features: ["liga=0"]` on `text` or `textBox` turns the ligatures off. In Rust, call `features` on the run or `with_feature` on the text box's options. See [Text and fonts](../../pages/guides/fonts-and-text.md).

Next: [images and vectors](../../pages/recipes/images-and-vectors.md), with shapes, colour, transparency and two images.
