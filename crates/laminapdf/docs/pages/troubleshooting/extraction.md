---
title: "Troubleshooting text extraction"
description: "Fixes for extracted text that comes out empty, in the wrong order, with missing characters, or without its headers and footers."
version: "0.2.0"
url: "https://docs.laminapdf.com/troubleshooting/extraction/"
---

# Troubleshooting text extraction

Fixes for extracted text that comes out empty, in the wrong order, with missing characters, or without its headers and footers.

These entries cover text extraction, in TypeScript, Rust and `laminapdf extract`. A PDF stores where each piece of text is drawn, not the order to read it in, so most surprises come from the file rather than from your code. The options these fixes set are in TypeScript and Rust; `laminapdf extract` has no flags for them yet. Each entry starts from what you see, then gives the cause and the fix.

## A page's text is empty, with `extract-needs-ocr`

**Cause:** the page draws no text that extraction can use. A scanned page is one image, and outlined text is drawn as shapes. On the command line, the warning makes `laminapdf extract` exit 3.

**Fix:** send the pages the warning names to OCR (optical character recognition) or a vision model. Each page in the JSON has a `textLayer` whose `status` is `needs-ocr` for these pages. See [Extract text for an LLM](../../pages/recipes/extract-markdown.md) and [`extract-needs-ocr`](../../errors/extract-needs-ocr.md).

## A searchable scan gives no text

**Cause:** OCR made the scan searchable by putting invisible text under the image. Extraction leaves text a reader can't see out of the blocks, so the page's `textLayer` status is `ocr-layer` and the warning is `extract-needs-ocr`.

**Fix:** to keep the existing OCR text, set `drop: { keepSpans: true }` (`DropOptions::keep_spans` in Rust). The text is then listed under `dropped`, marked `invisible`. Or run OCR again, if you don't trust the old layer. See [`extract-needs-ocr`](../../errors/extract-needs-ocr.md).

## The text runs across two columns, with `extract-column-order-uncertain`

**Cause:** extraction works the reading order out from the layout, and this page left it in doubt. For example, two columns set closer than a font size apart read as one. The blocks in doubt have a `confidence` of 0.5.

**Fix:** check those pages by hand, or send them to a layout model, which reads columns from the page's image. If the file is tagged, its structure tree gives the order, and a page read in the tree's order never raises this warning. See [`extract-column-order-uncertain`](../../errors/extract-column-order-uncertain.md).

## A tagged file reads in layout order, with `extract-struct-tree-ignored`

**Cause:** the file's structure tree, which gives its reading order, failed extraction's checks. A tree that covers less than 80% of the page's visible text, or that jumps back across columns, isn't trusted, and extraction reads the page by its layout instead.

**Fix:** nothing, in most cases: the layout order is the best one extraction can find. To use the tree anyway, set `readingOrder: "tagged"` (`ReadingOrder::Tagged` in Rust). `"layout"` and `"content-stream"`, the order the page draws its text, are the other choices. See [`extract-struct-tree-ignored`](../../errors/extract-struct-tree-ignored.md).

## Characters come out as `�`, with `unmapped-glyph`

**Cause:** the font has no map from its glyphs to characters, so extraction can't tell which letter each glyph draws. Most of these are subset fonts without a [ToUnicode](../../pages/reference/glossary.md#tounicode) map: the table in a font that says which character each glyph draws. Extraction writes U+FFFD, the replacement character, for each glyph.

**Fix:** only OCR can recover the text, because the file doesn't hold it. A page with many such glyphs is also `needs-ocr`. See [`unmapped-glyph`](../../errors/unmapped-glyph.md).

## Headers, footers and page numbers are missing

**Cause:** extraction drops page furniture by default: text the file marks as an artifact, and headers, footers and page numbers that repeat across pages. Each one is listed under `dropped` instead, so a language model doesn't read the same header on every page.

**Fix:** to keep them in the text, set `drop: { furniture: false }` (`DropOptions::furniture` in Rust).

Next: [rendering pages](../../pages/troubleshooting/rendering.md), for PDFium, large images and pages that draw wrong.
