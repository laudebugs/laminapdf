---
title: "flow-keep-broken"
version: "0.2.0-alpha.4"
url: "https://docs.laminapdf.com/reference/codes/flow-keep-broken/"
---

# flow-keep-broken

- Type: warning
- Kind: `usage` (the caller misused the API)

## What happened

`Builder::flow` met blocks it was asked to keep on one page, and together they are taller than a page. The flow split them across pages anyway. Blocks are kept together by a `FlowBlock::Keep` group, or by a heading or a paragraph set to keep with the block after it.

## Why

Blocks that can't fit on one page can't be kept on one page. No layout can keep them together, so the flow doesn't look for one. It lays them out as it would without the rule, and every page still ends on a whole line.

## How to fix

Keep fewer blocks together: split a long `Keep` group into smaller ones, or turn off `keep_with_next` on a paragraph in a long chain. Or make the page taller, or its margins smaller, so the group fits.

## Example

```rust
let group = FlowBlock::Keep(vec![long_intro, long_paragraph]); // taller than a page
let report = builder.flow(&Flow::new(template).block(group))?;
assert_eq!(report.warnings[0].code(), "flow-keep-broken");
```

## Data fields

- `page`: the page the blocks start on, counted from 0 within the flow.
