---
title: "invalid-box"
version: "0.2.0-alpha.4"
url: "https://docs.laminapdf.com/reference/codes/invalid-box/"
---

# invalid-box

- Type: warning
- Kind: `input` (the input is malformed beyond what tolerance repairs)

## What happened

A page boundary box is not four numbers; it reads as absent.

## Why

A writer bug: a box that is not an array of four numbers (too few, or names or strings in it).

## How to fix

Nothing; the page uses the next box in the chain.

## Example

```text
3 0 obj << /Type /Page /MediaBox [0 0 612 792] /CropBox [0 0 612] >>
```

The CropBox has three numbers, so it reads as absent and the page uses its MediaBox. `data` is `{ "key": "CropBox" }`.

## Data fields

`key` (which box, from Lamina PDF's fixed list).
