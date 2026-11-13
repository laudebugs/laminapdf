---
title: "invalid-label-ranges"
version: "0.3.0"
url: "https://docs.laminapdf.com/reference/codes/invalid-label-ranges/"
---

# invalid-label-ranges

- Type: error
- Kind: `usage` (the caller misused the API)

## What happened

A `SetPageLabels` op gave page label ranges that break a rule of the PDF standard, so nothing was changed.

## Why

A document's page labels are a list of ranges, each starting at a page (ISO 32000-2 §12.4.2). The list must start on the first page, so every page has a label. The ranges must follow the page order, with at most one starting on any page. Numbers start from 1, so a range can't number from 0.

## How to fix

Give the ranges in page order, put the first on the document's first page, and number each from 1 or more. To remove every label, give an empty list.

## Example

```rust
let first = doc.page_info(0)?.id;
let third = doc.page_info(2)?.id;
let ranges = vec![
    PageLabelRange::new(third, Some(LabelStyle::Decimal)),
    PageLabelRange::new(first, Some(LabelStyle::LowerRoman)),
];
doc.apply(vec![Op::from(SetPageLabels::new(ranges))]); // invalid-label-ranges: out of page order
```

## Data fields

- `what`: the rule that was broken: `first-page` (the first range isn't on the first page), `order` (ranges out of page order, or two on one page) or `first-number` (a range numbered from 0).
