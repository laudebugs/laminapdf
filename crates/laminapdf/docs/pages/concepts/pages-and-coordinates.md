---
title: "Pages and coordinates"
description: "Points from the top-left corner of the page as it's displayed, y going down, and pages numbered from 0."
version: "0.2.0-alpha.5"
url: "https://docs.laminapdf.com/concepts/pages-and-coordinates/"
---

# Pages and coordinates

Points from the top-left corner of the page as it's displayed, y going down, and pages numbered from 0.

Every position you pass Lamina PDF is in [page space](../../pages/reference/glossary.md#page-space): points measured from the top-left corner of the page, with y going down. It's the way a screen or a sheet of paper reads, so a number means the same thing in every part of the API.

## Points

A point is 1/72 of an inch. An A4 page is about 595 × 842 points, and a US Letter page is 612 × 792. Some pages set a [user unit](../../pages/reference/glossary.md#user-unit), which scales the file's own units; Lamina PDF applies it, so a point is always 1/72 inch.

## The top-left corner of the page you see

The origin is the top-left corner of the page as a viewer displays it: its visible area (its [crop box](../../pages/reference/glossary.md#crop-box)), turned the way the page is rotated. So `[72, 96]` is one inch from the left edge and 96 points from the top, whichever way the page is rotated in the file.

For text, the second number is the baseline, the line the letters sit on. Letters reach up from it, and descenders such as "g" and "p" hang below it.

This app places "Hello, world!" one inch from the top-left corner, with its baseline one text size lower:

**TypeScript**

`packages/sdk/examples/quick-start/node.ts`:

```ts
// Builds a one-page PDF that says "Hello, world!" and writes it to hello.pdf, in Node.
import { writeFile } from "node:fs/promises";
import { Builder, type BuilderOptions, Font, init } from "@laminapdf/sdk";

/** The space between the page's edge and the text, in points (72 points = 1 inch). */
const MARGIN = 72;
/** The text's size in points; its baseline sits this far below the margin. */
const FONT_SIZE = 24;

/** Builds the page and hands its bytes to `save`, which writes hello.pdf unless you pass another `save` function. */
export async function hello(
  save: (pdf: Uint8Array) => void | Promise<void> = (pdf) => writeFile("hello.pdf", pdf),
  options: BuilderOptions = {},
): Promise<void> {
  await init({ text: true });
  using builder = new Builder(options);
  const page = builder.page("A4");
  // Points from the page's top-left corner, y down; the text's baseline sits FONT_SIZE points below the margin.
  page.surface().text("Hello, world!", {
    font: Font.standard("Helvetica"),
    size: FONT_SIZE,
    at: [MARGIN, MARGIN + FONT_SIZE],
  });
  page.finish();
  using doc = builder.finish();
  await save(doc.save());
}

// `node hello.ts` runs it (Node 24.2 or newer); imported, it only exports `hello`.
if (import.meta.main) await hello();
```

**Result:** [hello.pdf](https://docs.laminapdf.com/examples/pdf/hello.pdf)

## Pages count from 0

The first page is page 0 everywhere: `doc.page(0)`, a link's target, a page label's start.

## PDF's own coordinates

Inside the file, PDF measures from the bottom-left corner with y going up, before rotation. That's [user space](../../pages/reference/glossary.md#user-space). Lamina PDF converts for you, so you only meet it in Rust, in names that contain `UserSpace` or `user_space`.

*Page space starts at the top-left corner you see; PDF's user space starts at the bottom-left, before rotation.*

On an upright US Letter page, 612 by 792 points, page space starts at the top-left corner with y going down, and PDF's user space starts at the bottom-left corner with y going up. The point `[72, 96]` is one inch from the left edge and 96 points from the top, on the baseline of a heading whose box runs from `[72, 72]` to `[272, 100]`. With `/Rotate 90` a viewer shows the same page on its side, 792 by 612 points. Page space still starts at the top-left corner you see, so `[72, 96]` lands in the same place on the screen. User space starts at that corner too, with its x axis going down and its y axis going right. The heading in the file now runs down the right edge, from `[692, 72]` to `[720, 272]` in page space.
