# `cmap-invalid`

- Type: warning
- Kind: `input` (the input is malformed beyond what tolerance repairs)

## What happened

A font's CMap (its Type 0 `/Encoding`) or ToUnicode CMap couldn't be fully used, for one of three reasons. It has entries that can't be read, which are skipped. Or it names a predefined CMap lamina has no table for, which is read as Identity: lamina knows only `Identity-H`/`-V` and the `Uni…-UCS2-…`/`Uni…-UTF16-…` ones. Or its `/UseCMap` chain is deeper than 8 levels, which is cut.

## Why

A producer bug (bad ranges, a missing `end…` keyword), or a CJK file using one of Adobe's character-collection CMaps without embedding it.

## How to fix

Nothing to do. The rest of the CMap is used; affected glyphs may extract as other text or U+FFFD.

## Example

```text
1 beginbfchar
<0003> <0020
endbfchar
```

The destination string is not closed; the entry is skipped, and the rest of the ToUnicode CMap is used.

## Data fields

None.

The `location` is `object`, the font dictionary's object; none for a direct font dictionary.

## For contributors

The decisions and todos behind this code:

- [B9b](../todos/done/B9b-text-extraction-core.md): text extraction
