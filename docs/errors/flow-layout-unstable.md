# `flow-layout-unstable`

- Type: error
- Kind: `usage` (the caller misused the API)

## What happened

`Builder::flow` laid out a flow with a contents page four times, and the page numbers on the contents page still changed. The flow stopped and added no pages.

## Why

A contents page lists each heading with the page it starts on, but the contents page takes up room itself. The flow lays everything out, writes the page numbers into the contents page, and lays out again. Usually the numbers settle on the second pass. They keep changing when the contents page sits on the same pages as the headings it lists, so that each new set of numbers makes the contents page longer or shorter and moves the headings again.

## How to fix

Put a page break after the contents block, so the contents page has pages of its own and the headings after it don't move when its numbers change. Listing fewer heading levels with `Contents::with_levels` also makes the contents page shorter.

## Example

```rust
let flow = Flow::new(template)
    .block(FlowBlock::Contents(Contents::new(&style)))
    .block(FlowBlock::PageBreak) // the contents page keeps its own pages
    .block(FlowBlock::Heading(1, Paragraph::plain("Introduction", &heading)));
builder.flow(&flow)?;
```

## Data fields

None.
