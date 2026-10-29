# `tounicode-conflict`

- Type: warning
- Kind: `usage` (the caller misused the API)

## What happened

One glyph was used for two different texts; the ToUnicode CMap maps it to the first.

## Why

A ToUnicode CMap maps each code to one string (ISO 32000-2 §9.10.3), for example when one space glyph stands for both U+0020 and U+00A0.

## How to fix

Mark the second occurrence with `/ActualText` (§14.9.4) so extraction gets its text; the builder (B5b) does this.

## Example

To be added ([L1b](../todos/ready/L1b-error-contract-codegen.md)).

## Data fields

- `font`: the font's PostScript name (**untrusted**: read from the font).
- `gid`: the glyph, in the original font's numbering.
- `kept`, `dropped`: the two texts (**untrusted**: caller text).
