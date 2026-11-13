---
title: "content-unbalanced-bt"
version: "0.3.0"
url: "https://docs.laminapdf.com/reference/codes/content-unbalanced-bt/"
---

# content-unbalanced-bt

- Type: warning
- Kind: `input` (the input is malformed beyond what tolerance repairs)

## What happened

A `BT` inside a text object, an `ET` outside one, a text operator outside one, or a content stream that ended inside one.

## Why

A producer bug, or damaged content.

## How to fix

Nothing to do; text operators were handled tolerantly.

## Example

```text
BT /F1 12 Tf (one) Tj BT (two) Tj ET
```

The second `BT` opens a text object inside one; Lamina PDF treats it as a new text object and carries on.

## Data fields

- `offset`: the operator's byte offset, or the end of the content.

The `location` is `object` (the page for page content, the Form XObject for a form) with `streamOffset` in its decoded content; none for content interpreted on its own or a direct page.
