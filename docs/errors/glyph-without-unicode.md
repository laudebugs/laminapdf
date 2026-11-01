# `glyph-without-unicode`

- Type: error
- Kind: `usage` (the caller misused the API)

## What happened

Under PDF/A-2u or PDF/UA-1, a glyph was drawn that stands for no text of its own (one of several glyphs a shaped cluster was drawn with), or whose text is U+0000, U+FEFF or U+FFFE.

## Why

PDF/A-2u (ISO 19005-2 §6.2.11.7.2) and PDF/UA-1 (ISO 14289-1 §7.21.7, Matterhorn Protocol 31-027 to 31-029) require every glyph to map to a usable Unicode value, so the text can be searched, copied and read aloud. lamina maps a glyph that stands for no text to an empty value, which these profiles don't accept.

## How to fix

Check the text for control characters and byte-order marks. For scripts whose shaping draws several glyphs per character cluster, choose PDF/A-2b, which doesn't need Unicode for every glyph.

## Example

```rust
surface.text("\u{FEFF}Report", &font, Pt::new(12.0), at)?;
builder.finish()?; // glyph-without-unicode under Profile::PdfA2u
```

## Data fields

- `profile`: the profile, `PDF/A-2u` or `PDF/UA-1`.
- `font`: the font's PostScript name (untrusted: read from the font file).
- `gid`: the glyph's ID in the font.
