# `glyph-without-unicode`

- Type: error
- Kind: `usage` (the caller misused the API)

## What happened

Under PDF/A-2u or PDF/UA-1, a glyph was drawn that stands for no text of its own, or whose text is U+0000, U+FEFF or U+FFFE. Shaping can draw one cluster (a group of characters that display as one unit) with several glyphs. The text then belongs to the first glyph, and the others stand for none.

## Why

Software can search, copy or read text aloud only when every glyph maps to a usable Unicode value. The PDF/A-2u archival standard (ISO 19005-2 §6.2.11.7.2) and the PDF/UA-1 standard for accessible PDF (ISO 14289-1 §7.21.7) both need that mapping. These are conditions 31-027 to 31-029 of the Matterhorn Protocol, the PDF Association's checklist of PDF/UA failures. Lamina PDF maps a glyph that stands for no text to an empty value, which these profiles reject.

## How to fix

Check the text for control characters and byte-order marks. If shaping draws several glyphs for one cluster, choose a font that draws each cluster as one glyph. An archive that needs no accessibility can use PDF/A-2b instead, which doesn't need Unicode for every glyph. PDF/UA-1 has no such relaxed variant, so a PDF/UA-1 document needs the font change.

## Example

```rust
surface.text("\u{FEFF}Report", &font, Pt::new(12.0), at)?;
builder.finish()?; // glyph-without-unicode under Profile::PdfA2u
```

## Data fields

- `profile`: the profile, `PDF/A-2u` or `PDF/UA-1`.
- `font`: the font's PostScript name (untrusted: read from the font file).
- `gid`: the glyph's ID in the font.
