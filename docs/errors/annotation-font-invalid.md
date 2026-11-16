# `annotation-font-invalid`

- Type: error
- Kind: `usage` (the caller misused the API)

## What happened

A text box names a font by the ID of bytes added to the session, and those bytes aren't a font Lamina PDF can read and embed.

## Why

A text box's font is embedded in the file as a subset, the glyphs the box uses, so every reader shows the same letters. The bytes must be a TrueType or OpenType font whose licence allows embedding. A font marked restricted, or one that holds only bitmaps, can't be embedded.

## How to fix

Add the bytes of a TrueType or OpenType font that allows embedding, and name that blob's ID as the box's font.

## Example

A box whose `font` is the ID of a JPEG added with `addBlob`.

## Data fields

None.

## For contributors

The decisions and todos behind this code:

- [E4e](../todos/done/E4e-freetext-callouts.md): text boxes and callouts
