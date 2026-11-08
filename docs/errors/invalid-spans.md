# `invalid-spans`

- Type: error
- Kind: `usage` (the caller passed something invalid)

## What happened

`break_spans` was given styled spans that don't cover the paragraph's text exactly once, in order. Two spans leave a gap between them, overlap, or come in the wrong order. Or the last span stops before the end of the text or runs past it, or a span starts or ends inside a character.

## Why

Each character's font, size and language come from the span that holds it. A character in no span, or in two, would have no style or two styles, so Lamina PDF refuses the spans rather than guess.

## How to fix

List the spans in the order of the text. Start the first at byte 0, start each span where the one before it ends, and end the last at the text's length. Take span ends from `char_indices` or `str::len`, so that they fall between characters. An empty text needs no spans.

## Example

```rust
let text = "Hello, world";
let spans = [StyledSpan::new(0..5), StyledSpan::new(6..12)]; // byte 5 is in no span
break_spans(&shaper, &fonts, text, &spans, &options)?; // invalid-spans
```

## Data fields

- `at`: the byte offset where the spans stop covering the text.
