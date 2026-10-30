# `cmap-invalid`

- Type: warning
- Kind: `input` (the input is malformed beyond what tolerance repairs)

## What happened

A font's CMap (its Type 0 `/Encoding`) or ToUnicode CMap had entries that could not be read, which were skipped; or it names a predefined CMap lamina has no table for (only `Identity-H`/`-V` and the `Uni…-UCS2-…`/`Uni…-UTF16-…` ones are known), which is read as Identity; or its `/UseCMap` chain is deeper than 8 levels, which is cut (todo B9b).

## Why

A producer bug (bad ranges, a missing `end…` keyword), or a CJK file using one of Adobe's character-collection CMaps without embedding it.

## How to fix

Nothing to do. The rest of the CMap is used; affected glyphs may extract as other text or U+FFFD.

## Example

To be added ([L1b](../todos/ready/L1b-error-contract-codegen.md)).

## Data fields

None.

The `location` is `object`, the font dictionary's object; none for a direct font dictionary.
