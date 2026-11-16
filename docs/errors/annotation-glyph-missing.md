# `annotation-glyph-missing`

- Type: error
- Kind: `usage` (the caller misused the API)

## What happened

A text box holds a character that none of its fonts can draw, so Lamina PDF refuses to draw the box.

## Why

Lamina PDF never draws a missing character as a blank box or skips it without saying so: the file would show different text from what you gave. Helvetica, Times and Courier hold Western European characters only (the Windows-1252 set). A font given by its bytes holds whatever its designer drew.

## How to fix

Give the box a fallback font that has the character, for example a Noto font for the script you need, or choose a font that has it. Fallback fonts need the WebAssembly text module or the native crates.

## Example

A box in Helvetica that holds `日本`, with no fallback font.

## Data fields

None.

## For contributors

The decisions and todos behind this code:

- [E4e](../todos/done/E4e-freetext-callouts.md): text boxes and callouts
