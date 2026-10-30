# `subset-failed`

- Type: error
- Kind: `unsupported` (valid input that lamina doesn't implement)

## What happened

Subsetting a font at `FontRegistry::finish_fonts` failed.

## Why

The `subsetter` crate could not rebuild the font program: a font feature it does not support, or a table it could not parse.

## How to fix

Try another version or format of the font (for example TrueType instead of CFF), and report the font if it is a common one.

## Example

To be added ([L1b](../todos/ongoing/L1b-error-contract-codegen.md)).

## Data fields

- `font`: the font's PostScript name (**untrusted**: read from the font).
- `reason`: the subsetter's failure class (fixed text).
