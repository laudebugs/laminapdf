# `text-overflow`

- Type: error
- Kind: `usage` (the caller misused the API)

## What happened

`Surface::text_box` laid its text out and not every line fit the box (its height, or a single grapheme cluster wider than the box), and its `TextBoxOptions` asked for `Overflow::Error`. Nothing was drawn.

## Why

A text box draws only lines that fit inside it. With `Overflow::Error` the caller asked to be told instead of having the text cut off.

## How to fix

Make the box taller or wider, the font smaller or the text shorter; or use `Overflow::Clip` (the default), which draws the lines that fit and reports where the rest starts in `TextLayout::overflow`.

## Example

To be added ([L1b](../todos/ongoing/L1b-error-contract-codegen.md)).

## Data fields

- `fits`: the number of lines that fit.
- `needed`: the number of lines the text needs.
