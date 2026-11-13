---
title: "font-invalid"
version: "0.2.0"
url: "https://docs.laminapdf.com/reference/codes/font-invalid/"
---

# font-invalid

- Type: warning
- Kind: `input` (the input is malformed beyond what tolerance repairs)

## What happened

A font dictionary read for text extraction has a missing, unreadable or malformed entry (the descendant font, `/Encoding` or its `/Differences`, `/Widths`, `/W`, `/W2`, the font descriptor or its font program). Its text was decoded with defaults: the standard encoding, the standard 14 font's widths or `/MissingWidth`, `/DW`.

## Why

A producer bug, a damaged file, or a font dictionary that relies on what a viewer guesses.

## How to fix

Nothing to do. Text or positions extracted for that font may be off; regenerating the file with a current producer fixes it.

## Example

```text
<< /Type /Font /Subtype /Type0 /BaseFont /Foo /Encoding /Identity-H /DescendantFonts 99 0 R >>
```

The descendant font is missing, so text in this font is decoded with defaults: two-byte codes, default widths. The `location` is the font dictionary's `object`.

## Data fields

None.

The `location` is `object`, the font dictionary's object; none for a direct font dictionary.
