---
title: "encoding-full"
version: "0.2.0-alpha.5"
url: "https://docs.laminapdf.com/reference/codes/encoding-full/"
---

# encoding-full

- Type: error
- Kind: `limit` (a `Limits` value or another resource bound was hit)

## What happened

A standard 14 font had no free one-byte code left for another glyph in this document.

## Why

A standard 14 font is a simple font: text is written one byte per character, so one font resource addresses at most 256 glyphs (ISO 32000-2 §9.6.5). Lamina PDF writes the Latin fonts in `WinAnsiEncoding`, and gives each glyph of the standard Latin character set outside it (`Lslash`, `Scedilla`, `minus`, …) one of the 37 codes `WinAnsiEncoding` leaves undefined, through `/Differences`.

A document that draws more such glyphs with one font than there are free codes runs out. Assignment is all or nothing: when one `encode` call needs more new codes than are free, none are assigned and nothing of the text is recorded.

## How to fix

Embed a font (`Font::from_bytes`), which addresses every glyph through a Type 0 font, or split the text across two registrations of the standard font (each is its own font resource with its own codes).

## Example

Drawing more than 37 different non-WinAnsi Latin glyphs (Ā, ă, Ą, …) with `Standard14::Helvetica` in one document: the 38th has no free code in the font's `/Differences`, so the call fails with `encoding-full`.

## Data fields

- `font`: the standard font's PostScript name (fixed text: one of the fourteen).
- `limit`: how many codes `/Differences` can assign for this font.
- `free`: how many of them were still free; the text needed more, so none were assigned.
