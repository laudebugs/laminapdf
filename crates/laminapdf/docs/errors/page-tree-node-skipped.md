---
title: "page-tree-node-skipped"
version: "0.3.0"
url: "https://docs.laminapdf.com/reference/codes/page-tree-node-skipped/"
---

# page-tree-node-skipped

- Type: warning
- Kind: `input` (the input is malformed beyond what tolerance repairs)

## What happened

A page-tree node was missing, unparsable, of the wrong type or too deep, and was skipped.

## Why

Damaged page trees: a kid that is missing, unparsable or of the wrong type, a `/Kids` that is not an array, or a tree deeper than `max_page_tree_depth`.

## How to fix

Check the page count; pages under the skipped node are missing.

## Example

```text
2 0 obj << /Type /Pages /Kids [3 0 R 9 0 R] /Count 2 >>
9 0 obj (not a page) endobj
```

Kid 9 is a string, not a page or a pages node: it is skipped with this warning, so the document has one page. The `location` is the skipped node's `object` when it has one.

## Data fields

None.
