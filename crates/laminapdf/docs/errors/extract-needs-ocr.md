---
title: "extract-needs-ocr"
version: "0.2.0-alpha.5"
url: "https://docs.laminapdf.com/reference/codes/extract-needs-ocr/"
---

# extract-needs-ocr

- Type: warning
- Kind: `unsupported` (something Lamina PDF can't do yet)

## What happened

The page has no text that extraction can use, so the page's Markdown, text and blocks are empty or close to it. Each page's `textLayer` in the extraction says why, and this warning names the pages whose status is `needs-ocr` or `ocr-layer`.

## Why

A page's text comes from its fonts. Some pages draw their words in other ways:

- a scanned page is one image, with no text at all;
- a page made searchable by OCR has an image under invisible text, which extraction leaves out of the blocks because a reader can't see it;
- outlined text is drawn as shapes, not glyphs;
- a font without a Unicode mapping draws glyphs that extraction can't turn into characters.

## How to fix

Send the pages this warning names to OCR or a vision model. On an `ocr-layer` page you can instead keep the existing OCR layer: extraction lists its text under `dropped` as `invisible` when you set `drop: { keepSpans: true }` (`DropOptions::keep_spans` in Rust). Like every warning, it fails the call in strict mode.

## Example

A scanned contract: every page is `needs-ocr`, and the extraction's Markdown is empty.

## Data fields

- `status`: the page's text-layer status, `needs-ocr` or `ocr-layer`.

The `location` has the page's `pageIndex`.
