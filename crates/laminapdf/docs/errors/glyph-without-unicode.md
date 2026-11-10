---
title: "glyph-without-unicode"
version: "0.2.0-alpha.5"
url: "https://docs.laminapdf.com/reference/codes/glyph-without-unicode/"
---

# glyph-without-unicode

- Type: error
- Kind: `usage` (the caller misused the API)

## What happened

Under PDF/A-2u or PDF/UA-1, a glyph was drawn with no Unicode value, or with one that holds U+0000, U+FEFF or U+FFFE.

A glyph that maps to the empty value doesn't fail. Shaping can draw one character with several glyphs: Noto Sans Arabic draws a beh as a dotless body and a dot. The body then carries the letter and the dot maps to the empty value, so the letter is read once.

## Why

Software can search, copy or read text aloud only when every glyph maps to a usable Unicode value. The PDF/A-2u archival standard (ISO 19005-2 §6.2.11.7.2) and the PDF/UA-1 standard for accessible PDF (ISO 14289-1 §7.21.7) both need that mapping. These are conditions 31-027 to 31-029 of the Matterhorn Protocol, the PDF Association's checklist of PDF/UA failures.

## How to fix

Remove the NUL characters (U+0000) from the text, and check what it was decoded from: a NUL usually means a UTF-16 string read as UTF-8, or the end of a C string. An archive that needs no accessibility can use PDF/A-2b instead, which doesn't need Unicode for every glyph. PDF/UA-1 has no such relaxed variant.

## Example

```rust
surface.text("\u{0}Report", &font, Pt::new(12.0), at)?;
builder.finish()?; // glyph-without-unicode under Profile::PdfA2u
```

## Data fields

- `profile`: the profile, `PDF/A-2u` or `PDF/UA-1`.
- `font`: the font's PostScript name (untrusted: read from the font file).
- `gid`: the glyph's ID in the font.
