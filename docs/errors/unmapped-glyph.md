# `unmapped-glyph`

- Type: warning
- Kind: `input` (the input is malformed beyond what tolerance repairs)

## What happened

A font showed glyphs that its ToUnicode CMap, its encoding's glyph names and its font program's `cmap` all leave unmapped. They extract as U+FFFD; `PageText::unmapped` counts them (todo B9b).

## Why

The file has no ToUnicode CMap for a subset or symbolic font, uses arbitrary glyph names (`g37`), or obfuscates its encoding (ISO 32000-2 §9.10.2: there is then no way to know what the glyph means).

## How to fix

Nothing to do in lamina. A producer that writes ToUnicode CMaps (lamina does) avoids it; OCR is the only way to recover such text.

## Example

To be added ([L1b](../todos/ongoing/L1b-error-contract-codegen.md)).

## Data fields

None.

The `location` is `object`, the font dictionary's object; none for a direct font dictionary.
