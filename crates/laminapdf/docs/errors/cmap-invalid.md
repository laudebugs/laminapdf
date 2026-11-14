---
title: "cmap-invalid"
version: "0.3.0"
url: "https://docs.laminapdf.com/reference/codes/cmap-invalid/"
---

# cmap-invalid

- Type: warning
- Kind: `input` (the input is malformed beyond what tolerance repairs)

## What happened

A font's CMap (its Type 0 `/Encoding`) or ToUnicode CMap couldn't be fully used, for one of three reasons:

- entries that can't be read, which are skipped;
- a predefined CMap Lamina PDF has no table for, read as Identity (Lamina PDF knows only `Identity-H`/`-V` and the `Uni…-UCS2-…`/`Uni…-UTF16-…` ones);
- a `/UseCMap` chain deeper than 8 levels, cut there.

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
