# `encoding-full`

- Type: error
- Kind: `limit` (a `Limits` value or another resource bound was hit)

## What happened

A standard 14 font had no free one-byte code left for another glyph in this document.

## Why

A standard 14 font is a simple font: text is written one byte per character, so one font resource addresses at most 256 glyphs (ISO 32000-2 §9.6.5). lamina writes the Latin fonts in `WinAnsiEncoding`, and gives each glyph of the standard Latin character set outside it (`Lslash`, `Scedilla`, `minus`, …) one of the 37 codes `WinAnsiEncoding` leaves undefined, through `/Differences`. A document that draws more such glyphs with one font than there are free codes runs out.

## How to fix

Embed a font (`Font::from_bytes`), which addresses every glyph through a Type 0 font, or split the text across two registrations of the standard font (each is its own font resource with its own codes).

## Example

To be added ([L1b](../todos/ready/L1b-error-contract-codegen.md)).

## Data fields

- `font`: the standard font's PostScript name (fixed text: one of the fourteen).
- `limit`: how many codes `/Differences` can assign for this font.
