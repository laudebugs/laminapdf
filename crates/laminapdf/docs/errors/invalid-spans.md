---
title: "invalid-spans"
version: "0.2.0-alpha.5"
url: "https://docs.laminapdf.com/reference/codes/invalid-spans/"
---

# invalid-spans

- Type: error
- Kind: `usage` (the caller passed something invalid)

## What happened

The styled spans passed to `break_spans` don't cover the paragraph's text exactly once, in order, or name fonts it can't use. One of these is true:

- two spans leave a gap, overlap, or come out of order;
- the last span stops before the end of the text, or runs past it;
- a span starts or ends inside a character, or between a letter and its accent;
- a span lists no fonts, or a font index past the end of the list.

## Why

Each character's font, size and language come from the span that holds it. A character in no span, or in two, would have no style or two styles, so Lamina PDF refuses the spans rather than guess. A letter and its accent are drawn as one glyph cluster, so they can't take two styles. An index past the end of the font list names no font.

## How to fix

List the spans in the order of the text. Start the first at byte 0, start each span where the one before it ends, and end the last at the text's length. Take span ends from `grapheme_boundaries` or `str::len`, so that they fall between clusters. List at least one font per span, each an index into the font list. An empty text needs no spans.

## Example

```rust
let text = "Hello, world";
let spans = [StyledSpan::new(0..5), StyledSpan::new(6..12)]; // byte 5 is in no span
break_spans(&shaper, &fonts, text, &spans, &options)?; // invalid-spans
```

## Data fields

- `at`: the byte offset where the spans stop covering the text, or where the span with a bad font list starts.
