# `glyph-out-of-range`

- Type: error
- Kind: `usage` (the caller misused the API)

## What happened

`FontRegistry::use_glyph` was given a glyph ID at or past the font's glyph count.

## Why

Glyph IDs must come from the same font (its `cmap` or a shaper run on it).

## How to fix

Pass glyph IDs from the font you registered.

## Example

To be added ([L1b](../todos/ongoing/L1b-error-contract-codegen.md)).

## Data fields

- `gid`: the glyph ID.
- `count`: the font's glyph count.
- `font`: the font's PostScript name (**untrusted**: read from the font).
