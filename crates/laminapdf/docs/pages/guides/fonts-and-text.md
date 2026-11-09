---
title: "Text and fonts"
description: "Draw text in any script with your own fonts or the standard 14, one line at a time or as a paragraph that wraps in a box."
version: "0.2.0-alpha.4"
url: "https://docs.laminapdf.com/guides/fonts-and-text/"
---

# Text and fonts

Draw text in any script with your own fonts or the standard 14, one line at a time or as a paragraph that wraps in a box.

You'll draw text on a page: single lines, and paragraphs that wrap to fit a box, in any script and direction. You need this whenever a PDF holds words, from a one-line label to a report in five languages.

Text needs the text module: `init({ text: true })` or `loadText()` in TypeScript, and the default `text` feature in Rust.

## Pick a font

Lamina PDF draws with two kinds of font:

- **The [standard 14 fonts](../../pages/reference/glossary.md#standard-14-fonts)** (`Font.standard("Helvetica")`, `Standard14::Helvetica` in Rust) need no font file and aren't embedded, because every PDF reader has them. They cover Latin text only, without shaping.
- **Your own fonts** (`Font.fromBytes(bytes)`: TrueType, OpenType or a font collection) handle any script. Lamina PDF shapes their text, so letters join and ligatures form, and embeds only the glyphs you draw.

A font is embedded once per document, however many `Font` objects name it.

## Draw a line or a paragraph

`surface.text(str, { font, size, at })` draws one line, with `at[1]` its baseline. `surface.textBox(str, { font, size, box, direction, align })` wraps a paragraph to fit its box and lays out each line's words in display order, so right-to-left text reads correctly. When the text doesn't fit, it reports `overflow`.

This example draws five scripts (Latin with ligatures, Arabic, Devanagari, Hebrew and Chinese), each in its own embedded font:

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

[See the output](../../pages/recipes/multilingual.md). Text extracted from it, with pdf.js, pypdf or pdftotext, equals the input strings, ligatures included.

## When a font can't draw the text

- **A missing character:** text a font can't encode fails with `missing-glyph`, never with a blank box.
- **Too many letters for a standard 14 font:** a standard 14 font holds at most 37 distinct letters beyond its Western European set (WinAnsi); one more fails with `encoding-full`. Use your own font instead.
- **A font that may not be embedded:** a font whose licence bits forbid embedding (restricted, or bitmap only) fails with `font-not-embeddable`. In Rust, `EmbedPolicy::Override` embeds it anyway, if you hold the rights. A font marked "no subsetting" is embedded whole.

> **Details:** Lamina PDF shapes your own fonts' text with HarfRust, a Rust port of the HarfBuzz shaper, and subsets each font to the glyphs you draw. It embeds them as [Type 0](../../pages/reference/glossary.md#type-0) fonts with `/W` widths and a [ToUnicode](../../pages/reference/glossary.md#tounicode) [CMap](../../pages/reference/glossary.md#cmap), which is what lets the text be extracted again. Line breaking follows [UAX #14](https://www.unicode.org/reports/tr14/), and the order of mixed-direction text follows [UAX #9](https://www.unicode.org/reports/tr9/) (bidi). The licence bits are the font's `fsType`.

Next: [shapes, colour and images](../../pages/guides/images-and-vectors.md).
