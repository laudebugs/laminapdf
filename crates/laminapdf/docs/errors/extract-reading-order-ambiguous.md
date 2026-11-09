---
title: "extract-reading-order-ambiguous"
version: "0.2.0-alpha.4"
url: "https://docs.laminapdf.com/reference/codes/extract-reading-order-ambiguous/"
---

# extract-reading-order-ambiguous

- Type: warning
- Kind: `unsupported` (valid input that Lamina PDF doesn't implement)

## What happened

A page mixes horizontal and vertical writing. Extraction put its blocks in an order, but no layout rule tells which comes first, so the order is a guess.

## Why

Japanese and Chinese pages often set the body vertically and captions, headings or page numbers horizontally. Without a structure tree, nothing in the file says how the two relate.

## How to fix

Nothing to do in Lamina PDF; the file is valid. Like every warning, it fails the call in strict mode. Check the order of that page's blocks, or ask for the order the content stream draws (`ReadingOrder::ContentStream`). A tagged file avoids the guess.

## Example

A page whose body text is drawn with a vertical font (`Identity-V`) and whose page number is drawn with a horizontal one.

## Data fields

None.

The `location` has the page's `pageIndex`.
