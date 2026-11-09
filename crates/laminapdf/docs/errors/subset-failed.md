---
title: "subset-failed"
version: "0.2.0-alpha.4"
url: "https://docs.laminapdf.com/reference/codes/subset-failed/"
---

# subset-failed

- Type: error
- Kind: `unsupported` (valid input that Lamina PDF doesn't implement)

## What happened

Subsetting a font at `FontRegistry::finish_fonts` failed.

## Why

The `subsetter` crate could not rebuild the font program: a font feature it does not support, or a table it could not parse.

## How to fix

Try another version or format of the font (for example TrueType instead of CFF), and report the font if it is a common one.

## Example

A font whose tables `subsetter` can't rebuild makes `FontRegistry::finish_fonts` (and so `Builder::finish`) fail with `subset-failed`; `data.font` is the font's PostScript name.

## Data fields

- `font`: the font's PostScript name (**untrusted**: read from the font).
- `reason`: the subsetter's failure class (fixed text).
