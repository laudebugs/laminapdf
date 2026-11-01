# `unmapped-glyph`

- Type: warning
- Kind: `input` (the input is malformed beyond what tolerance repairs)

## What happened

A font showed glyphs that its ToUnicode CMap, its encoding's glyph names and its font program's `cmap` all leave unmapped. They extract as U+FFFD; `PageText::unmapped` counts them.

## Why

The file has no ToUnicode CMap for a subset or symbolic font, uses arbitrary glyph names (`g37`), or obfuscates its encoding (ISO 32000-2 §9.10.2: there is then no way to know what the glyph means).

## How to fix

Nothing to do in Lamina PDF. A producer that writes ToUnicode CMaps (Lamina PDF does) avoids it; OCR is the only way to recover such text.

## Example

```text
<< /Subtype /Type0 /BaseFont /ABCDEF+Subset /Encoding /Identity-H >>   % no /ToUnicode
```

A subset font with Identity-H and no ToUnicode: unless its embedded `cmap` maps the glyphs, they extract as U+FFFD and the warning carries the font's `object`.

## Data fields

None.

The `location` is `object`, the font dictionary's object; none for a direct font dictionary.

## For contributors

The decisions and todos behind this code:

- [B9b](../todos/done/B9b-text-extraction-core.md): text extraction
