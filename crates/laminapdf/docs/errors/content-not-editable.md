---
title: "content-not-editable"
version: "0.8.0"
url: "https://docs.laminapdf.com/reference/codes/content-not-editable/"
---

# content-not-editable

- Type: error
- Kind: `unsupported` (valid input that Lamina PDF does not support yet)

## What happened

An edit tried to change content that Lamina PDF can't change safely. The edit changed nothing, and the content stays on the page, selectable and readable.

Most reasons are about text. Two are about moving any object: whether it clips what comes after it, and whether it draws the same in its new place in the stacking order.

## Why

PDF stores text as glyphs placed one by one in a font's own encoding, not as characters. To change text, Lamina PDF has to find the glyphs that belong together, learn which characters they stand for, and place new glyphs in the same font. It refuses, and names the reason, when it can't do that reliably:

- `type3-font`: the font draws each glyph with its own small drawing, which has no characters to map new text to.
- `vertical-text`: the text is written top to bottom, and editing handles only horizontal text so far.
- `no-unicode-map`: the font says nothing about which characters its glyphs stand for.
- `rotated-run`: glyphs of one line are turned against each other.
- `glyphs-out-of-order`: the file draws the line's glyphs out of reading order.
- `actual-text`: the glyphs stand in for other text the file gives (its `ActualText`), which copy and search read; changing the glyphs would leave that text behind.
- `right-to-left`: the text is written right to left. Files draw such glyphs left to right or right to left, so Lamina PDF can't tell in which order to write new text.
- `too-deep`: the content sits inside more nested drawings than the limits allow.
- `content-unreadable`: the page's content could not be read.
- `clipping-path`: the object clips everything drawn after it. Moving, deleting or reordering it would change other content too, so its clip has to be edited on its own.
- `state-not-portable`: moving the object in the stacking order would change how it looks, because the clip, the transparency or the font differ where it would go.

## How to fix

The file is fine: Lamina PDF reads this content and shows it as it is. To change it, delete the object and add new content in its place (except for `clipping-path`, whose object can't be deleted either), or edit the file in the program that made it. For `state-not-portable`, try another place in the stacking order: next to objects drawn under the same clip.

## Example

```text
BT /T3 12 Tf 72 700 Td (Total) Tj ET
```

When `/T3` is a Type 3 font, an edit of this text fails with `content-not-editable` and the reason `type3-font`.

## Data fields

- `reason`: why the content can't be edited: `type3-font`, `vertical-text`, `no-unicode-map`, `rotated-run`, `glyphs-out-of-order`, `actual-text`, `right-to-left`, `too-deep`, `content-unreadable`, `clipping-path` or `state-not-portable`.
