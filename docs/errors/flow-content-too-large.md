# `flow-content-too-large`

- Type: error
- Kind: `usage` (the caller misused the API)

## What happened

`Builder::flow` found a figure, or a single line of text, taller than the body of a page: the space between the top and bottom margins. No page can hold it, so the flow stopped and added no pages.

## Why

A flow moves a figure to the next page when it doesn't fit, and splits a paragraph only between lines. A figure or a line taller than a whole page would fit on no page at all, and cutting it would hide part of it.

## How to fix

Make the figure narrower with `Figure::with_width`, so its height shrinks with it. For a line, use a smaller text size or a smaller line height. Or give the page more room with a taller page size or smaller top and bottom margins.

## Example

```rust
let tall = Image::rgba(1, 10, &[0; 40])?; // ten times as tall as it is wide
let flow = Flow::new(PageTemplate::new(PageSize::A6)).block(FlowBlock::Figure(Figure::new(tall, "A bar")));
builder.flow(&flow)?; // flow-content-too-large
```

## Data fields

- `what`: what is too tall, `line` or `figure`.
