# `flow-keep-broken`

- Type: warning
- Kind: `usage` (the caller misused the API)

## What happened

`Builder::flow` met blocks it was asked to keep on one page, and together they are taller than a page. The flow split them across pages anyway. Blocks are kept together by a `FlowBlock::Keep` group, or by a heading or a paragraph set to keep with the block after it.

## Why

Blocks that can't fit on one page can't be kept on one page. The flow lays them out as it would without the rule, so every page still ends with a whole line, rather than looking further for a layout that can't exist.

## How to fix

Keep fewer blocks together: split a long `Keep` group into smaller ones, or turn off `keep_with_next` on a paragraph in a long chain. Or make the page taller, or its margins smaller, so the group fits.

## Example

```rust
let group = FlowBlock::Keep(vec![long_table_intro, long_paragraph]); // taller than a page
let report = builder.flow(&Flow::new(template).block(group))?;
assert_eq!(report.warnings[0].code(), "flow-keep-broken");
```

## Data fields

- `page`: the page the blocks start on, counted from 0 within the flow.
