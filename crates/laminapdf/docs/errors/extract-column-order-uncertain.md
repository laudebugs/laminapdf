---
title: "extract-column-order-uncertain"
version: "0.3.0"
url: "https://docs.laminapdf.com/reference/codes/extract-column-order-uncertain/"
---

# extract-column-order-uncertain

- Type: warning
- Kind: `unsupported` (something Lamina PDF can't do yet)

## What happened

Extraction read the page's columns in an order its layout rules had to guess, so the page's blocks may be in the wrong order. The blocks the guess concerns have a `confidence` of 0.5 instead of 0.75; the order itself is the rules' best one.

## Why

A PDF stores where each piece of text is drawn, not the order to read it in, so extraction works the order out from the layout: columns first, top to bottom. Three layouts leave that order in doubt, and the warning's `reason` names which one it found:

- `narrow-gutter`: two columns sit closer together than a font size, so lines may have been read straight across both columns.
- `columns-revisited`: a block across the gutter, such as a pull quote with the text wrapped round it, cut the columns into rows, so the order leaves a column and comes back to it.
- `near-grid`: blocks that look like an unruled table, with one cell wrapped onto a second line, were read column by column; read by rows, the order would differ.

A page that mixes horizontal and vertical writing raises [`extract-reading-order-ambiguous`](extract-reading-order-ambiguous.md) instead.

## How to fix

Check the page's text, or send the page to a layout model, which reads columns from the page's image. A tagged file read in its structure tree's order never raises this warning. Like every warning, it fails the call in strict mode.

## Example

A newsletter page with two columns set 7 pt apart: each line of the extracted text runs across both columns, and the page is warned with `narrow-gutter`.

## Data fields

- `reason`: `narrow-gutter`, `columns-revisited` or `near-grid`, the first check that doubted the order.

The `location` has the page's `pageIndex`.
