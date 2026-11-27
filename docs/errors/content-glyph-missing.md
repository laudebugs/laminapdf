# `content-glyph-missing`

- Type: error
- Kind: `usage` (the caller misused the API)

## What happened

An op adds text to a page, and a character of it is in none of the fonts the op names, so no font can draw it.

## Why

Lamina PDF draws every character in a font that has a glyph for it, and tries the op's fonts in order: the first, then each fallback. A character none of them covers would have to be drawn as an empty box or not at all, which would change what the page says, so the op is refused instead of drawing it silently wrong.

## How to fix

Add a fallback font that has the character to the op's `fonts`: a symbol font for arrows, stars and mathematical signs, or the font of another script for its letters. Or choose a font that covers the text. The reader's font picker offers a symbol font for the symbols it inserts.

## Example

```json
{ "op": "addText", "args": { "id": "tick", "page": "p1.0", "rect": [72, 72, 288, 96], "runs": [{ "text": "Done \u2713" }], "fonts": ["helvetica"], "modified": "D:20261118120000Z" } }
```

Helvetica has no check mark (U+2713). Add the bytes of a font that has it, such as Noto Sans Symbols 2, after Helvetica in `fonts`.

## Data fields

None.

## For contributors

The decisions and todos behind this code:

- [E6c](../todos/done/E6c-add-text.md): text and special characters added as page content
