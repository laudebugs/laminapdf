# `font-not-embedded`

- Type: error
- Kind: `usage` (the caller misused the API)

## What happened

Text was drawn with one of the standard 14 fonts (`Builder::standard_font`) under PDF/UA-1 or a PDF/A profile. The standard 14 fonts are never embedded: viewers substitute their own.

## Why

The PDF/A standard for archival PDF (ISO 19005-2 §6.2.11.4.1) needs every font embedded, so the document looks the same everywhere in the future. The PDF/UA-1 standard for accessible PDF (ISO 14289-1 §7.21.4.1) needs it too, so viewers can render the text and map it to characters reliably. This is condition 31-009 of the Matterhorn Protocol, the PDF Association's checklist of PDF/UA failures.

## How to fix

Load a font file with `Builder::font` (or `Builder::font_bytes` in the façade): Lamina PDF embeds the subset of glyphs you draw.

## Example

```rust
let bold = builder.standard_font(Standard14::HelveticaBold)?;
surface.text("Invoice", &bold, Pt::new(24.0), at)?;
builder.finish()?; // font-not-embedded under Profile::PdfA2b
```

## Data fields

- `profile`: the profile, such as `PDF/A-2b`.
- `font`: the font's PostScript name, such as `Helvetica-Bold`.
