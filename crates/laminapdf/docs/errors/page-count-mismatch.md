---
title: "page-count-mismatch"
version: "0.2.0-alpha.5"
url: "https://docs.laminapdf.com/reference/codes/page-count-mismatch/"
---

# page-count-mismatch

- Type: warning
- Kind: `input` (the input is malformed beyond what tolerance repairs)

## What happened

The page tree's `/Count` differs from the number of pages walked; the walk wins.

## Why

Writers that don't update `/Count`.

## How to fix

Nothing; Lamina PDF uses the walked count.

## Example

```text
2 0 obj << /Type /Pages /Kids [3 0 R 4 0 R] /Count 7 >>
```

The root says 7 pages but two are reachable: `page_count()` is 2, and the warning's `data` is `{ "declared": 7, "walked": 2 }`.

## Data fields

`declared`, `walked`.
