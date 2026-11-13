---
title: "flow-layout-unstable"
version: "0.3.0"
url: "https://docs.laminapdf.com/reference/codes/flow-layout-unstable/"
---

# flow-layout-unstable

- Type: error
- Kind: `internal` (a bug in Lamina PDF or a broken invariant)

## What happened

`Builder::flow` laid out a flow with a contents page four times, and the page numbers on the contents page still changed. The flow stopped and added no pages.

## Why

A contents page lists each heading with the page it starts on, but those pages aren't known until the flow is laid out. So the flow lays everything out, writes the numbers it found into the contents page, and lays out again to check them. Each entry keeps the same room for its number however wide the number is, so the contents page's length never depends on the numbers, and the second layout finds the same pages.

The flow still checks up to four layouts. This error means the numbers kept changing anyway, which points to a fault in Lamina PDF's layout rather than in your document.

## How to fix

Report the flow that fails, with the blocks and page template that produce it. Until it is fixed, a page break after the contents block may avoid the error, since the headings after it then start on pages of their own.

## Example

```rust
let flow = Flow::new(template)
    .block(FlowBlock::Contents(Contents::new(&style)))
    .block(FlowBlock::PageBreak) // the headings start on their own pages
    .block(FlowBlock::Heading(1, Paragraph::plain("Introduction", &heading)));
builder.flow(&flow)?;
```

## Data fields

None.
