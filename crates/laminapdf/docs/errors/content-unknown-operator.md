---
title: "content-unknown-operator"
version: "0.4.0"
url: "https://docs.laminapdf.com/reference/codes/content-unknown-operator/"
---

# content-unknown-operator

- Type: warning
- Kind: `input` (the input is malformed beyond what tolerance repairs)

## What happened

A content stream used an operator ISO 32000-2 Annex A does not define, outside a `BX`/`EX` compatibility section; it was skipped.

## Why

A producer wrote a private or misspelled operator.

## How to fix

Nothing to do; check the page's rendering if the operator mattered.

## Example

```text
BT /F1 12 Tf 72 720 Td (Hi) Tj ET
0 0 1 setcolor
```

`setcolor` is not an Annex A operator and is not inside `BX`/`EX`, so it is skipped with its operands. `data.offset` is its byte offset in the decoded content.

## Data fields

- `offset`: the operator's byte offset in the decoded content.

The `location` is `object` (the page for page content, the Form XObject for a form) with `streamOffset` in its decoded content; none for content interpreted on its own or a direct page.
