# `font-invalid`

- Type: warning
- Kind: `input` (the input is malformed beyond what tolerance repairs)

## What happened

A font dictionary read for text extraction has a missing, unreadable or malformed entry (the descendant font, `/Encoding` or its `/Differences`, `/Widths`, `/W`, `/W2`, the font descriptor or its font program). Its text was decoded with defaults: the standard encoding, the standard 14 font's widths or `/MissingWidth`, `/DW` (todo B9b).

## Why

A producer bug, a damaged file, or a font dictionary that relies on what a viewer guesses.

## How to fix

Nothing to do. Text or positions extracted for that font may be off; regenerating the file with a current producer fixes it.

## Example

To be added ([L1b](../todos/ready/L1b-error-contract-codegen.md)).

## Data fields

None.

The `location` is `object`, the font dictionary's object; none for a direct font dictionary.
