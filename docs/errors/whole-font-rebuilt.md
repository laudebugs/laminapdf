# `whole-font-rebuilt`

- Type: warning
- Kind: `usage` (the caller misused the API)

## What happened

A font whose `OS/2.fsType` forbids subsetting was embedded with every glyph, but rebuilt rather than byte for byte.

## Why

It is a variable-font instance or has CFF2 outlines; PDF can embed neither as it is, so `subsetter` instanced it (or converted CFF2 to TrueType outlines) while keeping every glyph. Its `cmap` and layout tables are dropped.

## How to fix

Check the font's licence allows this, or use a static TrueType/CFF version of the font.

## Example

To be added ([L1b](../todos/ongoing/L1b-error-contract-codegen.md)).

## Data fields

- `font`: the font's PostScript name (**untrusted**: read from the font).
