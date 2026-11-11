---
title: "extract-table-limit"
version: "0.2.0-alpha.6"
url: "https://docs.laminapdf.com/reference/codes/extract-table-limit/"
---

# extract-table-limit

- Type: warning
- Kind: `limit` (a `Limits` value or another resource bound was hit)

## What happened

A page's ruling lines would have made a table with more cells than `ExtractLimits::max_table_cells` allows, or the page drew so many lines that matching them all would cost too much. Extraction made no table from those lines; their text is still in the page's paragraphs.

## Why

A page drawn with thousands of lines, such as a large grid, a chart's gridlines or a file built to slow extraction down. To find a table, extraction compares every horizontal line with every vertical one, so the work grows with the square of the line count.

## How to fix

Raise `max_table_cells` if the page really holds a table that big.

## Example

```rust
let mut options = laminapdf_extract::ExtractOptions::default();
options.limits.max_table_cells = 50_000;
```

## Data fields

- `limit`: the limit that was reached.

The `location` has the page's `pageIndex`.
