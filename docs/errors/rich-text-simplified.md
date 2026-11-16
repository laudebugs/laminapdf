# `rich-text-simplified`

- Type: warning
- Kind: `usage` (the caller misused the API)

## What happened

An edit changed a text box made by another app: its text, colour, font or box, and that box's rich text held styles Lamina PDF doesn't keep, such as a size or colour for a single word. The box now uses one font, size and colour for all of its text, with bold and italic kept.

## Why

A text box keeps its styled text as XHTML in its `/RC` key. Lamina PDF keeps a small part of it: paragraphs, and bold and italic words, with one font, size, colour and alignment for the box. Supporting all of XHTML and CSS would need a browser's layout engine. Lamina PDF reads the wider styles so a box keeps its look until you edit it, then draws it again in the subset.

## How to fix

Check how the edited box looks. To give a word its own size or colour, use a second text box. The warning never blocks the edit or the save.

## Example

Changing the text or colour of an Acrobat box whose `/RC` colours one word red.

## Data fields

- `annotations`: the IDs of the text boxes the edit simplified.

## For contributors

The decisions and todos behind this code:

- [E4e](../todos/done/E4e-freetext-callouts.md): text boxes and callouts
- [E4 plan §11](../plans/editor/e4-markup-and-drawing-annotations.md#11-decisions-on-the-open-questions): the rich-text subset
