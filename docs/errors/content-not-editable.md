# `content-not-editable`

- Type: error
- Kind: `unsupported` (valid input that Lamina PDF does not support yet)

## What happened

An edit tried to change content that Lamina PDF can't change safely. The edit changed nothing, and the content stays on the page, selectable and readable.

## Why

PDF stores text as glyphs placed one by one in a font's own encoding, not as characters. To change text, Lamina PDF has to find the glyphs that belong together, learn which characters they stand for, and place new glyphs in the same font. It refuses when it can't do that reliably:

- `type3-font`: the font draws each glyph with its own small drawing, which has no characters to map new text to.
- `vertical-text`: the text is written top to bottom, which editing doesn't support yet.
- `no-unicode-map`: the font says nothing about which characters its glyphs stand for.
- `rotated-run`: glyphs of one line are turned against each other.
- `glyphs-out-of-order`: the file draws the line's glyphs out of reading order.
- `too-deep`: the content sits inside more nested drawings than the limits allow.
- `content-unreadable`: the page's content could not be read.

## How to fix

Nothing in the file needs fixing: Lamina PDF reads this content and shows it as it is. To change it, delete the object and add new content in its place, or edit the file in the program that made it.

## Example

```text
BT /T3 12 Tf 72 700 Td (Total) Tj ET
```

When `/T3` is a Type 3 font, an edit of this text fails with `content-not-editable` and the reason `type3-font`.

## Data fields

- `reason`: why the content can't be edited: `type3-font`, `vertical-text`, `no-unicode-map`, `rotated-run`, `glyphs-out-of-order`, `too-deep` or `content-unreadable`.

## For contributors

The decisions and todos behind this code:

- [ADR 0022 §5–§7](../decisions/0022-content-stream-editing.md#5-finding-text-runs-marked-content-first-layout-second): which text runs are read-only, and why
- [E7c](../todos/done/E7c-content-ir.md): the content IR and this code
